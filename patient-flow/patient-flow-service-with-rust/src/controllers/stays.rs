//! Stay lifecycle: admit → update / transfer / `Red2Green` / infection
//! flags → discharge-ready → discharge (spec `patient-journey.md`).
//!
//! Placement paths lock the affected bed rows (`FOR UPDATE`) and run
//! stay change + bed transition + transfer row + audit + events on one
//! transaction (PF-D9). Stay detail is a **sensitive read**: it is
//! audited and honours the ABAC `mask` obligation.

use authentication_verifier::Action;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::flow::bed_state::{BedState, Transition};
use crate::flow::{journey, tokens};
use crate::metrics::Metrics;
use crate::models::_entities::{
    bays, beds, infection_flags, red_green_days, stays, transfers, wards,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/stays` — admit.
#[derive(Debug, Deserialize)]
struct AdmitPayload {
    /// The patient: `person:<uuid>` URN.
    person_ref: String,
    /// Caller-supplied display name (else resolved best-effort).
    #[serde(default)]
    display_name: Option<String>,
    /// Admission source token.
    source: String,
    /// The destination bed (must be available or reserved).
    bed_pid: Uuid,
    /// The fulfilled bed request, when the admission serves one.
    #[serde(default)]
    bed_request_pid: Option<Uuid>,
    #[serde(default)]
    named_nurse_ref: Option<String>,
    #[serde(default)]
    consultant_ref: Option<String>,
    /// SAFER "A": expected discharge date, ideally set on admission.
    #[serde(default)]
    edd: Option<chrono::NaiveDate>,
    #[serde(default)]
    ccd: Option<String>,
    /// Virtual-ward stays: where the patient is.
    #[serde(default)]
    home_location_note: Option<String>,
    #[serde(default)]
    alerts: Vec<String>,
}

/// `PUT /api/stays/{pid}` — the whiteboard-editable fields.
#[derive(Debug, Deserialize)]
struct StayUpdate {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    named_nurse_ref: Option<String>,
    #[serde(default)]
    consultant_ref: Option<String>,
    #[serde(default)]
    edd: Option<chrono::NaiveDate>,
    #[serde(default)]
    ccd: Option<String>,
    #[serde(default)]
    ccd_met: Option<bool>,
    /// Stamp SAFER "S": the senior review happened now.
    #[serde(default)]
    senior_review_now: bool,
    #[serde(default)]
    alerts: Option<Vec<String>>,
    #[serde(default)]
    home_location_note: Option<String>,
}

/// `POST /api/stays/{pid}/transfer`.
#[derive(Debug, Deserialize)]
struct TransferPayload {
    to_bed_pid: Uuid,
    /// A transfer-reason token.
    reason: String,
    /// Operator overrides for allocation rules 2 / 5 (audited).
    #[serde(default)]
    override_sex: bool,
    #[serde(default)]
    override_ward_fit: bool,
    #[serde(default)]
    override_reason: Option<String>,
}

/// `POST /api/stays/{pid}/discharge-ready`.
#[derive(Debug, Deserialize)]
struct DischargeReadyPayload {
    /// Discharge pathway token (p0–p3).
    pathway: String,
}

/// `POST /api/stays/{pid}/discharge`.
#[derive(Debug, Deserialize)]
struct DischargePayload {
    destination: String,
}

/// `POST /api/stays/{pid}/red-green` — record/update today's day.
#[derive(Debug, Deserialize)]
struct RedGreenPayload {
    classification: String,
    #[serde(default)]
    delay_reasons: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/stays/{pid}/infection-flags`.
#[derive(Debug, Deserialize)]
struct FlagPayload {
    precaution: String,
    #[serde(default)]
    organism: Option<String>,
    /// `suspected` or `confirmed`.
    status: String,
    #[serde(default)]
    requires_side_room: bool,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn validate_admit(p: &AdmitPayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_ref("person_ref", entity_ref::EntityType::Person, &p.person_ref);
    problems.require_token("source", tokens::STAY_SOURCES, &p.source);
    problems.cap_opt("display_name", p.display_name.as_deref());
    problems.ref_opt(
        "named_nurse_ref",
        entity_ref::EntityType::Worker,
        p.named_nurse_ref.as_deref(),
    );
    problems.ref_opt(
        "consultant_ref",
        entity_ref::EntityType::Worker,
        p.consultant_ref.as_deref(),
    );
    problems.cap_opt("ccd", p.ccd.as_deref());
    problems.cap_opt("home_location_note", p.home_location_note.as_deref());
    problems.cap_list("alerts", &p.alerts);
    problems.into_vec()
}

/// The bed's bay + ward context, loaded together.
async fn bed_context<C: sea_orm::ConnectionTrait>(
    db: &C,
    bed: &beds::Model,
) -> Result<(bays::Model, wards::Model)> {
    let bay = records::find_bay(db, bed.bay_pid).await?;
    let ward = records::find_ward(db, bay.ward_pid).await?;
    Ok((bay, ward))
}

/// Load + exclusively lock an active bed row inside `txn`.
async fn lock_bed(txn: &sea_orm::DatabaseTransaction, pid: Uuid) -> Result<beds::Model> {
    beds::Entity::find()
        .filter(beds::Column::Pid.eq(pid))
        .filter(beds::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(txn)
        .await?
        .ok_or(Error::NotFound)
}

/// The one-occupant invariant: no other active stay may reference the
/// bed.
async fn assert_unoccupied(txn: &sea_orm::DatabaseTransaction, bed_pid: Uuid) -> Result<()> {
    let occupant = stays::Entity::find()
        .filter(stays::Column::BedPid.eq(bed_pid))
        .filter(stays::Column::DischargedAt.is_null())
        .filter(stays::Column::DeletedAt.is_null())
        .one(txn)
        .await?;
    if occupant.is_some() {
        return Err(unprocessable("bed already has an active occupant"));
    }
    Ok(())
}

/// Whether the stay carries an uncleared transmissible infection flag
/// (contact/droplet/airborne ⇒ the vacated bed needs a deep clean).
async fn is_infectious<C: sea_orm::ConnectionTrait>(db: &C, stay_pid: Uuid) -> Result<bool> {
    let flags = infection_flags::Entity::find()
        .filter(infection_flags::Column::StayPid.eq(stay_pid))
        .filter(infection_flags::Column::ClearedAt.is_null())
        .all(db)
        .await?;
    Ok(flags.iter().any(|f| f.precaution != "protective"))
}

/// `POST /api/stays` — admit a patient into a bed.
// One line over the pedantic limit (101/100). Admission is a single
// transaction that validates the payload, checks bed state, writes the
// stay, and emits its event; splitting it to save one line would scatter
// that transaction across helpers for no readability gain. Surfaced only
// once CI began treating clippy warnings as errors.
#[allow(clippy::too_many_lines)]
#[debug_handler]
async fn admit(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AdmitPayload>,
) -> Result<Response> {
    ensure_valid(&validate_admit(&payload))?;
    // Resolve the display name outside the transaction (best-effort;
    // never blocks the admission — PF-D11).
    let display_name = if let Some(name) = &payload.display_name {
        name.clone()
    } else {
        let entity_ref: entity_ref::EntityRef = payload
            .person_ref
            .parse()
            .map_err(|_| unprocessable("bad person_ref"))?;
        crate::clients::display_name(&entity_ref)
            .await
            .unwrap_or_else(|| payload.person_ref.clone())
    };
    let txn = ctx.db.begin().await?;
    let bed = lock_bed(&txn, payload.bed_pid).await?;
    assert_unoccupied(&txn, bed.pid).await?;
    let (_, ward) = bed_context(&txn, &bed).await?;
    if !ward.open {
        return Err(unprocessable("ward is closed"));
    }
    let outcome = super::topology::apply_transition(&bed, &Transition::Admit)?;
    let bed_row = super::topology::persist_outcome(&txn, bed, &outcome, caller.actor()).await?;
    let stay = stays::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        display_name: ActiveValue::set(display_name),
        status: ActiveValue::set("admitted".to_string()),
        admitted_at: ActiveValue::set(chrono::Utc::now().into()),
        source: ActiveValue::set(payload.source.clone()),
        ward_pid: ActiveValue::set(Some(ward.pid)),
        bed_pid: ActiveValue::set(Some(bed_row.pid)),
        home_location_note: ActiveValue::set(payload.home_location_note.clone()),
        named_nurse_ref: ActiveValue::set(payload.named_nurse_ref.clone()),
        consultant_ref: ActiveValue::set(payload.consultant_ref.clone()),
        senior_review_at: ActiveValue::set(None),
        edd: ActiveValue::set(payload.edd),
        ccd: ActiveValue::set(payload.ccd.clone()),
        ccd_met: ActiveValue::set(false),
        discharge_pathway: ActiveValue::set(None),
        discharge_ready_at: ActiveValue::set(None),
        discharged_at: ActiveValue::set(None),
        discharge_destination: ActiveValue::set(None),
        alerts: ActiveValue::set(serde_json::json!(payload.alerts)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    transfers::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        stay_pid: ActiveValue::set(stay.pid),
        from_bed_pid: ActiveValue::set(None),
        to_bed_pid: ActiveValue::set(Some(bed_row.pid)),
        reason: ActiveValue::set("admission".to_string()),
        moved_at: ActiveValue::set(chrono::Utc::now().into()),
        moved_by_ref: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    // Fulfil the originating bed request, when named.
    if let Some(request_pid) = payload.bed_request_pid
        && let Ok(request) = records::find_bed_request(&txn, request_pid).await
    {
        let mut active: crate::models::_entities::bed_requests::ActiveModel = request.into();
        active.status = ActiveValue::set("fulfilled".to_string());
        active.resolved_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        active.update(&txn).await?;
    }
    let snapshot = serde_json::json!({
        "ward_pid": ward.pid.to_string(),
        "bed_pid": bed_row.pid.to_string(),
        "source": payload.source,
        "edd_missing": stay.edd.is_none(),
    });
    Audit::record(
        &txn,
        "stay",
        stay.pid,
        "stay_admitted",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "stay",
        "stay_admitted",
        &stay.pid.to_string(),
        &stay.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    Metrics::global().stay_admitted_total.inc();
    // SAFER "A" nudge: flag a missing EDD in the response.
    format::json(serde_json::json!({
        "pid": stay.pid.to_string(),
        "ward_pid": ward.pid.to_string(),
        "edd_missing": stay.edd.is_none(),
    }))
}

/// The full stay view: the row plus its transfers, `Red2Green` run, and
/// infection flags.
#[derive(Debug, Serialize)]
struct StayDetail {
    stay: stays::Model,
    transfers: Vec<transfers::Model>,
    red_green: Vec<red_green_days::Model>,
    infection_flags: Vec<infection_flags::Model>,
    length_of_stay_days: i64,
    dtoc: bool,
}

/// `GET /api/stays/{pid}` — the MDT view. Sensitive read: audited,
/// record-level ABAC + `mask` obligation honoured.
#[debug_handler]
async fn get_stay(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    let obligations =
        auth::authorize_record(&caller, Action::Read, &auth::stay_resource_attrs(&stay))
            .map_err(record_rejection)?;
    let mut stay = stay;
    if obligations.iter().any(|o| o == "mask") {
        stay = auth::mask_stay(stay);
    }
    let transfer_rows = transfers::Entity::find()
        .filter(transfers::Column::StayPid.eq(stay.pid))
        .order_by_asc(transfers::Column::Id)
        .all(&ctx.db)
        .await?;
    let red_green = red_green_days::Entity::find()
        .filter(red_green_days::Column::StayPid.eq(stay.pid))
        .order_by_asc(red_green_days::Column::Day)
        .all(&ctx.db)
        .await?;
    let flags = infection_flags::Entity::find()
        .filter(infection_flags::Column::StayPid.eq(stay.pid))
        .order_by_asc(infection_flags::Column::Id)
        .all(&ctx.db)
        .await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let detail = StayDetail {
        length_of_stay_days: journey::length_of_stay_days(
            stay.admitted_at,
            stay.discharged_at,
            now,
        ),
        dtoc: journey::is_dtoc(stay.discharge_ready_at, stay.discharged_at, now),
        stay,
        transfers: transfer_rows,
        red_green,
        infection_flags: flags,
    };
    // Sensitive read → audit (family posture; spec `audit.md`).
    Audit::record(
        &ctx.db,
        "stay",
        detail.stay.pid,
        "stay_read",
        caller.actor(),
        detail
            .stay
            .ward_pid
            .map(|w| serde_json::json!({ "ward_pid": w.to_string() })),
    )
    .await?;
    format::json(detail)
}

/// `GET /api/stays/{pid}/time-analysis` — this stay as one leg of a
/// stitched patient journey.
///
/// Satisfies the timeline contract that `care-pathway-service` follows
/// across a `continues_as` link (its `src/journey.rs`): four numbers —
/// clock bounds, elapsed span, and value-adding time — so a journey
/// that begins on a care pathway and continues into an inpatient stay
/// can be measured end to end instead of stopping at the boundary.
///
/// **A green day is the value-adding time.** `Red2Green` already
/// answers time-based analysis's question in the NHS's own vocabulary:
/// a green day moves the patient toward discharge, a red day does not.
/// Nothing new had to be invented, and nothing here is a clinical
/// judgement this service is not entitled to make.
///
/// Unclassified days count as **non-value-adding**, matching the
/// consuming service's denominator rule — elapsed calendar time is the
/// denominator and unrecorded time counts against you, because the
/// alternative rewards recording less. So the figure is a floor, and
/// `coverage` / `confidence` say how much of the stay it rests on: an
/// unclassified stay and a genuinely red one both report little
/// value-adding time, and only that distinguishes them.
///
/// **A coverage ceiling worth knowing about.** `POST
/// /api/stays/{pid}/red-green` classifies *today* and takes no `day`,
/// so a stay admitted before the board was in use can never be fully
/// classified retrospectively. Its coverage is capped by when
/// classification started, not by how diligent the ward was — which is
/// why the confidence label matters more here than a bare percentage.
///
/// A **sensitive read** like the MDT view: record-level ABAC, audited.
/// The `mask` obligation is deliberately not applied — the response
/// carries no identifiers, only durations, so there is nothing in it to
/// redact.
#[debug_handler]
async fn stay_time_analysis(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(&caller, Action::Read, &auth::stay_resource_attrs(&stay))
        .map_err(record_rejection)?;
    let rows = red_green_days::Entity::find()
        .filter(red_green_days::Column::StayPid.eq(stay.pid))
        .order_by_asc(red_green_days::Column::Day)
        .all(&ctx.db)
        .await?;
    let classifications: Vec<(chrono::NaiveDate, String)> = rows
        .iter()
        .map(|r| (r.day, r.classification.clone()))
        .collect();
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let timeline =
        journey::stay_timeline(stay.admitted_at, stay.discharged_at, now, &classifications);

    // Reading a stay's timeline is reading the stay, so it is audited
    // like the MDT view — a caller assembling a cross-service journey
    // leaves the same trail as one opening the record.
    Audit::record(
        &ctx.db,
        "stay",
        stay.pid,
        "stay_time_analysis_read",
        caller.actor(),
        None,
    )
    .await?;

    format::json(serde_json::json!({
        "as_of": now,
        "stay": { "pid": stay.pid, "status": stay.status },
        "note": "one leg of a stitched patient journey. A green Red2Green day \
                 is value-adding time — the method already answers that \
                 question in the NHS's own vocabulary. Unclassified days count \
                 as non-value-adding, so value_time_ms is a floor; `coverage` \
                 and `confidence` say how much of the stay it rests on. An \
                 unclassified stay and a genuinely red one both report little \
                 value-adding time, and only the confidence tells them apart.",
        "clock": {
            "start_ms": timeline.clock_start_ms,
            "stop_ms": timeline.clock_stop_ms,
            "start_source": "admitted_at",
            "stop_source": if stay.discharged_at.is_some() { "discharged_at" } else { "as_of" },
            "running": stay.discharged_at.is_none(),
        },
        "lead_time_ms": timeline.lead_time_ms,
        "value_time_ms": timeline.value_time_ms,
        "span_days": timeline.span_days,
        "classified_days": timeline.classified_days,
        "green_days": timeline.green_days,
        "coverage": timeline.coverage_ratio(),
        "confidence": timeline.confidence(),
    }))
}

/// `PUT /api/stays/{pid}` — whiteboard-editable fields (EDD, CCD,
/// named staff, alerts, senior review).
#[debug_handler]
async fn update_stay(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StayUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.cap_opt("display_name", payload.display_name.as_deref());
    problems.ref_opt(
        "named_nurse_ref",
        entity_ref::EntityType::Worker,
        payload.named_nurse_ref.as_deref(),
    );
    problems.ref_opt(
        "consultant_ref",
        entity_ref::EntityType::Worker,
        payload.consultant_ref.as_deref(),
    );
    problems.cap_opt("ccd", payload.ccd.as_deref());
    problems.cap_opt("home_location_note", payload.home_location_note.as_deref());
    if let Some(alerts) = &payload.alerts {
        problems.cap_list("alerts", alerts);
    }
    ensure_valid(&problems.into_vec())?;
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    if stay.status == "discharged" {
        return Err(unprocessable("stay is discharged"));
    }
    let txn = ctx.db.begin().await?;
    let ward_pid = stay.ward_pid;
    let stay_pid = stay.pid;
    let mut active: stays::ActiveModel = stay.into();
    if let Some(v) = payload.display_name {
        active.display_name = ActiveValue::set(v);
    }
    if let Some(v) = payload.named_nurse_ref {
        active.named_nurse_ref = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.consultant_ref {
        active.consultant_ref = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.edd {
        active.edd = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.ccd {
        active.ccd = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.ccd_met {
        active.ccd_met = ActiveValue::set(v);
    }
    if payload.senior_review_now {
        active.senior_review_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    }
    if let Some(v) = payload.alerts {
        active.alerts = ActiveValue::set(serde_json::json!(v));
    }
    if let Some(v) = payload.home_location_note {
        active.home_location_note = ActiveValue::set(Some(v));
    }
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "stay",
        stay_pid,
        "updated",
        caller.actor(),
        ward_pid.map(|w| serde_json::json!({ "ward_pid": w.to_string() })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "stay",
        "updated",
        &stay_pid.to_string(),
        &row.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/stays/{pid}/transfer` — move to another bed.
#[debug_handler]
#[allow(clippy::too_many_lines)] // both bed transitions + eligibility in one transaction
async fn transfer(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<TransferPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("reason", tokens::TRANSFER_REASONS, &payload.reason);
    if (payload.override_sex || payload.override_ward_fit)
        && payload
            .override_reason
            .as_deref()
            .is_none_or(|r| r.trim().is_empty())
    {
        problems.push("override_reason is required when overriding an allocation rule");
    }
    ensure_valid(&problems.into_vec())?;
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    if stay.status == "discharged" {
        return Err(unprocessable("stay is discharged"));
    }
    let from_bed_pid = stay
        .bed_pid
        .ok_or_else(|| unprocessable("stay has no current bed"))?;
    if from_bed_pid == payload.to_bed_pid {
        return Err(unprocessable("destination is the current bed"));
    }
    let infectious = is_infectious(&ctx.db, stay.pid).await?;
    let txn = ctx.db.begin().await?;
    // Lock both beds in pid order (deadlock avoidance), then re-load.
    let (first, second) = if from_bed_pid < payload.to_bed_pid {
        (from_bed_pid, payload.to_bed_pid)
    } else {
        (payload.to_bed_pid, from_bed_pid)
    };
    let bed_a = lock_bed(&txn, first).await?;
    let bed_b = lock_bed(&txn, second).await?;
    let (from_bed, to_bed) = if bed_a.pid == from_bed_pid {
        (bed_a, bed_b)
    } else {
        (bed_b, bed_a)
    };
    assert_unoccupied(&txn, to_bed.pid).await?;
    // Destination eligibility (same rules as allocation): a stay with
    // an uncleared transmissible flag may only move to a side room /
    // isolation-capable bed (spec `infection-control.md`).
    let (to_bay, to_ward) = bed_context(&txn, &to_bed).await?;
    let facts = crate::flow::allocation::BedFacts {
        state: BedState::parse(&to_bed.state).map_err(|e| unprocessable(&e.to_string()))?,
        ward_open: to_ward.open,
        ward_closed_to_admissions: to_ward.closed_to_admissions,
        bay_closed_to_admissions: to_bay.closed_to_admissions,
        bay_sex_designation: to_bay.sex_designation.clone(),
        side_room: to_bay.side_room,
        isolation_capable: to_bed.isolation_capable,
        oxygen: to_bed.oxygen,
        bariatric: to_bed.bariatric,
        // A directed transfer names its destination — ward fit is the
        // clinician's call unless they marked it an outlier override.
        ward_matches_target: !payload.override_ward_fit,
        specialty_matches: false,
        is_virtual: to_bed.is_virtual,
    };
    let requirements = crate::flow::allocation::Requirements {
        isolation: infectious,
        ..Default::default()
    };
    let overrides = crate::flow::allocation::Overrides {
        sex: payload.override_sex,
        ward_fit: payload.override_ward_fit,
    };
    // Reserved destinations are permitted for a directed transfer when
    // the reservation is being consumed; only genuine breaches refuse.
    let breaches: Vec<_> = crate::flow::allocation::breaches(&facts, &requirements, overrides)
        .into_iter()
        .filter(|b| {
            !(matches!(b, crate::flow::allocation::Breach::NotOpen)
                && facts.state == BedState::Reserved
                && to_ward.open
                && !facts.ward_closed_to_admissions
                && !facts.bay_closed_to_admissions)
        })
        .collect();
    if !breaches.is_empty() {
        return Err(unprocessable(&format!(
            "destination bed ineligible: {breaches:?}"
        )));
    }
    // Vacate the old bed, occupy the new one.
    let out_old = super::topology::apply_transition(&from_bed, &Transition::Vacate { infectious })?;
    super::topology::persist_outcome(&txn, from_bed, &out_old, caller.actor()).await?;
    let out_new = super::topology::apply_transition(&to_bed, &Transition::Admit)?;
    let to_bed_row =
        super::topology::persist_outcome(&txn, to_bed, &out_new, caller.actor()).await?;
    let stay_pid = stay.pid;
    let display_name = stay.display_name.clone();
    let mut active: stays::ActiveModel = stay.into();
    active.ward_pid = ActiveValue::set(Some(to_ward.pid));
    active.bed_pid = ActiveValue::set(Some(to_bed_row.pid));
    let row = active.update(&txn).await?;
    transfers::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        stay_pid: ActiveValue::set(stay_pid),
        from_bed_pid: ActiveValue::set(Some(from_bed_pid)),
        to_bed_pid: ActiveValue::set(Some(to_bed_row.pid)),
        reason: ActiveValue::set(payload.reason.clone()),
        moved_at: ActiveValue::set(chrono::Utc::now().into()),
        moved_by_ref: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let snapshot = serde_json::json!({
        "ward_pid": to_ward.pid.to_string(),
        "from_bed_pid": from_bed_pid.to_string(),
        "to_bed_pid": to_bed_row.pid.to_string(),
        "reason": payload.reason,
        "override_sex": payload.override_sex,
        "override_ward_fit": payload.override_ward_fit,
        "override_reason": payload.override_reason,
    });
    Audit::record(
        &txn,
        "stay",
        stay_pid,
        "stay_transferred",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "stay",
        "stay_transferred",
        &stay_pid.to_string(),
        &display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    Metrics::global().stay_transferred_total.inc();
    format::json(row)
}

/// `POST /api/stays/{pid}/discharge-ready` — requires EDD + CCD met.
#[debug_handler]
async fn discharge_ready(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<DischargeReadyPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("pathway", tokens::DISCHARGE_PATHWAYS, &payload.pathway);
    ensure_valid(&problems.into_vec())?;
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    if stay.status != "admitted" {
        return Err(unprocessable(&format!("stay is {}", stay.status)));
    }
    if stay.edd.is_none() {
        return Err(unprocessable(
            "set an expected discharge date first (SAFER)",
        ));
    }
    if !stay.ccd_met {
        return Err(unprocessable("clinical criteria for discharge are not met"));
    }
    let txn = ctx.db.begin().await?;
    let stay_pid = stay.pid;
    let ward_pid = stay.ward_pid;
    let display_name = stay.display_name.clone();
    let mut active: stays::ActiveModel = stay.into();
    active.status = ActiveValue::set("discharge_ready".to_string());
    active.discharge_ready_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.discharge_pathway = ActiveValue::set(Some(payload.pathway.clone()));
    let row = active.update(&txn).await?;
    let snapshot = serde_json::json!({
        "pathway": payload.pathway,
        "ward_pid": ward_pid.map(|w| w.to_string()),
    });
    Audit::record(
        &txn,
        "stay",
        stay_pid,
        "stay_discharge_ready",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "stay",
        "stay_discharge_ready",
        &stay_pid.to_string(),
        &display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/stays/{pid}/discharge`.
#[debug_handler]
async fn discharge(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<DischargePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token(
        "destination",
        tokens::DISCHARGE_DESTINATIONS,
        &payload.destination,
    );
    ensure_valid(&problems.into_vec())?;
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    if stay.status == "discharged" {
        return Err(unprocessable("stay is already discharged"));
    }
    let infectious = is_infectious(&ctx.db, stay.pid).await?;
    let txn = ctx.db.begin().await?;
    // Vacate the bed (if any — virtual stays always have a slot too).
    if let Some(bed_pid) = stay.bed_pid {
        let bed = lock_bed(&txn, bed_pid).await?;
        let outcome = super::topology::apply_transition(&bed, &Transition::Vacate { infectious })?;
        super::topology::persist_outcome(&txn, bed, &outcome, caller.actor()).await?;
        transfers::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            stay_pid: ActiveValue::set(stay.pid),
            from_bed_pid: ActiveValue::set(Some(bed_pid)),
            to_bed_pid: ActiveValue::set(None),
            reason: ActiveValue::set("discharge".to_string()),
            moved_at: ActiveValue::set(chrono::Utc::now().into()),
            moved_by_ref: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    // Close out open infection flags (the bed keeps its deep-clean flag).
    let open_flags = infection_flags::Entity::find()
        .filter(infection_flags::Column::StayPid.eq(stay.pid))
        .filter(infection_flags::Column::ClearedAt.is_null())
        .all(&txn)
        .await?;
    for flag in open_flags {
        let mut active: infection_flags::ActiveModel = flag.into();
        active.status = ActiveValue::set("cleared".to_string());
        active.cleared_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        active.update(&txn).await?;
    }
    let stay_pid = stay.pid;
    let ward_pid = stay.ward_pid;
    let display_name = stay.display_name.clone();
    let mut active: stays::ActiveModel = stay.into();
    active.status = ActiveValue::set("discharged".to_string());
    active.discharged_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.discharge_destination = ActiveValue::set(Some(payload.destination.clone()));
    active.bed_pid = ActiveValue::set(None);
    let row = active.update(&txn).await?;
    let snapshot = serde_json::json!({
        "destination": payload.destination,
        "ward_pid": ward_pid.map(|w| w.to_string()),
    });
    Audit::record(
        &txn,
        "stay",
        stay_pid,
        "stay_discharged",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "stay",
        "stay_discharged",
        &stay_pid.to_string(),
        &display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    Metrics::global().stay_discharged_total.inc();
    format::json(row)
}

/// `POST /api/stays/{pid}/red-green` — record (or same-day update)
/// today's `Red2Green` entry. Days before today are frozen.
#[debug_handler]
async fn red_green(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<RedGreenPayload>,
) -> Result<Response> {
    journey::validate_red_green(&payload.classification, &payload.delay_reasons)
        .map_err(|problems| super::ensure_valid(&problems).expect_err("non-empty problems"))?;
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    if stay.status == "discharged" {
        return Err(unprocessable("stay is discharged"));
    }
    let today = chrono::Utc::now().date_naive();
    let txn = ctx.db.begin().await?;
    let existing = red_green_days::Entity::find()
        .filter(red_green_days::Column::StayPid.eq(stay.pid))
        .filter(red_green_days::Column::Day.eq(today))
        .one(&txn)
        .await?;
    let row = if let Some(existing) = existing {
        let mut active: red_green_days::ActiveModel = existing.into();
        active.classification = ActiveValue::set(payload.classification.clone());
        active.delay_reasons = ActiveValue::set(serde_json::json!(payload.delay_reasons));
        active.note = ActiveValue::set(payload.note.clone());
        active.update(&txn).await?
    } else {
        red_green_days::ActiveModel {
            stay_pid: ActiveValue::set(stay.pid),
            day: ActiveValue::set(today),
            classification: ActiveValue::set(payload.classification.clone()),
            delay_reasons: ActiveValue::set(serde_json::json!(payload.delay_reasons)),
            note: ActiveValue::set(payload.note.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?
    };
    let snapshot = serde_json::json!({
        "day": today.to_string(),
        "classification": payload.classification,
        "delay_reasons": payload.delay_reasons,
        "ward_pid": stay.ward_pid.map(|w| w.to_string()),
    });
    Audit::record(
        &txn,
        "red_green",
        stay.pid,
        "red_green_recorded",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "red_green",
        "red_green_recorded",
        &stay.pid.to_string(),
        &stay.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/stays/{pid}/infection-flags` — raise a precaution flag.
#[debug_handler]
async fn add_flag(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<FlagPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("precaution", tokens::PRECAUTIONS, &payload.precaution);
    problems.require_token("status", &["suspected", "confirmed"], &payload.status);
    problems.cap_opt("organism", payload.organism.as_deref());
    ensure_valid(&problems.into_vec())?;
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    if stay.status == "discharged" {
        return Err(unprocessable("stay is discharged"));
    }
    let txn = ctx.db.begin().await?;
    let row = infection_flags::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        stay_pid: ActiveValue::set(stay.pid),
        precaution: ActiveValue::set(payload.precaution.clone()),
        organism: ActiveValue::set(payload.organism.clone()),
        status: ActiveValue::set(payload.status.clone()),
        requires_side_room: ActiveValue::set(payload.requires_side_room),
        flagged_at: ActiveValue::set(chrono::Utc::now().into()),
        cleared_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let snapshot = serde_json::json!({
        "precaution": payload.precaution,
        "organism": payload.organism,
        "status": payload.status,
        "ward_pid": stay.ward_pid.map(|w| w.to_string()),
    });
    Audit::record(
        &txn,
        "infection_flag",
        row.pid,
        "infection_flagged",
        caller.actor(),
        Some(snapshot),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "infection_flag",
        "infection_flagged",
        &row.pid.to_string(),
        &stay.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `POST /api/stays/{pid}/infection-flags/{flag_pid}/clear`.
#[debug_handler]
async fn clear_flag(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((pid, flag_pid)): Path<(String, String)>,
) -> Result<Response> {
    let stay = records::find_stay(&ctx.db, records::parse_pid(&pid)?).await?;
    let flag = infection_flags::Entity::find()
        .filter(infection_flags::Column::Pid.eq(records::parse_pid(&flag_pid)?))
        .filter(infection_flags::Column::StayPid.eq(stay.pid))
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if flag.cleared_at.is_some() {
        return Err(unprocessable("flag is already cleared"));
    }
    let txn = ctx.db.begin().await?;
    let flag_pid = flag.pid;
    let mut active: infection_flags::ActiveModel = flag.into();
    active.status = ActiveValue::set("cleared".to_string());
    active.cleared_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "infection_flag",
        flag_pid,
        "infection_cleared",
        caller.actor(),
        stay.ward_pid
            .map(|w| serde_json::json!({ "ward_pid": w.to_string() })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "infection_flag",
        "infection_cleared",
        &flag_pid.to_string(),
        &stay.display_name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// The stay routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/stays", post(admit))
        .add("/stays/{pid}", get(get_stay))
        .add("/stays/{pid}/time-analysis", get(stay_time_analysis))
        .add("/stays/{pid}", put(update_stay))
        .add("/stays/{pid}/transfer", post(transfer))
        .add("/stays/{pid}/discharge-ready", post(discharge_ready))
        .add("/stays/{pid}/discharge", post(discharge))
        .add("/stays/{pid}/red-green", post(red_green))
        .add("/stays/{pid}/infection-flags", post(add_flag))
        .add(
            "/stays/{pid}/infection-flags/{flag_pid}/clear",
            post(clear_flag),
        )
}
