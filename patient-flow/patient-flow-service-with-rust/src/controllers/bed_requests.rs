//! The bed-request demand queue + rule-checked allocation (spec
//! `bed-management.md`; PF-D7: the allocator advises, the operator
//! decides).

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::flow::allocation::{self, BedFacts, Overrides, Requirements};
use crate::flow::bed_state::{BedState, Transition};
use crate::flow::tokens;
use crate::metrics::Metrics;
use crate::models::_entities::{bays, bed_requests, beds, wards};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/bed-requests` body.
#[derive(Debug, Deserialize)]
struct RequestPayload {
    person_ref: String,
    origin: String,
    #[serde(default)]
    target_ward_pid: Option<Uuid>,
    #[serde(default)]
    specialty: Option<String>,
    priority: String,
    #[serde(default)]
    requirements: Requirements,
}

/// `POST /api/bed-requests/{pid}/allocate` body.
#[derive(Debug, Deserialize)]
struct AllocatePayload {
    bed_pid: Uuid,
    #[serde(default)]
    override_sex: bool,
    #[serde(default)]
    override_ward_fit: bool,
    #[serde(default)]
    override_reason: Option<String>,
}

/// One ranked eligible bed in the advisory response.
#[derive(Debug, Serialize)]
struct EligibleBed {
    bed_pid: String,
    number: String,
    ward_pid: String,
    ward_code: String,
    bay_name: String,
    side_room: bool,
    right_ward: bool,
}

/// `GET /api/bed-requests` list item: the row plus its live
/// eligible-bed count (the escalation signal when zero).
#[derive(Debug, Serialize)]
struct RequestView {
    #[serde(flatten)]
    request: bed_requests::Model,
    eligible_beds: usize,
}

fn validate_request(p: &RequestPayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_ref("person_ref", entity_ref::EntityType::Person, &p.person_ref);
    problems.require_token("origin", tokens::REQUEST_ORIGINS, &p.origin);
    problems.require_token("priority", tokens::REQUEST_PRIORITIES, &p.priority);
    problems.cap_opt("specialty", p.specialty.as_deref());
    if let Some(sex) = p.requirements.sex.as_deref() {
        problems.require_token("requirements.sex", tokens::SEXES, sex);
    }
    problems.into_vec()
}

/// Assemble [`BedFacts`] for every active bed, joined to its bay and
/// ward, against one request. Returns `(bed, bay, ward, facts)`.
async fn all_bed_facts(
    db: &DatabaseConnection,
    request: &bed_requests::Model,
) -> Result<Vec<(beds::Model, bays::Model, wards::Model, BedFacts)>> {
    let requirements: Requirements =
        serde_json::from_value(request.requirements.clone()).unwrap_or_default();
    let _ = &requirements; // facts are requirement-independent; kept for clarity
    let ward_rows = wards::Entity::find()
        .filter(wards::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let bay_rows = bays::Entity::find()
        .filter(bays::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let bed_rows = beds::Entity::find()
        .filter(beds::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let mut out = Vec::new();
    for bed in bed_rows {
        let Some(bay) = bay_rows.iter().find(|b| b.pid == bed.bay_pid) else {
            continue;
        };
        let Some(ward) = ward_rows.iter().find(|w| w.pid == bay.ward_pid) else {
            continue;
        };
        let Ok(state) = BedState::parse(&bed.state) else {
            continue;
        };
        let ward_matches_target = request
            .target_ward_pid
            .is_none_or(|target| target == ward.pid);
        let specialty_matches = match (&request.specialty, &ward.specialty) {
            (Some(want), Some(have)) => want.eq_ignore_ascii_case(have),
            (None, _) => true,
            (Some(_), None) => false,
        };
        let facts = BedFacts {
            state,
            ward_open: ward.open,
            ward_closed_to_admissions: ward.closed_to_admissions,
            bay_closed_to_admissions: bay.closed_to_admissions,
            bay_sex_designation: bay.sex_designation.clone(),
            side_room: bay.side_room,
            isolation_capable: bed.isolation_capable,
            oxygen: bed.oxygen,
            bariatric: bed.bariatric,
            ward_matches_target,
            specialty_matches,
            is_virtual: bed.is_virtual,
        };
        out.push((bed, bay.clone(), ward.clone(), facts));
    }
    Ok(out)
}

/// The ranked eligible beds for a request (no overrides applied —
/// this is the advisory list).
async fn eligible_for(
    db: &DatabaseConnection,
    request: &bed_requests::Model,
) -> Result<Vec<EligibleBed>> {
    let requirements: Requirements =
        serde_json::from_value(request.requirements.clone()).unwrap_or_default();
    let mut hits: Vec<_> = all_bed_facts(db, request)
        .await?
        .into_iter()
        .filter(|(_, _, _, facts)| {
            allocation::breaches(facts, &requirements, Overrides::default()).is_empty()
        })
        .collect();
    hits.sort_by_key(|(_, _, _, facts)| allocation::rank_key(facts, &requirements));
    Ok(hits
        .into_iter()
        .map(|(bed, bay, ward, facts)| EligibleBed {
            bed_pid: bed.pid.to_string(),
            number: bed.number,
            ward_pid: ward.pid.to_string(),
            ward_code: ward.code,
            bay_name: bay.name,
            side_room: facts.side_room,
            right_ward: facts.ward_matches_target,
        })
        .collect())
}

/// `POST /api/bed-requests`.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<RequestPayload>,
) -> Result<Response> {
    ensure_valid(&validate_request(&payload))?;
    if let Some(ward_pid) = payload.target_ward_pid {
        records::find_ward(&ctx.db, ward_pid).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = bed_requests::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        origin: ActiveValue::set(payload.origin.clone()),
        target_ward_pid: ActiveValue::set(payload.target_ward_pid),
        specialty: ActiveValue::set(payload.specialty.clone()),
        priority: ActiveValue::set(payload.priority.clone()),
        requirements: ActiveValue::set(serde_json::to_value(&payload.requirements).unwrap_or_default()),
        status: ActiveValue::set("open".to_string()),
        allocated_bed_pid: ActiveValue::set(None),
        requested_at: ActiveValue::set(chrono::Utc::now().into()),
        resolved_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "bed_request", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(&txn, "bed_request", "bed_request_created", &row.pid.to_string(), &row.priority, caller.actor(), None).await?;
    txn.commit().await?;
    Metrics::global().bed_request_created_total.inc();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/bed-requests?status=open` — the demand board, priority
/// then wait order, each with its live eligible-bed count.
#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list(
    State(ctx): State<AppContext>,
    Query(params): Query<ListParams>,
) -> Result<Response> {
    let status = params.status.unwrap_or_else(|| "open".to_string());
    let rows = bed_requests::Entity::find()
        .filter(bed_requests::Column::DeletedAt.is_null())
        .filter(bed_requests::Column::Status.eq(status))
        .order_by_asc(bed_requests::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut views = Vec::with_capacity(rows.len());
    for request in rows {
        let eligible = if request.status == "open" {
            eligible_for(&ctx.db, &request).await?.len()
        } else {
            0
        };
        views.push(RequestView { request, eligible_beds: eligible });
    }
    // Priority order: emergency, urgent, routine; then longest wait.
    let weight = |p: &str| match p {
        "emergency" => 0_u8,
        "urgent" => 1,
        _ => 2,
    };
    views.sort_by(|a, b| {
        weight(&a.request.priority)
            .cmp(&weight(&b.request.priority))
            .then(a.request.requested_at.cmp(&b.request.requested_at))
    });
    format::json(views)
}

/// `GET /api/bed-requests/{pid}/eligible` — the ranked advisory list.
#[debug_handler]
async fn eligible(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let request = records::find_bed_request(&ctx.db, records::parse_pid(&pid)?).await?;
    if request.status != "open" {
        return Err(unprocessable(&format!("request is {}", request.status)));
    }
    format::json(eligible_for(&ctx.db, &request).await?)
}

/// `POST /api/bed-requests/{pid}/allocate` — reserve a chosen bed for
/// the request. Rule-checked; rules 2/5 overridable with a recorded
/// reason (audited — a reportable governance event).
#[debug_handler]
async fn allocate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<AllocatePayload>,
) -> Result<Response> {
    if (payload.override_sex || payload.override_ward_fit)
        && payload.override_reason.as_deref().is_none_or(|r| r.trim().is_empty())
    {
        return Err(unprocessable("override_reason is required when overriding a rule"));
    }
    let request = records::find_bed_request(&ctx.db, records::parse_pid(&pid)?).await?;
    if request.status != "open" {
        return Err(unprocessable(&format!("request is {}", request.status)));
    }
    let requirements: Requirements =
        serde_json::from_value(request.requirements.clone()).unwrap_or_default();
    // Assemble facts for the chosen bed and rule-check it.
    let chosen = all_bed_facts(&ctx.db, &request)
        .await?
        .into_iter()
        .find(|(bed, _, _, _)| bed.pid == payload.bed_pid)
        .ok_or(Error::NotFound)?;
    let (_, _, _, facts) = &chosen;
    let overrides = Overrides {
        sex: payload.override_sex,
        ward_fit: payload.override_ward_fit,
    };
    let breaches = allocation::breaches(facts, &requirements, overrides);
    if !breaches.is_empty() {
        return Err(unprocessable(&format!("bed ineligible: {breaches:?}")));
    }
    let txn = ctx.db.begin().await?;
    // Lock + re-check the bed, then reserve it.
    let bed = beds::Entity::find()
        .filter(beds::Column::Pid.eq(payload.bed_pid))
        .filter(beds::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    let outcome = super::topology::apply_transition(&bed, &Transition::Allocate)?;
    super::topology::persist_outcome(&txn, bed, &outcome, caller.actor()).await?;
    let request_pid = request.pid;
    let mut active: bed_requests::ActiveModel = request.into();
    active.status = ActiveValue::set("allocated".to_string());
    active.allocated_bed_pid = ActiveValue::set(Some(payload.bed_pid));
    let row = active.update(&txn).await?;
    let snapshot = serde_json::json!({
        "bed_pid": payload.bed_pid.to_string(),
        "override_sex": payload.override_sex,
        "override_ward_fit": payload.override_ward_fit,
        "override_reason": payload.override_reason,
    });
    Audit::record(&txn, "bed_request", request_pid, "bed_request_allocated", caller.actor(), Some(snapshot)).await?;
    streaming::emit_on(&txn, "bed_request", "bed_request_allocated", &request_pid.to_string(), &row.priority, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/bed-requests/{pid}/cancel` — cancel an open/allocated
/// request; an allocated bed is released back to available.
#[debug_handler]
async fn cancel(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let request = records::find_bed_request(&ctx.db, records::parse_pid(&pid)?).await?;
    if !matches!(request.status.as_str(), "open" | "allocated") {
        return Err(unprocessable(&format!("request is {}", request.status)));
    }
    let txn = ctx.db.begin().await?;
    if let Some(bed_pid) = request.allocated_bed_pid {
        let bed = beds::Entity::find()
            .filter(beds::Column::Pid.eq(bed_pid))
            .filter(beds::Column::DeletedAt.is_null())
            .lock_exclusive()
            .one(&txn)
            .await?;
        if let Some(bed) = bed
            && bed.state == "reserved" {
                let outcome = super::topology::apply_transition(&bed, &Transition::Release)?;
                super::topology::persist_outcome(&txn, bed, &outcome, caller.actor()).await?;
            }
    }
    let request_pid = request.pid;
    let mut active: bed_requests::ActiveModel = request.into();
    active.status = ActiveValue::set("cancelled".to_string());
    active.resolved_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&txn).await?;
    Audit::record(&txn, "bed_request", request_pid, "bed_request_cancelled", caller.actor(), None).await?;
    streaming::emit_on(&txn, "bed_request", "bed_request_cancelled", &request_pid.to_string(), &row.priority, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// The bed-request routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/bed-requests", post(create))
        .add("/bed-requests", get(list))
        .add("/bed-requests/{pid}/eligible", get(eligible))
        .add("/bed-requests/{pid}/allocate", post(allocate))
        .add("/bed-requests/{pid}/cancel", post(cancel))
}
