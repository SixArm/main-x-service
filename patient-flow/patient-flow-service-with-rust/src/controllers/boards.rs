//! Derived read views (PF-D6): ward whiteboard, hospital at a glance,
//! patient locate, and the capacity metrics snapshot. All are queries
//! over the operational tables with an `as_of` stamp — never a second
//! store.

use authentication_verifier::Action;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Serialize;
use uuid::Uuid;

use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::flow::bed_state::BedState;
use crate::flow::journey;
use crate::metrics::Metrics;
use crate::models::_entities::{bays, bed_requests, beds, infection_flags, red_green_days, sites, stays, wards};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;

/// One bed card on the ward whiteboard (spec `whiteboard.md`).
#[allow(clippy::struct_excessive_bools)] // the card's chips are genuinely independent flags
#[derive(Debug, Serialize)]
struct BedCard {
    bed_pid: String,
    bay_name: String,
    number: String,
    state: String,
    state_since: chrono::DateTime<chrono::FixedOffset>,
    closure_reason: Option<String>,
    deep_clean_required: bool,
    side_room: bool,
    // Occupied-bed fields (None on empty beds).
    stay_pid: Option<String>,
    display_name: Option<String>,
    named_nurse_ref: Option<String>,
    consultant_ref: Option<String>,
    edd: Option<chrono::NaiveDate>,
    edd_missing: bool,
    edd_overdue: bool,
    ccd_met: bool,
    discharge_pathway: Option<String>,
    discharge_ready: bool,
    dtoc: bool,
    senior_review_today: bool,
    red_green_today: Option<String>,
    infection: Vec<serde_json::Value>,
    alerts: Vec<String>,
}

/// The whiteboard response: bay-ordered bed cards + freshness.
#[derive(Debug, Serialize)]
struct Whiteboard {
    ward_pid: String,
    ward_name: String,
    ward_code: String,
    kind: String,
    closed_to_admissions: bool,
    escalation: bool,
    as_of: chrono::DateTime<chrono::FixedOffset>,
    masked: bool,
    cards: Vec<BedCard>,
}

/// `GET /api/whiteboard/{ward_pid}` — the digital ward whiteboard.
/// Record-level ABAC applies (ward-scoped policies); the `mask`
/// obligation redacts patient names + alerts while keeping bed states
/// visible (corridor mode).
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass assembling the full bed-card set
async fn whiteboard(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(ward_pid): Path<String>,
) -> Result<Response> {
    let ward = records::find_ward(&ctx.db, records::parse_pid(&ward_pid)?).await?;
    let obligations = auth::authorize_record(&caller, Action::Read, &auth::ward_resource_attrs(&ward))
        .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let today = now.date_naive();
    let bay_rows = bays::Entity::find()
        .filter(bays::Column::WardPid.eq(ward.pid))
        .filter(bays::Column::DeletedAt.is_null())
        .order_by_asc(bays::Column::Id)
        .all(&ctx.db)
        .await?;
    let bay_pids: Vec<Uuid> = bay_rows.iter().map(|b| b.pid).collect();
    let bed_rows = beds::Entity::find()
        .filter(beds::Column::BayPid.is_in(bay_pids))
        .filter(beds::Column::DeletedAt.is_null())
        .order_by_asc(beds::Column::Id)
        .all(&ctx.db)
        .await?;
    let stay_rows = stays::Entity::find()
        .filter(stays::Column::WardPid.eq(ward.pid))
        .filter(stays::Column::DischargedAt.is_null())
        .filter(stays::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let stay_pids: Vec<Uuid> = stay_rows.iter().map(|s| s.pid).collect();
    let flags = infection_flags::Entity::find()
        .filter(infection_flags::Column::StayPid.is_in(stay_pids.clone()))
        .filter(infection_flags::Column::ClearedAt.is_null())
        .all(&ctx.db)
        .await?;
    let rg_today = red_green_days::Entity::find()
        .filter(red_green_days::Column::StayPid.is_in(stay_pids))
        .filter(red_green_days::Column::Day.eq(today))
        .all(&ctx.db)
        .await?;
    let mut cards = Vec::with_capacity(bed_rows.len());
    for bed in &bed_rows {
        let bay = bay_rows.iter().find(|b| b.pid == bed.bay_pid);
        let stay = stay_rows.iter().find(|s| s.bed_pid == Some(bed.pid));
        let card = BedCard {
            bed_pid: bed.pid.to_string(),
            bay_name: bay.map(|b| b.name.clone()).unwrap_or_default(),
            number: bed.number.clone(),
            state: bed.state.clone(),
            state_since: bed.state_since,
            closure_reason: bed.closure_reason.clone(),
            deep_clean_required: bed.deep_clean_required,
            side_room: bay.is_some_and(|b| b.side_room),
            stay_pid: stay.map(|s| s.pid.to_string()),
            display_name: stay.map(|s| {
                if masked {
                    auth::MASKED.to_string()
                } else {
                    s.display_name.clone()
                }
            }),
            named_nurse_ref: stay.and_then(|s| s.named_nurse_ref.clone()),
            consultant_ref: stay.and_then(|s| s.consultant_ref.clone()),
            edd: stay.and_then(|s| s.edd),
            edd_missing: stay.is_some_and(|s| s.edd.is_none()),
            edd_overdue: stay.is_some_and(|s| journey::edd_overdue(s.edd, today)),
            ccd_met: stay.is_some_and(|s| s.ccd_met),
            discharge_pathway: stay.and_then(|s| s.discharge_pathway.clone()),
            discharge_ready: stay.is_some_and(|s| s.status == "discharge_ready"),
            dtoc: stay.is_some_and(|s| journey::is_dtoc(s.discharge_ready_at, s.discharged_at, now)),
            senior_review_today: stay
                .and_then(|s| s.senior_review_at)
                .is_some_and(|t| t.date_naive() == today),
            red_green_today: stay.and_then(|s| {
                rg_today
                    .iter()
                    .find(|r| r.stay_pid == s.pid)
                    .map(|r| r.classification.clone())
            }),
            infection: stay
                .map(|s| {
                    flags
                        .iter()
                        .filter(|f| f.stay_pid == s.pid)
                        .map(|f| {
                            serde_json::json!({
                                "precaution": f.precaution,
                                "organism": f.organism,
                                "status": f.status,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            alerts: if masked {
                Vec::new()
            } else {
                stay.map(|s| {
                    serde_json::from_value::<Vec<String>>(s.alerts.clone()).unwrap_or_default()
                })
                .unwrap_or_default()
            },
        };
        cards.push(card);
    }
    format::json(Whiteboard {
        ward_pid: ward.pid.to_string(),
        ward_name: ward.name,
        ward_code: ward.code,
        kind: ward.kind,
        closed_to_admissions: ward.closed_to_admissions,
        escalation: ward.escalation,
        as_of: now,
        masked,
        cards,
    })
}

/// One ward row in the at-a-glance view (spec `capacity.md`).
#[derive(Debug, Default, Serialize)]
struct WardGlance {
    ward_pid: String,
    site_pid: String,
    name: String,
    code: String,
    kind: String,
    escalation: bool,
    closed_to_admissions: bool,
    beds_total: usize,
    occupied: usize,
    available: usize,
    reserved: usize,
    awaiting_clean: usize,
    cleaning: usize,
    closed: usize,
    closed_for_infection: usize,
    occupancy_pct: f64,
    expected_discharges_today: usize,
    edd_overdue: usize,
    discharge_ready: usize,
    dtoc: usize,
    open_requests_targeting: usize,
    long_stay_7: usize,
    long_stay_21: usize,
}

/// The at-a-glance response: per-ward rows + site tiles + freshness.
#[derive(Debug, Serialize)]
struct AtAGlance {
    as_of: chrono::DateTime<chrono::FixedOffset>,
    wards: Vec<WardGlance>,
    site_tiles: serde_json::Value,
}

/// Compute the at-a-glance snapshot (shared by the endpoint and the
/// capacity metrics; also refreshes the Prometheus gauges).
#[allow(clippy::too_many_lines)] // one pass over the whole estate
async fn glance(ctx: &AppContext) -> Result<AtAGlance> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let today = now.date_naive();
    let ward_rows = wards::Entity::find()
        .filter(wards::Column::DeletedAt.is_null())
        .order_by_asc(wards::Column::Id)
        .all(&ctx.db)
        .await?;
    let bay_rows = bays::Entity::find()
        .filter(bays::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let bed_rows = beds::Entity::find()
        .filter(beds::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let stay_rows = stays::Entity::find()
        .filter(stays::Column::DischargedAt.is_null())
        .filter(stays::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let open_requests = bed_requests::Entity::find()
        .filter(bed_requests::Column::DeletedAt.is_null())
        .filter(bed_requests::Column::Status.eq("open"))
        .all(&ctx.db)
        .await?;
    let mut rows = Vec::with_capacity(ward_rows.len());
    for ward in &ward_rows {
        let ward_bays: Vec<Uuid> = bay_rows
            .iter()
            .filter(|b| b.ward_pid == ward.pid)
            .map(|b| b.pid)
            .collect();
        let ward_beds: Vec<_> = bed_rows.iter().filter(|b| ward_bays.contains(&b.bay_pid)).collect();
        let ward_stays: Vec<_> = stay_rows.iter().filter(|s| s.ward_pid == Some(ward.pid)).collect();
        let count_state = |token: &str| ward_beds.iter().filter(|b| b.state == token).count();
        let closed = count_state(BedState::Closed.token());
        let total = ward_beds.len();
        let occupied = count_state(BedState::Occupied.token());
        let usable = total.saturating_sub(closed);
        let mut row = WardGlance {
            ward_pid: ward.pid.to_string(),
            site_pid: ward.site_pid.to_string(),
            name: ward.name.clone(),
            code: ward.code.clone(),
            kind: ward.kind.clone(),
            escalation: ward.escalation,
            closed_to_admissions: ward.closed_to_admissions,
            beds_total: total,
            occupied,
            available: count_state(BedState::Available.token()),
            reserved: count_state(BedState::Reserved.token()),
            awaiting_clean: count_state(BedState::AwaitingClean.token()),
            cleaning: count_state(BedState::Cleaning.token()),
            closed,
            closed_for_infection: ward_beds
                .iter()
                .filter(|b| b.state == "closed" && b.closure_reason.as_deref() == Some("infection"))
                .count(),
            #[allow(clippy::cast_precision_loss)]
            occupancy_pct: if usable == 0 {
                0.0
            } else {
                (occupied as f64 / usable as f64 * 1000.0).round() / 10.0
            },
            expected_discharges_today: ward_stays.iter().filter(|s| s.edd == Some(today)).count(),
            edd_overdue: ward_stays
                .iter()
                .filter(|s| journey::edd_overdue(s.edd, today))
                .count(),
            discharge_ready: ward_stays.iter().filter(|s| s.status == "discharge_ready").count(),
            dtoc: ward_stays
                .iter()
                .filter(|s| journey::is_dtoc(s.discharge_ready_at, s.discharged_at, now))
                .count(),
            open_requests_targeting: open_requests
                .iter()
                .filter(|r| r.target_ward_pid == Some(ward.pid))
                .count(),
            long_stay_7: ward_stays
                .iter()
                .filter(|s| journey::length_of_stay_days(s.admitted_at, None, now) > 6)
                .count(),
            long_stay_21: ward_stays
                .iter()
                .filter(|s| journey::length_of_stay_days(s.admitted_at, None, now) > 20)
                .count(),
        };
        // Virtual wards report census but no cleaning pipeline.
        if ward.kind == "virtual" {
            row.awaiting_clean = 0;
            row.cleaning = 0;
        }
        rows.push(row);
    }
    let physical: Vec<_> = rows.iter().filter(|r| r.kind != "virtual").collect();
    let virtual_census: usize = rows.iter().filter(|r| r.kind == "virtual").map(|r| r.occupied).sum();
    let available_now: usize = physical.iter().map(|r| r.available).sum();
    let predicted_discharges: usize = physical.iter().map(|r| r.expected_discharges_today).sum();
    let reserved: usize = physical.iter().map(|r| r.reserved).sum();
    let dtoc_total: usize = rows.iter().map(|r| r.dtoc).sum();
    let by_priority = |p: &str| open_requests.iter().filter(|r| r.priority == p).count();
    let site_tiles = serde_json::json!({
        "available_now": available_now,
        "predicted_available_by_midnight":
            (available_now + predicted_discharges).saturating_sub(reserved),
        "open_requests": {
            "emergency": by_priority("emergency"),
            "urgent": by_priority("urgent"),
            "routine": by_priority("routine"),
        },
        "dtoc": dtoc_total,
        "virtual_ward_census": virtual_census,
        "escalation_beds_open": rows.iter().filter(|r| r.escalation).map(|r| r.beds_total).sum::<usize>(),
    });
    // Refresh the Prometheus gauges from this snapshot.
    let m = Metrics::global();
    m.beds_occupied
        .set(i64::try_from(physical.iter().map(|r| r.occupied).sum::<usize>()).unwrap_or(0));
    m.beds_available.set(i64::try_from(available_now).unwrap_or(0));
    m.dtoc_current.set(i64::try_from(dtoc_total).unwrap_or(0));
    m.bed_requests_open
        .set(i64::try_from(open_requests.len()).unwrap_or(0));
    Ok(AtAGlance { as_of: now, wards: rows, site_tiles })
}

/// `GET /api/at-a-glance` — per-ward rows + site tiles.
#[debug_handler]
async fn at_a_glance(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(glance(&ctx).await?)
}

/// `GET /api/capacity/metrics` — the same snapshot, flat, for
/// dashboards (also refreshes the Prometheus gauges).
#[debug_handler]
async fn capacity_metrics(State(ctx): State<AppContext>) -> Result<Response> {
    let snapshot = glance(&ctx).await?;
    format::json(serde_json::json!({
        "as_of": snapshot.as_of,
        "site": snapshot.site_tiles,
        "wards": snapshot.wards,
    }))
}

/// `GET /api/locate/{person_ref}` — *where is patient X right now?*
/// Sensitive read: ABAC-gated (ward-scoped) and audited.
#[debug_handler]
async fn locate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(person_ref): Path<String>,
) -> Result<Response> {
    // Validate the URN shape before touching the database.
    let parsed: entity_ref::EntityRef = person_ref
        .parse()
        .map_err(|_| super::unprocessable("person_ref must be a person:<uuid> URN"))?;
    if parsed.entity_type != entity_ref::EntityType::Person {
        return Err(super::unprocessable("person_ref must reference a person"));
    }
    let stay = stays::Entity::find()
        .filter(stays::Column::PersonRef.eq(person_ref.clone()))
        .filter(stays::Column::DeletedAt.is_null())
        .order_by_desc(stays::Column::Id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let obligations = auth::authorize_record(&caller, Action::Read, &auth::stay_resource_attrs(&stay))
        .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");
    let (ward, bay, bed) = match (stay.ward_pid, stay.bed_pid) {
        (Some(ward_pid), Some(bed_pid)) => {
            let ward = records::find_ward(&ctx.db, ward_pid).await.ok();
            let bed = records::find_bed(&ctx.db, bed_pid).await.ok();
            let bay = match &bed {
                Some(b) => records::find_bay(&ctx.db, b.bay_pid).await.ok(),
                None => None,
            };
            (ward, bay, bed)
        }
        (Some(ward_pid), None) => (records::find_ward(&ctx.db, ward_pid).await.ok(), None, None),
        _ => (None, None, None),
    };
    let site = match &ward {
        Some(w) => sites::Entity::find()
            .filter(sites::Column::Pid.eq(w.site_pid))
            .one(&ctx.db)
            .await?,
        None => None,
    };
    // Locate is personal data: every read is audited (spec `audit.md`).
    Audit::record(
        &ctx.db,
        "stay",
        stay.pid,
        "locate_read",
        caller.actor(),
        stay.ward_pid.map(|w| serde_json::json!({ "ward_pid": w.to_string() })),
    )
    .await?;
    format::json(serde_json::json!({
        "person_ref": stay.person_ref,
        "display_name": if masked { auth::MASKED.to_string() } else { stay.display_name },
        "status": stay.status,
        "stay_pid": stay.pid.to_string(),
        "site": site.map(|s| s.name),
        "ward": ward.map(|w| serde_json::json!({ "pid": w.pid.to_string(), "name": w.name, "code": w.code, "kind": w.kind })),
        "bay": bay.map(|b| b.name),
        "bed": bed.map(|b| b.number),
        "home_location_note": if masked { None } else { stay.home_location_note },
        "discharged_at": stay.discharged_at,
    }))
}

/// The board routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/whiteboard/{ward_pid}", get(whiteboard))
        .add("/at-a-glance", get(at_a_glance))
        .add("/capacity/metrics", get(capacity_metrics))
        .add("/locate/{person_ref}", get(locate))
}
