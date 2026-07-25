//! Assessments — aptitude, personality, psychometric, and selection
//! tests across the hiring pipeline and talent development (WPM-R20).
//!
//! Three record kinds: an **instrument** catalog (the named test, its
//! category, and the scales it reports), an **assessment** (one
//! administration of an instrument to one candidate or employee,
//! optionally tied to an application), and per-scale **results**.
//!
//! The pure rules live in [`crate::rules::assessment`]: the
//! category↔scale mapping (with the deliberate psychometric overlap),
//! the lifecycle machine, the score bounds, the band split, currency
//! (completed and unexpired), and the gap list. This module only wires
//! them, persists, audits, and emits.
//!
//! **Results are sensitive personal data.** They profile cognition and
//! behaviour, so — like salary and payslips — every read path honours
//! the ABAC `mask` obligation ([`mask_result`]: the scale and the band
//! survive; raw scores, percentiles, and narratives do not) and every
//! unmasked read of scored results is audited.
//!
//! The derived views report **real scores only**: a mean is `null`
//! rather than zero when nothing carries a percentile, and every
//! payload names its derivation.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{
    assessment_instruments, assessment_results, assessments, candidates,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::assessment as rules;
use crate::streaming;
use crate::validation::Problems;

/// A `{pid}` reference response.
#[derive(serde::Serialize)]
struct PidRef {
    pid: String,
}

impl PidRef {
    fn of(pid: Uuid) -> Self {
        Self {
            pid: pid.to_string(),
        }
    }
}

// ─── Instruments ────────────────────────────────────────────────────────────

/// `POST /api/assessment-instruments` body.
#[derive(Debug, Deserialize)]
struct InstrumentPayload {
    name: String,
    category: String,
    #[serde(default)]
    provider: Option<String>,
    /// The scales this instrument reports; each must suit the category.
    #[serde(default)]
    scales: Vec<String>,
    #[serde(default)]
    duration_minutes: Option<i32>,
    /// How long a sitting stays current, in months (drives `expires_on`
    /// when an assessment completes).
    #[serde(default)]
    validity_months: Option<i32>,
}

/// `POST /api/assessment-instruments` — add a test to the catalog.
///
/// Every declared scale must be permitted by the category
/// ([`rules::category_permits`]) — `psychometric` spans aptitude and
/// personality, everything else stays in its own lane.
#[debug_handler]
async fn create_instrument(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<InstrumentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_token("category", rules::ASSESSMENT_CATEGORIES, &payload.category);
    problems.cap_opt("provider", payload.provider.as_deref());
    problems.cap_list("scales", &payload.scales);
    for (index, scale) in payload.scales.iter().enumerate() {
        problems.require_token(&format!("scales[{index}]"), rules::ASSESSMENT_SCALES, scale);
        if rules::ASSESSMENT_SCALES.contains(&scale.as_str())
            && !rules::category_permits(&payload.category, scale)
        {
            problems.push(format!(
                "scales[{index}]: `{scale}` is not measured by a `{}` instrument",
                payload.category
            ));
        }
    }
    if payload.duration_minutes.is_some_and(|m| m <= 0) {
        problems.push("duration_minutes must be positive");
    }
    if payload.validity_months.is_some_and(|m| m <= 0) {
        problems.push("validity_months must be positive");
    }
    ensure_valid(&problems.into_vec())?;

    let row = assessment_instruments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        category: ActiveValue::set(payload.category.clone()),
        provider: ActiveValue::set(payload.provider.clone()),
        scales: ActiveValue::set(serde_json::json!(payload.scales)),
        duration_minutes: ActiveValue::set(payload.duration_minutes),
        validity_months: ActiveValue::set(payload.validity_months),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        "assessment_instrument",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    format::json(PidRef::of(row.pid))
}

/// Query for the instrument catalog.
#[derive(Debug, Deserialize)]
struct InstrumentQuery {
    category: Option<String>,
}

/// `GET /api/assessment-instruments?category=` — the catalog.
#[debug_handler]
async fn list_instruments(
    axum::extract::Query(query): axum::extract::Query<InstrumentQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    if let Some(category) = &query.category
        && !rules::ASSESSMENT_CATEGORIES.contains(&category.as_str())
    {
        return Err(unprocessable(&format!(
            "unknown category `{category}` (categories: {:?})",
            rules::ASSESSMENT_CATEGORIES
        )));
    }
    let mut find = assessment_instruments::Entity::find()
        .filter(assessment_instruments::Column::DeletedAt.is_null());
    if let Some(category) = &query.category {
        find = find.filter(assessment_instruments::Column::Category.eq(category.clone()));
    }
    let rows = find
        .order_by_asc(assessment_instruments::Column::Name)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

// ─── Assessments ────────────────────────────────────────────────────────────

/// `POST /api/assessments` body — schedule (or record) a sitting.
#[derive(Debug, Deserialize)]
struct AssessmentPayload {
    instrument_pid: Uuid,
    /// `candidate` or `employee`.
    subject_kind: String,
    /// The candidate's or employee's pid (existence is checked).
    subject_pid: Uuid,
    /// The application this sitting belongs to, for a hiring process.
    #[serde(default)]
    application_pid: Option<Uuid>,
    #[serde(default)]
    scheduled_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    administered_by: Option<String>,
    #[serde(default)]
    notes: Option<String>,
}

/// `POST /api/assessments` — schedule a sitting for a candidate or an
/// employee. The subject and (when given) the application must exist;
/// an application-linked sitting must belong to that application's
/// candidate, so a result can never be filed against the wrong hire.
#[debug_handler]
async fn create_assessment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AssessmentPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token(
        "subject_kind",
        rules::ASSESSMENT_SUBJECTS,
        &payload.subject_kind,
    );
    problems.cap_opt("administered_by", payload.administered_by.as_deref());
    problems.cap_opt("notes", payload.notes.as_deref());
    ensure_valid(&problems.into_vec())?;

    let instrument = records::find_assessment_instrument(&ctx.db, payload.instrument_pid).await?;
    let subject_name =
        subject_display_name(&ctx, &payload.subject_kind, payload.subject_pid, &caller).await?;

    if let Some(application_pid) = payload.application_pid {
        let application = records::find_application(&ctx.db, application_pid).await?;
        if payload.subject_kind != "candidate" {
            return Err(unprocessable(
                "only a candidate assessment can belong to an application",
            ));
        }
        if application.candidate_pid != payload.subject_pid {
            return Err(unprocessable(
                "the application belongs to a different candidate",
            ));
        }
    }

    let txn = ctx.db.begin().await?;
    let row = assessments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        instrument_pid: ActiveValue::set(instrument.pid),
        subject_kind: ActiveValue::set(payload.subject_kind.clone()),
        subject_pid: ActiveValue::set(payload.subject_pid),
        application_pid: ActiveValue::set(payload.application_pid),
        status: ActiveValue::set("scheduled".to_string()),
        scheduled_on: ActiveValue::set(payload.scheduled_on),
        completed_on: ActiveValue::set(None),
        expires_on: ActiveValue::set(None),
        administered_by: ActiveValue::set(payload.administered_by.clone()),
        notes: ActiveValue::set(payload.notes.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "assessment",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "instrument": instrument.name,
            "category": instrument.category,
            "subject_kind": payload.subject_kind,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "assessment",
        "created",
        &row.pid.to_string(),
        &instrument.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    tracing::debug!(subject = %subject_name, "assessment scheduled");
    format::json(PidRef::of(row.pid))
}

/// `POST /api/assessments/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct StatusPayload {
    to: String,
}

/// `POST /api/assessments/{pid}/status` — the lifecycle move (the pure
/// machine refuses an illegal one). Completing stamps `completed_on`
/// and, when the instrument declares a `validity_months`, derives
/// `expires_on` from it — so currency is a recorded fact, not an
/// assumption at read time.
///
/// Completion requires at least one recorded result: "completed" must
/// not assert a scoring that never happened.
#[debug_handler]
async fn assessment_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let row = records::find_assessment(&ctx.db, records::parse_pid(&pid)?).await?;
    rules::assessment_transition(&row.status, &payload.to)
        .map_err(|reason| unprocessable(&reason))?;

    let today = chrono::Utc::now().date_naive();
    let instrument = records::find_assessment_instrument(&ctx.db, row.instrument_pid).await?;
    let from = row.status.clone();
    let row_pid = row.pid;

    if payload.to == "completed" {
        let scored = assessment_results::Entity::find()
            .filter(assessment_results::Column::AssessmentPid.eq(row_pid))
            .all(&ctx.db)
            .await?;
        if scored.is_empty() {
            return Err(unprocessable(
                "record at least one result before completing the assessment",
            ));
        }
    }

    let txn = ctx.db.begin().await?;
    let mut active: assessments::ActiveModel = row.into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "completed" {
        active.completed_on = ActiveValue::set(Some(today));
        if let Some(months) = instrument.validity_months {
            active.expires_on = ActiveValue::set(expiry_from(today, months));
        }
    }
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "assessment",
        row_pid,
        "status_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "assessment",
        &format!("assessment_{}", payload.to),
        &row_pid.to_string(),
        &instrument.name,
        caller.actor(),
        Some(serde_json::json!({ "from": from })),
    )
    .await?;
    txn.commit().await?;
    format::json(updated)
}

/// The expiry date `months` after `from`, or `None` when the arithmetic
/// would overflow the calendar (never panics — security invariant 2).
fn expiry_from(from: chrono::NaiveDate, months: i32) -> Option<chrono::NaiveDate> {
    u32::try_from(months)
        .ok()
        .and_then(|m| from.checked_add_months(chrono::Months::new(m)))
}

/// `POST /api/assessments/{pid}/results` body — record (or replace) one
/// scale's outcome.
#[derive(Debug, Deserialize)]
struct ResultPayload {
    scale: String,
    #[serde(default)]
    raw_score: Option<i32>,
    #[serde(default)]
    max_score: Option<i32>,
    /// Norm-referenced percentile, 0–100. The band is derived from it
    /// unless one is given explicitly.
    #[serde(default)]
    percentile: Option<i32>,
    #[serde(default)]
    band: Option<String>,
    #[serde(default)]
    narrative: Option<String>,
}

/// `POST /api/assessments/{pid}/results` — record a scale's outcome
/// (upsert: one row per assessment+scale).
///
/// The scale must be permitted by the **instrument's** category, and —
/// when the instrument declares its scales — must be one it reports.
/// Scores are bounded by the pure rules; the band is derived from the
/// percentile when not given.
#[debug_handler]
async fn record_result(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ResultPayload>,
) -> Result<Response> {
    let assessment = records::find_assessment(&ctx.db, records::parse_pid(&pid)?).await?;
    let instrument =
        records::find_assessment_instrument(&ctx.db, assessment.instrument_pid).await?;

    let mut problems = Problems::new();
    problems.require_token("scale", rules::ASSESSMENT_SCALES, &payload.scale);
    problems.token_opt("band", rules::SCORE_BANDS, payload.band.as_deref());
    problems.cap_opt("narrative", payload.narrative.as_deref());
    if rules::ASSESSMENT_SCALES.contains(&payload.scale.as_str())
        && !rules::category_permits(&instrument.category, &payload.scale)
    {
        problems.push(format!(
            "`{}` is not measured by a `{}` assessment",
            payload.scale, instrument.category
        ));
    }
    if let Some(declared) = declared_scales(&instrument)
        && !declared.iter().any(|s| s == &payload.scale)
    {
        problems.push(format!(
            "`{}` is not one of the scales `{}` reports",
            payload.scale, instrument.name
        ));
    }
    if payload
        .percentile
        .is_some_and(|p| !rules::valid_percentile(p))
    {
        problems.push("percentile must be between 0 and 100");
    }
    match (payload.raw_score, payload.max_score) {
        (Some(raw), Some(max)) if !rules::valid_raw_score(raw, max) => {
            problems.push("raw_score must be between 0 and max_score, and max_score positive");
        }
        (Some(_), None) => problems.push("raw_score requires max_score"),
        _ => {}
    }
    if assessment.status == "cancelled" || assessment.status == "expired" {
        problems.push(format!(
            "cannot record a result on a `{}` assessment",
            assessment.status
        ));
    }
    ensure_valid(&problems.into_vec())?;

    let band = payload.band.clone().or_else(|| {
        payload
            .percentile
            .map(|p| rules::band_for_percentile(p).to_string())
    });

    let existing = assessment_results::Entity::find()
        .filter(assessment_results::Column::AssessmentPid.eq(assessment.pid))
        .filter(assessment_results::Column::Scale.eq(payload.scale.clone()))
        .one(&ctx.db)
        .await?;
    let row = match existing {
        Some(found) => {
            let mut active: assessment_results::ActiveModel = found.into();
            active.raw_score = ActiveValue::set(payload.raw_score);
            active.max_score = ActiveValue::set(payload.max_score);
            active.percentile = ActiveValue::set(payload.percentile);
            active.band = ActiveValue::set(band);
            active.narrative = ActiveValue::set(payload.narrative.clone());
            active.update(&ctx.db).await?
        }
        None => {
            assessment_results::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                assessment_pid: ActiveValue::set(assessment.pid),
                scale: ActiveValue::set(payload.scale.clone()),
                raw_score: ActiveValue::set(payload.raw_score),
                max_score: ActiveValue::set(payload.max_score),
                percentile: ActiveValue::set(payload.percentile),
                band: ActiveValue::set(band),
                narrative: ActiveValue::set(payload.narrative.clone()),
                ..Default::default()
            }
            .insert(&ctx.db)
            .await?
        }
    };
    // The scale is recorded; the score itself never enters the trail.
    Audit::record(
        &ctx.db,
        "assessment_result",
        row.pid,
        "recorded",
        caller.actor(),
        Some(serde_json::json!({ "assessment": assessment.pid, "scale": payload.scale })),
    )
    .await?;
    format::json(PidRef::of(row.pid))
}

/// `GET /api/assessments/{pid}` — one assessment with its instrument
/// and its results.
///
/// Results are sensitive: the `mask` obligation redacts the scores
/// ([`mask_result`]), and an **unmasked** read of scored results is
/// audited.
#[debug_handler]
async fn get_assessment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let assessment = records::find_assessment(&ctx.db, records::parse_pid(&pid)?).await?;
    let instrument =
        records::find_assessment_instrument(&ctx.db, assessment.instrument_pid).await?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &assessment_resource_attrs(&assessment, &instrument),
    )
    .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");

    let rows = assessment_results::Entity::find()
        .filter(assessment_results::Column::AssessmentPid.eq(assessment.pid))
        .order_by_asc(assessment_results::Column::Scale)
        .all(&ctx.db)
        .await?;
    if !masked && !rows.is_empty() {
        Audit::record(
            &ctx.db,
            "assessment",
            assessment.pid,
            "results_read",
            caller.actor(),
            Some(serde_json::json!({ "category": instrument.category, "scales": rows.len() })),
        )
        .await?;
    }
    let results: Vec<assessment_results::Model> = if masked {
        rows.into_iter().map(mask_result).collect()
    } else {
        rows
    };
    format::json(serde_json::json!({
        "assessment": assessment,
        "instrument": {
            "pid": instrument.pid, "name": instrument.name,
            "category": instrument.category, "provider": instrument.provider,
        },
        "masked": masked,
        "results": results,
    }))
}

/// `DELETE /api/assessments/{pid}` — withdraw a sitting (soft delete),
/// keeping the audit trail intact.
#[debug_handler]
async fn delete_assessment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let assessment = records::find_assessment(&ctx.db, records::parse_pid(&pid)?).await?;
    let instrument =
        records::find_assessment_instrument(&ctx.db, assessment.instrument_pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Delete,
        &assessment_resource_attrs(&assessment, &instrument),
    )
    .map_err(record_rejection)?;

    let row_pid = assessment.pid;
    let txn = ctx.db.begin().await?;
    let mut active: assessments::ActiveModel = assessment.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(&txn, "assessment", row_pid, "deleted", caller.actor(), None).await?;
    streaming::emit_on(
        &txn,
        "assessment",
        "deleted",
        &row_pid.to_string(),
        &instrument.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": row_pid, "deleted": true }))
}

// ─── Derived views ──────────────────────────────────────────────────────────

/// Query for the subject profile.
#[derive(Debug, Deserialize)]
struct ProfileQuery {
    /// Judge currency as of this date (default today).
    as_of: Option<chrono::NaiveDate>,
}

/// `GET /api/candidates/{pid}/assessment-profile` — a candidate's
/// profile (see [`subject_profile`]).
#[debug_handler]
async fn candidate_profile(
    axum::extract::Query(query): axum::extract::Query<ProfileQuery>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let candidate = records::find_candidate(&ctx.db, records::parse_pid(&pid)?).await?;
    subject_profile(&ctx, &caller, "candidate", candidate.pid, query.as_of).await
}

/// `GET /api/employees/{pid}/assessment-profile` — an employee's
/// profile. Authorized (and masked) at the **employee** level, so an
/// assessment profile is never a way around the employee's own
/// record-level policy.
#[debug_handler]
async fn employee_profile(
    axum::extract::Query(query): axum::extract::Query<ProfileQuery>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");
    build_profile(&ctx, "employee", employee.pid, query.as_of, masked).await
}

/// Shared body of the candidate profile: authorize coarsely (there is
/// no per-candidate resource shape), then build.
async fn subject_profile(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    subject_kind: &str,
    subject_pid: Uuid,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Response> {
    let obligations = auth::authorize_record(
        caller,
        authentication_verifier::Action::Read,
        &BTreeMap::from([
            ("subject_kind".to_string(), vec![subject_kind.to_string()]),
            ("record".to_string(), vec!["assessment".to_string()]),
        ]),
    )
    .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");
    build_profile(ctx, subject_kind, subject_pid, as_of, masked).await
}

/// Build one subject's assessment profile: per category, the sittings
/// recorded and current, the **current reading per scale** (the most
/// recently completed current assessment reporting it), the scales with
/// no current reading, and — for selection — the mean percentile with
/// its numerator and denominator.
///
/// Under the `mask` obligation the bands survive and every score
/// (including the mean) is withheld: an aggregate read must never
/// reveal more than the equivalent single read.
async fn build_profile(
    ctx: &AppContext,
    subject_kind: &str,
    subject_pid: Uuid,
    as_of: Option<chrono::NaiveDate>,
    masked: bool,
) -> Result<Response> {
    let as_of = as_of.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let sittings = assessments::Entity::find()
        .filter(assessments::Column::SubjectKind.eq(subject_kind.to_string()))
        .filter(assessments::Column::SubjectPid.eq(subject_pid))
        .filter(assessments::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let instruments = assessment_instruments::Entity::find().all(&ctx.db).await?;
    let instrument_of: BTreeMap<Uuid, &assessment_instruments::Model> =
        instruments.iter().map(|i| (i.pid, i)).collect();

    let mut categories = Vec::with_capacity(rules::ASSESSMENT_CATEGORIES.len());
    let mut selection_percentiles: Vec<i32> = Vec::new();

    for category in rules::ASSESSMENT_CATEGORIES {
        let in_category: Vec<&assessments::Model> = sittings
            .iter()
            .filter(|a| {
                instrument_of
                    .get(&a.instrument_pid)
                    .is_some_and(|i| i.category == *category)
            })
            .collect();
        let current: Vec<&&assessments::Model> = in_category
            .iter()
            .filter(|a| rules::is_current(&a.status, a.expires_on, as_of))
            .collect();

        // Latest current reading per scale; ties on the completion date
        // break on the later row id, so the reading is deterministic.
        let mut latest: BTreeMap<String, (&assessments::Model, assessment_results::Model)> =
            BTreeMap::new();
        for sitting in &current {
            let rows = assessment_results::Entity::find()
                .filter(assessment_results::Column::AssessmentPid.eq(sitting.pid))
                .all(&ctx.db)
                .await?;
            for row in rows {
                let replace = latest.get(&row.scale).is_none_or(|(held, _)| {
                    (sitting.completed_on, sitting.id) > (held.completed_on, held.id)
                });
                if replace {
                    latest.insert(row.scale.clone(), (sitting, row));
                }
            }
        }
        if *category == "selection" {
            selection_percentiles.extend(latest.values().filter_map(|(_, r)| r.percentile));
        }

        let scales: Vec<serde_json::Value> = latest
            .iter()
            .map(|(scale, (sitting, result))| {
                serde_json::json!({
                    "scale": scale,
                    "band": result.band,
                    "percentile": if masked { None } else { result.percentile },
                    "raw_score": if masked { None } else { result.raw_score },
                    "max_score": if masked { None } else { result.max_score },
                    "instrument": instrument_of.get(&sitting.instrument_pid).map(|i| i.name.clone()),
                    "completed_on": sitting.completed_on,
                    "assessment_pid": sitting.pid,
                })
            })
            .collect();
        let measured: Vec<String> = latest.keys().cloned().collect();

        categories.push(serde_json::json!({
            "category": category,
            "recorded": in_category.len(),
            "current": current.len(),
            "scales": scales,
            "scales_not_assessed": rules::scales_not_assessed(category, &measured),
        }));
    }

    let suitability = if masked {
        serde_json::Value::Null
    } else {
        rules::mean_percentile(&selection_percentiles).map_or(serde_json::Value::Null, |(sum, count, mean)| {
            serde_json::json!({ "numerator": sum, "denominator": count, "value": mean })
        })
    };

    format::json(serde_json::json!({
        "as_of": as_of,
        "subject": { "kind": subject_kind, "pid": subject_pid },
        "masked": masked,
        "derivation": "a scale's current reading = the most recently completed, unexpired \
                       assessment reporting it; selection suitability = the mean percentile of \
                       those current selection readings, real scores only (null when none carry \
                       a percentile)",
        "categories": categories,
        "selection_suitability": suitability,
    }))
}

/// `GET /api/applications/{pid}/assessments` — every sitting recorded
/// for one application's candidate, with the per-scale bands.
///
/// The hiring view: what the candidate has been asked to do, what is
/// outstanding, and how the completed sittings read. Scores obey the
/// `mask` obligation exactly as elsewhere.
#[debug_handler]
async fn application_assessments(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let application = records::find_application(&ctx.db, records::parse_pid(&pid)?).await?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &BTreeMap::from([
            ("record".to_string(), vec!["assessment".to_string()]),
            ("stage".to_string(), vec![application.stage.clone()]),
        ]),
    )
    .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");

    let sittings = assessments::Entity::find()
        .filter(assessments::Column::ApplicationPid.eq(application.pid))
        .filter(assessments::Column::DeletedAt.is_null())
        .order_by_asc(assessments::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut view = Vec::with_capacity(sittings.len());
    for sitting in &sittings {
        let instrument =
            records::find_assessment_instrument(&ctx.db, sitting.instrument_pid).await?;
        let rows = assessment_results::Entity::find()
            .filter(assessment_results::Column::AssessmentPid.eq(sitting.pid))
            .order_by_asc(assessment_results::Column::Scale)
            .all(&ctx.db)
            .await?;
        let results: Vec<assessment_results::Model> = if masked {
            rows.into_iter().map(mask_result).collect()
        } else {
            rows
        };
        view.push(serde_json::json!({
            "pid": sitting.pid,
            "instrument": instrument.name,
            "category": instrument.category,
            "status": sitting.status,
            "scheduled_on": sitting.scheduled_on,
            "completed_on": sitting.completed_on,
            "results": results,
        }));
    }
    let outstanding = sittings
        .iter()
        .filter(|s| matches!(s.status.as_str(), "scheduled" | "in_progress"))
        .count();
    format::json(serde_json::json!({
        "application": { "pid": application.pid, "stage": application.stage },
        "candidate_pid": application.candidate_pid,
        "masked": masked,
        "outstanding": outstanding,
        "assessments": view,
    }))
}

/// `GET /api/assessments/analytics` — programme-level rollups: sittings
/// by category and status, completion counts, and the band distribution
/// per scale.
///
/// Aggregate counts only — no individual's score appears here, so it is
/// safe to serve without the per-record mask (a band distribution over
/// the whole population identifies no one).
#[debug_handler]
async fn analytics(State(ctx): State<AppContext>) -> Result<Response> {
    let sittings = assessments::Entity::find()
        .filter(assessments::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let instruments = assessment_instruments::Entity::find().all(&ctx.db).await?;
    let category_of: BTreeMap<Uuid, &str> = instruments
        .iter()
        .map(|i| (i.pid, i.category.as_str()))
        .collect();

    // category → status → count
    let mut by_category: BTreeMap<&str, BTreeMap<String, usize>> = BTreeMap::new();
    for sitting in &sittings {
        let Some(category) = category_of.get(&sitting.instrument_pid) else {
            continue;
        };
        *by_category
            .entry(category)
            .or_default()
            .entry(sitting.status.clone())
            .or_default() += 1;
    }

    // scale → band → count, over every recorded result.
    let results = assessment_results::Entity::find().all(&ctx.db).await?;
    let mut bands: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for row in &results {
        if let Some(band) = &row.band {
            *bands
                .entry(row.scale.clone())
                .or_default()
                .entry(band.clone())
                .or_default() += 1;
        }
    }

    let categories: Vec<serde_json::Value> = by_category
        .iter()
        .map(|(category, by_status)| {
            let completed = by_status.get("completed").copied().unwrap_or(0);
            let total: usize = by_status.values().sum();
            serde_json::json!({
                "category": category,
                "by_status": by_status,
                "completed": completed,
                "total": total,
            })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "counts over live sittings; the band distribution is aggregate only — \
                 no individual score is reported here",
        "categories": categories,
        "band_distribution": bands,
    }))
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Apply the `mask` obligation to a result: the scale and the
/// interpreted band survive; the raw score, its maximum, the
/// percentile, and the narrative are redacted. A masked caller learns
/// *that* a dimension was measured and roughly where it landed, never
/// the profile itself.
#[must_use]
fn mask_result(mut result: assessment_results::Model) -> assessment_results::Model {
    result.raw_score = None;
    result.max_score = None;
    result.percentile = None;
    result.narrative = None;
    result
}

/// Resource attributes for an assessment, for the `resource.*` policy
/// namespace: the instrument's category, the sitting's status, and the
/// subject kind. A deployment can then write e.g. "allow an unmasked
/// read of `resource.category=selection` only to recruiters, and a
/// masked read otherwise" — entirely as policy, no code change.
#[must_use]
fn assessment_resource_attrs(
    assessment: &assessments::Model,
    instrument: &assessment_instruments::Model,
) -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([
        ("record".to_string(), vec!["assessment".to_string()]),
        ("category".to_string(), vec![instrument.category.clone()]),
        ("status".to_string(), vec![assessment.status.clone()]),
        (
            "subject_kind".to_string(),
            vec![assessment.subject_kind.clone()],
        ),
    ])
}

/// The scales an instrument declares, when it declares any (an empty
/// or absent list means "any scale its category permits").
fn declared_scales(instrument: &assessment_instruments::Model) -> Option<Vec<String>> {
    let declared: Vec<String> =
        serde_json::from_value(instrument.scales.clone()).unwrap_or_default();
    if declared.is_empty() {
        None
    } else {
        Some(declared)
    }
}

/// Resolve the subject (candidate or employee) and return its display
/// name, `404`-ing when it does not exist — so a sitting can never be
/// booked against a subject that is not there. An employee subject is
/// additionally authorized at the employee level.
async fn subject_display_name(
    ctx: &AppContext,
    subject_kind: &str,
    subject_pid: Uuid,
    caller: &MaybeAuthUser,
) -> Result<String> {
    if subject_kind == "employee" {
        let employee = records::find_employee(&ctx.db, subject_pid).await?;
        auth::authorize_record(
            caller,
            authentication_verifier::Action::Write,
            &auth::employee_resource_attrs(&employee),
        )
        .map_err(record_rejection)?;
        return Ok(employee.display_name);
    }
    let candidate = candidates::Entity::find()
        .filter(candidates::Column::Pid.eq(subject_pid))
        .filter(candidates::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(candidate.display_name)
}

/// The assessment routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/assessment-instruments", post(create_instrument))
        .add("/assessment-instruments", get(list_instruments))
        .add("/assessments", post(create_assessment))
        .add("/assessments/analytics", get(analytics))
        .add("/assessments/{pid}", get(get_assessment))
        .add("/assessments/{pid}", delete(delete_assessment))
        .add("/assessments/{pid}/status", post(assessment_status))
        .add("/assessments/{pid}/results", post(record_result))
        .add(
            "/applications/{pid}/assessments",
            get(application_assessments),
        )
        .add(
            "/candidates/{pid}/assessment-profile",
            get(candidate_profile),
        )
        .add("/employees/{pid}/assessment-profile", get(employee_profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored result, as the DB would return it.
    fn a_result(scale: &str, percentile: Option<i32>) -> assessment_results::Model {
        assessment_results::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: Uuid::new_v4(),
            assessment_pid: Uuid::new_v4(),
            scale: scale.to_string(),
            raw_score: Some(41),
            max_score: Some(50),
            percentile,
            band: percentile.map(|p| rules::band_for_percentile(p).to_string()),
            narrative: Some("strong under time pressure".to_string()),
        }
    }

    /// An instrument row declaring `scales`.
    fn an_instrument(category: &str, scales: &[&str]) -> assessment_instruments::Model {
        assessment_instruments::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: Uuid::new_v4(),
            name: "Test".to_string(),
            category: category.to_string(),
            provider: None,
            scales: serde_json::json!(scales),
            duration_minutes: None,
            validity_months: None,
            deleted_at: None,
        }
    }

    /// Masking keeps the scale and band, drops every score and the
    /// narrative (the sensitive-read invariant).
    #[test]
    fn mask_keeps_the_band_and_drops_the_profile() {
        let masked = mask_result(a_result("judgement_test", Some(88)));
        assert_eq!(masked.scale, "judgement_test");
        assert_eq!(masked.band.as_deref(), Some("above_average"));
        assert!(masked.raw_score.is_none() && masked.max_score.is_none());
        assert!(masked.percentile.is_none(), "the percentile is redacted");
        assert!(masked.narrative.is_none(), "the narrative is redacted");
    }

    /// An instrument's declared scales gate which results it accepts;
    /// an empty declaration means "any its category permits".
    #[test]
    fn declared_scales_gate_results() {
        let declared = an_instrument("aptitude", &["numerical_reasoning"]);
        let scales = declared_scales(&declared).expect("declared");
        assert_eq!(scales, vec!["numerical_reasoning".to_string()]);
        assert!(!scales.iter().any(|s| s == "verbal_reasoning"));

        let open = an_instrument("aptitude", &[]);
        assert!(
            declared_scales(&open).is_none(),
            "no declaration ⇒ any permitted scale"
        );
    }

    /// The resource attributes a policy can key on.
    #[test]
    fn resource_attrs_expose_category_status_and_subject() {
        let instrument = an_instrument("selection", &["job_simulation"]);
        let sitting = assessments::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: Uuid::new_v4(),
            instrument_pid: instrument.pid,
            subject_kind: "candidate".to_string(),
            subject_pid: Uuid::new_v4(),
            application_pid: None,
            status: "completed".to_string(),
            scheduled_on: None,
            completed_on: None,
            expires_on: None,
            administered_by: None,
            notes: None,
            deleted_at: None,
        };
        let attrs = assessment_resource_attrs(&sitting, &instrument);
        assert_eq!(attrs["category"], vec!["selection".to_string()]);
        assert_eq!(attrs["status"], vec!["completed".to_string()]);
        assert_eq!(attrs["subject_kind"], vec!["candidate".to_string()]);
    }

    /// Expiry arithmetic adds the months and refuses to panic on a
    /// negative or absurd validity.
    #[test]
    fn expiry_is_derived_and_never_panics() {
        let day = |y, m, d| chrono::NaiveDate::from_ymd_opt(y, m, d).expect("valid date");
        assert_eq!(expiry_from(day(2026, 7, 23), 12), Some(day(2027, 7, 23)));
        assert_eq!(expiry_from(day(2026, 1, 31), 1), Some(day(2026, 2, 28)));
        assert_eq!(expiry_from(day(2026, 7, 23), -1), None, "negative months");
        assert_eq!(
            expiry_from(day(2026, 7, 23), i32::MAX),
            None,
            "overflowing the calendar yields None, not a panic"
        );
    }
}
