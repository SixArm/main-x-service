//! Workforce-assessment endpoints — record and read the aptitude,
//! personality, psychometric, and selection tests a worker has taken
//! (domain model: [`crate::models::assessment`]; persistence:
//! [`crate::db::assessments`]).
//!
//! The surface is a sub-resource of a worker, so every request first
//! loads the worker (`404` when unknown) and authorises at the
//! **worker** level via the record-level ABAC guard
//! ([`authorize_record`]) — reading a worker's psychometric profile is
//! gated exactly like reading the worker, and recording one like
//! writing the worker. All of it is a no-op when `WORKER_REQUIRE_AUTH`
//! is off (the family's default-off posture,
//! `agents/share/security.md` §4).
//!
//! **Assessment results are sensitive personal data.** They profile a
//! person's cognition and behaviour, so:
//!
//! - reads honour the ABAC **`mask` obligation** — a masked caller gets
//!   [`Assessment::masked`] (bands survive; raw scores, percentiles,
//!   narratives, and operator notes do not), on **every** read path:
//!   the single fetch, the list, and the derived profile (security
//!   invariant 5 — a bulk read never reveals more than a single one);
//! - every read of the unmasked detail and every mutation writes an
//!   audit row.
//!
//! Mutations are validated with
//! [`validate_assessment`](crate::validation::validate_assessment)
//! (`422` on failure) and status changes go through the lifecycle
//! machine ([`AssessmentStatus::can_transition_to`]), so an assessment
//! cannot be walked backwards or resurrected from a terminal state.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use authentication_verifier::Action;

use super::auth::{MaybeAuthUser, authorize_record, worker_resource_attrs};
use super::state::AppState;
use crate::db::AuditContext;
use crate::db::assessments::{self, AssessmentUpdate};
use crate::db::audit::AuditActor;
use crate::models::Worker;
use crate::models::assessment::{
    Assessment, AssessmentCategory, AssessmentResult, AssessmentScale, AssessmentStatus, ScoreBand,
};
use crate::validation::validate_assessment;

/// Body of `POST /api/workers/{id}/assessments` — record an assessment
/// administration. `category` and `status` are the domain wire tokens;
/// an unknown token is a `422` rather than a silent default.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateAssessmentRequest {
    /// `aptitude` | `personality` | `psychometric` | `selection`.
    pub category: String,
    /// The instrument's name.
    pub instrument: String,
    /// The test publisher / administering provider.
    #[serde(default)]
    pub provider: Option<String>,
    /// Lifecycle status; defaults to `scheduled`.
    #[serde(default)]
    pub status: Option<String>,
    /// The date the assessment was taken.
    #[serde(default)]
    pub administered_on: Option<NaiveDate>,
    /// The date the results stop counting as current.
    #[serde(default)]
    pub expires_on: Option<NaiveDate>,
    /// Who administered it.
    #[serde(default)]
    pub administered_by: Option<String>,
    /// Operator notes about the administration.
    #[serde(default)]
    pub notes: Option<String>,
    /// The per-scale outcomes.
    #[serde(default)]
    pub results: Vec<AssessmentResult>,
}

/// Deserialize a present field into `Some(value)` — including a present
/// `null`, which becomes `Some(None)`.
///
/// Serde's default handling of `Option<Option<T>>` collapses an explicit
/// `null` to `None`, which would make "clear this field" indexing
/// indistinguishable from "leave it alone". Routing the field through
/// this helper keeps the two apart: absent ⇒ `None` (from
/// `#[serde(default)]`), `null` ⇒ `Some(None)` (clear), a value ⇒
/// `Some(Some(v))` (set).
fn deserialize_present<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Body of `PUT /api/workers/{id}/assessments/{assessment_id}`. Every
/// field is optional: an absent field is left untouched. `provider`,
/// `notes`, `administered_by`, and the dates use a double `Option` so an
/// explicit `null` clears the stored value while omission preserves it.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateAssessmentRequest {
    /// New lifecycle status token (checked against the transition machine).
    #[serde(default)]
    pub status: Option<String>,
    /// New instrument name.
    #[serde(default)]
    pub instrument: Option<String>,
    /// Set (or `null` to clear) the provider.
    #[serde(default, deserialize_with = "deserialize_present")]
    pub provider: Option<Option<String>>,
    /// Set (or `null` to clear) the administration date.
    #[serde(default, deserialize_with = "deserialize_present")]
    pub administered_on: Option<Option<NaiveDate>>,
    /// Set (or `null` to clear) the expiry date.
    #[serde(default, deserialize_with = "deserialize_present")]
    pub expires_on: Option<Option<NaiveDate>>,
    /// Set (or `null` to clear) the administering identity.
    #[serde(default, deserialize_with = "deserialize_present")]
    pub administered_by: Option<Option<String>>,
    /// Set (or `null` to clear) the operator notes.
    #[serde(default, deserialize_with = "deserialize_present")]
    pub notes: Option<Option<String>>,
    /// Replace the per-scale results wholesale.
    #[serde(default)]
    pub results: Option<Vec<AssessmentResult>>,
}

/// Query parameters for the assessment list endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct ListAssessmentsQuery {
    /// Filter to one category wire token.
    #[serde(default)]
    pub category: Option<String>,
    /// Filter to one status wire token.
    #[serde(default)]
    pub status: Option<String>,
    /// Keep only assessments whose results are current on this date
    /// (completed and not past their expiry).
    #[serde(default)]
    pub valid_on: Option<NaiveDate>,
}

/// Query parameters for the profile endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct ProfileQuery {
    /// The date validity is judged against; defaults to today (UTC).
    #[serde(default)]
    pub as_of: Option<NaiveDate>,
}

/// One scale's current reading in the derived profile.
#[derive(Debug, Serialize, PartialEq)]
pub struct ProfileScale {
    /// The measured dimension.
    pub scale: AssessmentScale,
    /// The interpreted band, when the current result carries one.
    pub band: Option<ScoreBand>,
    /// The percentile — redacted (`None`) on a masked read.
    pub percentile: Option<f64>,
    /// The instrument the current reading came from.
    pub instrument: String,
    /// When it was administered.
    pub administered_on: Option<NaiveDate>,
    /// The assessment the reading came from.
    pub assessment_id: Uuid,
}

/// One category's slice of the derived profile.
#[derive(Debug, Serialize, PartialEq)]
pub struct ProfileCategory {
    /// The category.
    pub category: AssessmentCategory,
    /// Live (not withdrawn) assessments recorded in this category.
    pub recorded: usize,
    /// How many of those are current as of the profile date.
    pub valid: usize,
    /// The current reading per scale, most recent administration wins.
    pub scales: Vec<ProfileScale>,
    /// Scales this category measures for which there is no current
    /// reading — the honest statement of what has **not** been assessed.
    pub scales_not_assessed: Vec<AssessmentScale>,
}

/// `GET /api/workers/{id}/assessment-profile` — the derived, at-a-glance
/// view across all four categories.
#[derive(Debug, Serialize, PartialEq)]
pub struct AssessmentProfile {
    /// The worker profiled.
    pub worker_id: Uuid,
    /// The date validity was judged against.
    pub as_of: NaiveDate,
    /// Whether the response was redacted under the `mask` obligation.
    pub masked: bool,
    /// One entry per category, in the domain's declaration order.
    pub categories: Vec<ProfileCategory>,
    /// Mean percentile across **current selection** assessments — the
    /// headline suitability figure for a hiring decision. `None` when no
    /// current selection assessment carries a percentile; never
    /// interpolated from bands, and always `None` on a masked read.
    pub selection_suitability: Option<f64>,
    /// How the view was derived, stated in the payload so a consumer
    /// cannot mistake it for something richer.
    pub derivation: &'static str,
}

/// How [`build_profile`] derives its numbers, echoed in the response.
const DERIVATION: &str = "current reading per scale = the most recently administered \
     completed, unexpired assessment reporting it; selection suitability = the mean \
     percentile of current selection results, real scores only";

/// Build the derived profile from a worker's live assessments.
///
/// Pure (no I/O, no clock — `as_of` is supplied) so the derivation is
/// unit-tested without a database. For each category it reports how many
/// assessments are recorded and how many are current, the **current
/// reading per scale** (the most recently administered current
/// assessment that reports that scale), and which of the category's own
/// scales have no current reading at all.
///
/// When `masked` is set, percentiles and the suitability figure are
/// withheld — the bands still describe the shape of the profile without
/// disclosing the scores.
#[must_use]
pub fn build_profile(
    worker_id: Uuid,
    assessments: &[Assessment],
    as_of: NaiveDate,
    masked: bool,
) -> AssessmentProfile {
    let mut categories = Vec::with_capacity(AssessmentCategory::ALL.len());

    for category in AssessmentCategory::ALL {
        let in_category: Vec<&Assessment> = assessments
            .iter()
            .filter(|a| a.category == category)
            .collect();
        let current: Vec<&&Assessment> = in_category
            .iter()
            .filter(|a| a.is_valid_on(as_of))
            .collect();

        // Latest current reading per scale. Ties on the administration
        // date (or a missing date) fall back to the later `updated_at`,
        // so the reading is deterministic rather than iteration-ordered.
        let mut latest: BTreeMap<AssessmentScale, (&Assessment, &AssessmentResult)> =
            BTreeMap::new();
        for assessment in &current {
            for result in &assessment.results {
                let entry = latest.entry(result.scale);
                match entry {
                    std::collections::btree_map::Entry::Vacant(slot) => {
                        slot.insert((assessment, result));
                    }
                    std::collections::btree_map::Entry::Occupied(mut slot) => {
                        let (held, _) = slot.get();
                        if is_more_recent(assessment, held) {
                            slot.insert((assessment, result));
                        }
                    }
                }
            }
        }

        let scales: Vec<ProfileScale> = latest
            .iter()
            .map(|(scale, (assessment, result))| ProfileScale {
                scale: *scale,
                band: result.effective_band(),
                percentile: if masked { None } else { result.percentile },
                instrument: assessment.instrument.clone(),
                administered_on: assessment.administered_on,
                assessment_id: assessment.id,
            })
            .collect();

        let scales_not_assessed: Vec<AssessmentScale> = category
            .own_scales()
            .iter()
            .copied()
            .filter(|scale| !latest.contains_key(scale))
            .collect();

        categories.push(ProfileCategory {
            category,
            recorded: in_category.len(),
            valid: current.len(),
            scales,
            scales_not_assessed,
        });
    }

    let selection_suitability = if masked {
        None
    } else {
        mean_selection_percentile(assessments, as_of)
    };

    AssessmentProfile {
        worker_id,
        as_of,
        masked,
        categories,
        selection_suitability,
        derivation: DERIVATION,
    }
}

/// Whether `candidate` is a more recent administration than `held`:
/// later administration date wins; a dated assessment beats an undated
/// one; ties break on the later `updated_at`.
fn is_more_recent(candidate: &Assessment, held: &Assessment) -> bool {
    match (candidate.administered_on, held.administered_on) {
        (Some(a), Some(b)) if a != b => a > b,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        _ => candidate.updated_at > held.updated_at,
    }
}

/// Mean percentile across the results of every **current selection**
/// assessment. `None` when none of them carries a percentile — the
/// figure is never interpolated from bands.
fn mean_selection_percentile(assessments: &[Assessment], as_of: NaiveDate) -> Option<f64> {
    let scored: Vec<f64> = assessments
        .iter()
        .filter(|a| a.category == AssessmentCategory::Selection && a.is_valid_on(as_of))
        .flat_map(|a| a.results.iter().filter_map(|r| r.percentile))
        .collect();
    if scored.is_empty() {
        return None;
    }
    #[allow(clippy::cast_precision_loss)] // result counts are tiny
    let n = scored.len() as f64;
    Some(scored.iter().sum::<f64>() / n)
}

/// Apply the list filters to a worker's assessments. Pure, so the
/// filter semantics are unit-tested without a database. An unparsable
/// filter token never reaches here — the handler rejects it with `422`.
#[must_use]
pub fn apply_filters(
    assessments: Vec<Assessment>,
    category: Option<AssessmentCategory>,
    status: Option<AssessmentStatus>,
    valid_on: Option<NaiveDate>,
) -> Vec<Assessment> {
    assessments
        .into_iter()
        .filter(|a| category.is_none_or(|c| a.category == c))
        .filter(|a| status.is_none_or(|s| a.status == s))
        .filter(|a| valid_on.is_none_or(|date| a.is_valid_on(date)))
        .collect()
}

// ─── Response helpers ───────────────────────────────────────────────────────

/// `404` — unknown worker id.
fn worker_not_found(id: Uuid) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("Worker with id '{id}' not found") })),
    )
        .into_response()
}

/// `404` — unknown (or withdrawn, or other-worker) assessment id.
fn assessment_not_found(id: Uuid) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("Assessment with id '{id}' not found") })),
    )
        .into_response()
}

/// `500` — database or data-integrity failure.
fn db_error(e: &crate::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": format!("{e}") })),
    )
        .into_response()
}

/// `422` — one message (token parse / lifecycle refusal).
fn unprocessable(message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "error": message })),
    )
        .into_response()
}

/// `422` — the full validation problem list.
fn validation_failed(errors: &[crate::validation::ValidationError]) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "error": "Validation failed", "details": errors })),
    )
        .into_response()
}

/// Map a record-level authorization rejection to a JSON error response
/// (`403` policy-denied / `401` fail-safe).
fn rejection((status, reason): (StatusCode, String)) -> Response {
    (status, Json(json!({ "error": reason }))).into_response()
}

/// Parse a category token, or a `422`-ready message.
fn parse_category(token: &str) -> Result<AssessmentCategory, String> {
    AssessmentCategory::from_token(token).ok_or_else(|| {
        let known: Vec<&str> = AssessmentCategory::ALL.iter().map(|c| c.as_str()).collect();
        format!("unknown category {token:?} (categories: {known:?})")
    })
}

/// Parse a status token, or a `422`-ready message.
fn parse_status(token: &str) -> Result<AssessmentStatus, String> {
    AssessmentStatus::from_token(token).ok_or_else(|| {
        let known: Vec<&str> = AssessmentStatus::ALL.iter().map(|s| s.as_str()).collect();
        format!("unknown status {token:?} (statuses: {known:?})")
    })
}

/// Build the best-effort audit context for an assessment access from the
/// verified caller (its `sub`, or `system` when unauthenticated).
fn audit_ctx(caller: &MaybeAuthUser) -> AuditContext {
    AuditContext {
        user_id: caller
            .claims()
            .map(|c| c.sub.clone())
            .or_else(|| Some("system".to_string())),
        ip_address: None,
        user_agent: None,
    }
}

/// Borrow an [`AuditContext`] as the [`AuditActor`] the audit-log
/// helpers take.
fn actor(ctx: &AuditContext) -> AuditActor<'_> {
    AuditActor {
        user_id: ctx.user_id.as_deref(),
        ip_address: ctx.ip_address.as_deref(),
        user_agent: ctx.user_agent.as_deref(),
    }
}

/// Load the worker and authorise the request against it, returning the
/// worker plus the ABAC obligations the handler must honour (e.g.
/// `mask`). Every endpoint in this module starts here, so the
/// assessment surface can never be a way around the worker's own
/// authorization.
async fn worker_and_obligations(
    state: &AppState,
    id: Uuid,
    caller: &MaybeAuthUser,
    action: Action,
) -> Result<(Worker, Vec<String>), Response> {
    let worker = match state.worker_repository.get_by_id(&id).await {
        Ok(Some(w)) => w,
        Ok(None) => return Err(worker_not_found(id)),
        Err(e) => return Err(db_error(&e)),
    };
    match authorize_record(caller, action, &worker_resource_attrs(&worker)) {
        Ok(obligations) => Ok((worker, obligations)),
        Err(r) => Err(rejection(r)),
    }
}

/// Whether the ABAC decision requires a masked response.
fn wants_mask(obligations: &[String]) -> bool {
    obligations.iter().any(|o| o == "mask")
}

/// Project an assessment for the response, honouring the `mask`
/// obligation.
fn project(assessment: &Assessment, masked: bool) -> Assessment {
    if masked {
        assessment.masked()
    } else {
        assessment.clone()
    }
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// Record an assessment administration.
/// `POST /api/workers/{id}/assessments`.
///
/// Loads the worker (`404` if unknown), authorises at the worker-write
/// level, parses the category/status tokens and validates the payload
/// (`422` on failure), persists the row, writes an audit record, and
/// responds `201` with the stored assessment.
#[utoipa::path(
    post,
    path = "/api/workers/{id}/assessments",
    tag = "assessments",
    params(("id" = Uuid, Path, description = "Worker UUID")),
    responses(
        (status = 201, description = "The recorded assessment"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Policy denied"),
        (status = 404, description = "Worker not found"),
        (status = 422, description = "Validation failure"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn create_assessment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    Json(req): Json<CreateAssessmentRequest>,
) -> Response {
    let (worker, _obligations) =
        match worker_and_obligations(&state, id, &caller, Action::Write).await {
            Ok(pair) => pair,
            Err(response) => return response,
        };

    let category = match parse_category(&req.category) {
        Ok(c) => c,
        Err(reason) => return unprocessable(&reason),
    };
    let status = match req.status.as_deref().map(parse_status).transpose() {
        Ok(s) => s.unwrap_or(AssessmentStatus::Scheduled),
        Err(reason) => return unprocessable(&reason),
    };

    let mut assessment = Assessment::new(worker.id, category, req.instrument);
    assessment.provider = req.provider;
    assessment.status = status;
    assessment.administered_on = req.administered_on;
    assessment.expires_on = req.expires_on;
    assessment.administered_by = req.administered_by;
    assessment.notes = req.notes;
    assessment.results = req.results;

    let problems = validate_assessment(&assessment);
    if !problems.is_empty() {
        return validation_failed(&problems);
    }

    let stored = match assessments::insert(&state.db, &assessment).await {
        Ok(row) => row,
        Err(e) => return db_error(&e),
    };
    let created = match assessments::to_domain(&stored) {
        Ok(a) => a,
        Err(e) => return db_error(&e),
    };

    let ctx = audit_ctx(&caller);
    if let Ok(new_values) = serde_json::to_value(&created)
        && let Err(e) = state
            .audit_log
            .log_create("worker_assessment", created.id, new_values, &actor(&ctx))
            .await
    {
        tracing::warn!("failed to audit worker assessment create: {e}");
    }

    (StatusCode::CREATED, Json(created)).into_response()
}

/// List a worker's live assessments.
/// `GET /api/workers/{id}/assessments`.
///
/// Optional filters: `category`, `status`, and `valid_on` (keep only
/// assessments current on that date). Honours the `mask` obligation on
/// every row — a bulk read never reveals more than the equivalent single
/// read — and audits the disclosure.
#[utoipa::path(
    get,
    path = "/api/workers/{id}/assessments",
    tag = "assessments",
    params(
        ("id" = Uuid, Path, description = "Worker UUID"),
        ("category" = Option<String>, Query, description = "aptitude | personality | psychometric | selection"),
        ("status" = Option<String>, Query, description = "scheduled | in_progress | completed | expired | cancelled"),
        ("valid_on" = Option<String>, Query, description = "Keep only assessments current on this date (YYYY-MM-DD)"),
    ),
    responses(
        (status = 200, description = "The worker's assessments"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Policy denied"),
        (status = 404, description = "Worker not found"),
        (status = 422, description = "Unknown filter token"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn list_assessments(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    Query(query): Query<ListAssessmentsQuery>,
) -> Response {
    let (worker, obligations) =
        match worker_and_obligations(&state, id, &caller, Action::Read).await {
            Ok(pair) => pair,
            Err(response) => return response,
        };

    let category = match query.category.as_deref().map(parse_category).transpose() {
        Ok(c) => c,
        Err(reason) => return unprocessable(&reason),
    };
    let status = match query.status.as_deref().map(parse_status).transpose() {
        Ok(s) => s,
        Err(reason) => return unprocessable(&reason),
    };

    let stored = match assessments::list_for_worker(&state.db, worker.id).await {
        Ok(rows) => rows,
        Err(e) => return db_error(&e),
    };
    let all = match stored
        .iter()
        .map(assessments::to_domain)
        .collect::<crate::Result<Vec<_>>>()
    {
        Ok(list) => list,
        Err(e) => return db_error(&e),
    };
    let masked = wants_mask(&obligations);
    let filtered = apply_filters(all, category, status, query.valid_on);
    let view: Vec<Assessment> = filtered.iter().map(|a| project(a, masked)).collect();

    let ctx = audit_ctx(&caller);
    if let Err(e) = state
        .audit_log
        .log_export(
            "worker_assessments",
            worker.id,
            json!({ "count": view.len(), "masked": masked }),
            &actor(&ctx),
        )
        .await
    {
        tracing::warn!("failed to audit worker assessment list: {e}");
    }

    (StatusCode::OK, Json(view)).into_response()
}

/// Fetch one assessment.
/// `GET /api/workers/{id}/assessments/{assessment_id}`.
///
/// The lookup is worker-scoped, so an id belonging to another worker is
/// a `404` rather than a disclosure. Honours the `mask` obligation and
/// audits the read.
#[utoipa::path(
    get,
    path = "/api/workers/{id}/assessments/{assessment_id}",
    tag = "assessments",
    params(
        ("id" = Uuid, Path, description = "Worker UUID"),
        ("assessment_id" = Uuid, Path, description = "Assessment UUID"),
    ),
    responses(
        (status = 200, description = "The assessment"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Policy denied"),
        (status = 404, description = "Worker or assessment not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_assessment(
    State(state): State<AppState>,
    Path((id, assessment_id)): Path<(Uuid, Uuid)>,
    caller: MaybeAuthUser,
) -> Response {
    let (worker, obligations) =
        match worker_and_obligations(&state, id, &caller, Action::Read).await {
            Ok(pair) => pair,
            Err(response) => return response,
        };
    let stored = match assessments::find(&state.db, worker.id, assessment_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return assessment_not_found(assessment_id),
        Err(e) => return db_error(&e),
    };
    let assessment = match assessments::to_domain(&stored) {
        Ok(a) => a,
        Err(e) => return db_error(&e),
    };
    let masked = wants_mask(&obligations);

    let ctx = audit_ctx(&caller);
    if let Err(e) = state
        .audit_log
        .log_export(
            "worker_assessment",
            assessment.id,
            json!({ "worker_id": worker.id, "masked": masked }),
            &actor(&ctx),
        )
        .await
    {
        tracing::warn!("failed to audit worker assessment read: {e}");
    }

    (StatusCode::OK, Json(project(&assessment, masked))).into_response()
}

/// Update an assessment (status move, scoring, corrections).
/// `PUT /api/workers/{id}/assessments/{assessment_id}`.
///
/// A `status` change must be legal for the stored status
/// ([`AssessmentStatus::can_transition_to`]) — an illegal move is a
/// `422` naming the current state. The merged record is re-validated
/// before persisting, so an update can never leave the row in a state a
/// create would have rejected.
#[utoipa::path(
    put,
    path = "/api/workers/{id}/assessments/{assessment_id}",
    tag = "assessments",
    params(
        ("id" = Uuid, Path, description = "Worker UUID"),
        ("assessment_id" = Uuid, Path, description = "Assessment UUID"),
    ),
    responses(
        (status = 200, description = "The updated assessment"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Policy denied"),
        (status = 404, description = "Worker or assessment not found"),
        (status = 422, description = "Validation failure or illegal transition"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn update_assessment(
    State(state): State<AppState>,
    Path((id, assessment_id)): Path<(Uuid, Uuid)>,
    caller: MaybeAuthUser,
    Json(req): Json<UpdateAssessmentRequest>,
) -> Response {
    let (worker, _obligations) =
        match worker_and_obligations(&state, id, &caller, Action::Write).await {
            Ok(pair) => pair,
            Err(response) => return response,
        };
    let stored = match assessments::find(&state.db, worker.id, assessment_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return assessment_not_found(assessment_id),
        Err(e) => return db_error(&e),
    };
    let current = match assessments::to_domain(&stored) {
        Ok(a) => a,
        Err(e) => return db_error(&e),
    };

    // A status change must be a legal lifecycle move.
    let status = match req.status.as_deref().map(parse_status).transpose() {
        Ok(s) => s,
        Err(reason) => return unprocessable(&reason),
    };
    if let Some(to) = status
        && !current.status.can_transition_to(to)
    {
        return unprocessable(&format!(
            "illegal transition `{}` → `{to}` for assessment {assessment_id}",
            current.status
        ));
    }

    let change = AssessmentUpdate {
        status,
        instrument: req.instrument,
        provider: req.provider,
        administered_on: req.administered_on,
        expires_on: req.expires_on,
        administered_by: req.administered_by,
        notes: req.notes,
        results: req.results,
    };
    if change.is_empty() {
        return unprocessable("no fields to update");
    }

    // Validate the *merged* record, so an update cannot reach a state a
    // create would have refused.
    let mut merged = current.clone();
    if let Some(status) = change.status {
        merged.status = status;
    }
    if let Some(instrument) = &change.instrument {
        merged.instrument = instrument.clone();
    }
    if let Some(provider) = &change.provider {
        merged.provider = provider.clone();
    }
    if let Some(administered_on) = change.administered_on {
        merged.administered_on = administered_on;
    }
    if let Some(expires_on) = change.expires_on {
        merged.expires_on = expires_on;
    }
    if let Some(administered_by) = &change.administered_by {
        merged.administered_by = administered_by.clone();
    }
    if let Some(notes) = &change.notes {
        merged.notes = notes.clone();
    }
    if let Some(results) = &change.results {
        merged.results.clone_from(results);
    }
    let problems = validate_assessment(&merged);
    if !problems.is_empty() {
        return validation_failed(&problems);
    }

    let updated_row = match assessments::update(&state.db, stored, &change).await {
        Ok(row) => row,
        Err(e) => return db_error(&e),
    };
    let updated = match assessments::to_domain(&updated_row) {
        Ok(a) => a,
        Err(e) => return db_error(&e),
    };

    let ctx = audit_ctx(&caller);
    if let (Ok(old_values), Ok(new_values)) = (
        serde_json::to_value(&current),
        serde_json::to_value(&updated),
    ) && let Err(e) = state
        .audit_log
        .log_update(
            "worker_assessment",
            updated.id,
            old_values,
            new_values,
            &actor(&ctx),
        )
        .await
    {
        tracing::warn!("failed to audit worker assessment update: {e}");
    }

    (StatusCode::OK, Json(updated)).into_response()
}

/// Withdraw (soft-delete) an assessment.
/// `DELETE /api/workers/{id}/assessments/{assessment_id}`.
///
/// The row survives with `deleted_at` stamped so the audit trail stays
/// intact; the record drops out of every read path.
#[utoipa::path(
    delete,
    path = "/api/workers/{id}/assessments/{assessment_id}",
    tag = "assessments",
    params(
        ("id" = Uuid, Path, description = "Worker UUID"),
        ("assessment_id" = Uuid, Path, description = "Assessment UUID"),
    ),
    responses(
        (status = 200, description = "Withdrawn"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Policy denied"),
        (status = 404, description = "Worker or assessment not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn delete_assessment(
    State(state): State<AppState>,
    Path((id, assessment_id)): Path<(Uuid, Uuid)>,
    caller: MaybeAuthUser,
) -> Response {
    let (worker, _obligations) =
        match worker_and_obligations(&state, id, &caller, Action::Delete).await {
            Ok(pair) => pair,
            Err(response) => return response,
        };
    let stored = match assessments::find(&state.db, worker.id, assessment_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return assessment_not_found(assessment_id),
        Err(e) => return db_error(&e),
    };
    let old_values = assessments::to_domain(&stored)
        .ok()
        .and_then(|a| serde_json::to_value(&a).ok());

    match assessments::soft_delete(&state.db, stored).await {
        Ok(deleted) => {
            let ctx = audit_ctx(&caller);
            if let Some(old_values) = old_values
                && let Err(e) = state
                    .audit_log
                    .log_delete("worker_assessment", deleted.id, old_values, &actor(&ctx))
                    .await
            {
                tracing::warn!("failed to audit worker assessment delete: {e}");
            }
            (StatusCode::OK, Json(json!({}))).into_response()
        }
        Err(e) => db_error(&e),
    }
}

/// The derived assessment profile.
/// `GET /api/workers/{id}/assessment-profile`.
///
/// Rolls the worker's live assessments up into the current reading per
/// scale in each category, what has **not** been assessed, and the
/// selection-suitability mean. Honours the `mask` obligation (bands
/// survive, percentiles and the suitability figure do not) and audits
/// the disclosure.
#[utoipa::path(
    get,
    path = "/api/workers/{id}/assessment-profile",
    tag = "assessments",
    params(
        ("id" = Uuid, Path, description = "Worker UUID"),
        ("as_of" = Option<String>, Query, description = "Judge validity as of this date (YYYY-MM-DD; default today)"),
    ),
    responses(
        (status = 200, description = "The derived profile"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Policy denied"),
        (status = 404, description = "Worker not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn assessment_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    Query(query): Query<ProfileQuery>,
) -> Response {
    let (worker, obligations) =
        match worker_and_obligations(&state, id, &caller, Action::Read).await {
            Ok(pair) => pair,
            Err(response) => return response,
        };
    let stored = match assessments::list_for_worker(&state.db, worker.id).await {
        Ok(rows) => rows,
        Err(e) => return db_error(&e),
    };
    let all = match stored
        .iter()
        .map(assessments::to_domain)
        .collect::<crate::Result<Vec<_>>>()
    {
        Ok(list) => list,
        Err(e) => return db_error(&e),
    };
    let as_of = query.as_of.unwrap_or_else(|| Utc::now().date_naive());
    let masked = wants_mask(&obligations);
    let profile = build_profile(worker.id, &all, as_of, masked);

    let ctx = audit_ctx(&caller);
    if let Err(e) = state
        .audit_log
        .log_export(
            "worker_assessment_profile",
            worker.id,
            json!({ "as_of": as_of, "masked": masked }),
            &actor(&ctx),
        )
        .await
    {
        tracing::warn!("failed to audit worker assessment profile read: {e}");
    }

    (StatusCode::OK, Json(profile)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed date, so the pure derivations are clock-free.
    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
    }

    /// A completed assessment administered on `on`, expiring on
    /// `expires`, carrying `results`.
    fn completed(
        worker_id: Uuid,
        category: AssessmentCategory,
        instrument: &str,
        on: NaiveDate,
        expires: Option<NaiveDate>,
        results: Vec<AssessmentResult>,
    ) -> Assessment {
        let mut a = Assessment::new(worker_id, category, instrument);
        a.status = AssessmentStatus::Completed;
        a.administered_on = Some(on);
        a.expires_on = expires;
        a.results = results;
        a
    }

    /// Unknown filter tokens are rejected with a message naming the
    /// closed vocabulary (the handler turns this into a `422`).
    #[test]
    fn unknown_tokens_are_rejected() {
        assert!(parse_category("aptitude").is_ok());
        assert!(parse_status("in_progress").is_ok());

        let category = parse_category("astrology").expect_err("unknown category");
        assert!(category.contains("astrology") && category.contains("psychometric"));
        let status = parse_status("vibing").expect_err("unknown status");
        assert!(status.contains("vibing") && status.contains("completed"));
    }

    /// The list filters compose: category, status, and validity each
    /// narrow the set, and validity excludes non-completed and expired
    /// records.
    #[test]
    fn filters_narrow_the_list() {
        let worker = Uuid::new_v4();
        let aptitude = completed(
            worker,
            AssessmentCategory::Aptitude,
            "SHL Verify",
            day(2026, 1, 5),
            Some(day(2027, 1, 5)),
            vec![AssessmentResult::percentile(
                AssessmentScale::NumericalReasoning,
                80.0,
            )],
        );
        let expired = completed(
            worker,
            AssessmentCategory::Aptitude,
            "Old test",
            day(2020, 1, 5),
            Some(day(2021, 1, 5)),
            vec![AssessmentResult::percentile(
                AssessmentScale::VerbalReasoning,
                55.0,
            )],
        );
        let scheduled = Assessment::new(worker, AssessmentCategory::Selection, "Work sample");
        let all = vec![aptitude.clone(), expired.clone(), scheduled.clone()];

        // No filters ⇒ everything.
        assert_eq!(apply_filters(all.clone(), None, None, None).len(), 3);
        // Category.
        let only_aptitude =
            apply_filters(all.clone(), Some(AssessmentCategory::Aptitude), None, None);
        assert_eq!(only_aptitude.len(), 2);
        // Status.
        let only_scheduled =
            apply_filters(all.clone(), None, Some(AssessmentStatus::Scheduled), None);
        assert_eq!(only_scheduled.len(), 1);
        assert_eq!(only_scheduled[0].id, scheduled.id);
        // Validity drops the expired and the not-yet-taken.
        let current = apply_filters(all.clone(), None, None, Some(day(2026, 7, 23)));
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].id, aptitude.id);
        // Filters compose.
        let none = apply_filters(
            all,
            Some(AssessmentCategory::Selection),
            None,
            Some(day(2026, 7, 23)),
        );
        assert!(none.is_empty(), "the selection test is only scheduled");
    }

    /// The profile reports the most recent current reading per scale,
    /// counts recorded vs current per category, and names the scales
    /// with no current reading.
    #[test]
    fn profile_takes_the_most_recent_current_reading() {
        let worker = Uuid::new_v4();
        let old = completed(
            worker,
            AssessmentCategory::Aptitude,
            "SHL Verify (2024)",
            day(2024, 6, 1),
            None,
            vec![AssessmentResult::percentile(
                AssessmentScale::NumericalReasoning,
                40.0,
            )],
        );
        let new = completed(
            worker,
            AssessmentCategory::Aptitude,
            "SHL Verify G+ (2026)",
            day(2026, 6, 1),
            None,
            vec![AssessmentResult::percentile(
                AssessmentScale::NumericalReasoning,
                95.0,
            )],
        );
        let stale = completed(
            worker,
            AssessmentCategory::Aptitude,
            "Expired test",
            day(2019, 1, 1),
            Some(day(2020, 1, 1)),
            vec![AssessmentResult::percentile(
                AssessmentScale::VerbalReasoning,
                99.0,
            )],
        );

        let profile = build_profile(worker, &[old, new.clone(), stale], day(2026, 7, 23), false);
        let aptitude = profile
            .categories
            .iter()
            .find(|c| c.category == AssessmentCategory::Aptitude)
            .expect("aptitude slice");

        assert_eq!(aptitude.recorded, 3, "three aptitude assessments on file");
        assert_eq!(aptitude.valid, 2, "the expired one is not current");
        assert_eq!(
            aptitude.scales.len(),
            1,
            "only one scale has a current reading"
        );

        let reading = &aptitude.scales[0];
        assert_eq!(reading.scale, AssessmentScale::NumericalReasoning);
        assert_eq!(reading.assessment_id, new.id, "the 2026 sitting wins");
        assert_eq!(reading.percentile, Some(95.0));
        assert_eq!(reading.band, Some(ScoreBand::High));

        // The expired verbal result does not count as a current reading.
        assert!(
            aptitude
                .scales_not_assessed
                .contains(&AssessmentScale::VerbalReasoning),
            "an expired reading leaves the scale unassessed"
        );
        assert!(
            aptitude
                .scales_not_assessed
                .contains(&AssessmentScale::LogicalThinking)
        );
    }

    /// Selection suitability is the mean percentile of current selection
    /// results, and is `None` when nothing current carries one.
    #[test]
    fn selection_suitability_averages_current_selection_scores() {
        let worker = Uuid::new_v4();
        let as_of = day(2026, 7, 23);
        let selection = completed(
            worker,
            AssessmentCategory::Selection,
            "Assessment centre",
            day(2026, 7, 1),
            None,
            vec![
                AssessmentResult::percentile(AssessmentScale::JobSimulation, 90.0),
                AssessmentResult::percentile(AssessmentScale::JudgementTest, 70.0),
            ],
        );
        // An aptitude score must not leak into the suitability figure.
        let aptitude = completed(
            worker,
            AssessmentCategory::Aptitude,
            "SHL Verify",
            day(2026, 7, 1),
            None,
            vec![AssessmentResult::percentile(
                AssessmentScale::NumericalReasoning,
                10.0,
            )],
        );

        let profile = build_profile(worker, &[selection.clone(), aptitude], as_of, false);
        let mean = profile.selection_suitability.expect("two scored results");
        assert!(
            (mean - 80.0).abs() < f64::EPSILON,
            "mean of 90 and 70 is 80, got {mean}"
        );

        // With nothing current, there is no figure — never a zero.
        let scheduled = Assessment::new(worker, AssessmentCategory::Selection, "Booked");
        let empty = build_profile(worker, &[scheduled], as_of, false);
        assert_eq!(empty.selection_suitability, None);
    }

    /// SEC-G3 / invariant 5: the masked profile keeps the bands but
    /// discloses neither percentiles nor the suitability figure — the
    /// aggregate view must not reveal more than a masked single read.
    #[test]
    fn masked_profile_withholds_scores() {
        let worker = Uuid::new_v4();
        let as_of = day(2026, 7, 23);
        let selection = completed(
            worker,
            AssessmentCategory::Selection,
            "Assessment centre",
            day(2026, 7, 1),
            None,
            vec![AssessmentResult::percentile(
                AssessmentScale::JobSimulation,
                90.0,
            )],
        );

        let open = build_profile(worker, std::slice::from_ref(&selection), as_of, false);
        assert_eq!(open.selection_suitability, Some(90.0));

        let masked = build_profile(worker, &[selection], as_of, true);
        assert!(masked.masked);
        assert_eq!(
            masked.selection_suitability, None,
            "the suitability mean is a score by another name"
        );
        let reading = &masked
            .categories
            .iter()
            .find(|c| c.category == AssessmentCategory::Selection)
            .expect("selection slice")
            .scales[0];
        assert_eq!(reading.band, Some(ScoreBand::High), "the band survives");
        assert_eq!(reading.percentile, None, "the percentile is redacted");
    }

    /// `project` honours the mask flag on the single-record path, and
    /// `wants_mask` reads the obligation list.
    #[test]
    fn projection_honours_the_mask_obligation() {
        let worker = Uuid::new_v4();
        let mut a = completed(
            worker,
            AssessmentCategory::Personality,
            "Big Five",
            day(2026, 2, 2),
            None,
            vec![AssessmentResult::percentile(
                AssessmentScale::TeamCompatibility,
                65.0,
            )],
        );
        a.notes = Some("sat remotely".to_string());

        assert!(!wants_mask(&[]));
        assert!(!wants_mask(&["audit".to_string()]));
        assert!(wants_mask(&["audit".to_string(), "mask".to_string()]));

        let open = project(&a, false);
        assert_eq!(open.results[0].percentile, Some(65.0));
        assert_eq!(open.notes.as_deref(), Some("sat remotely"));

        let masked = project(&a, true);
        assert_eq!(masked.results[0].percentile, None);
        assert_eq!(masked.results[0].band, Some(ScoreBand::Average));
        assert!(masked.notes.is_none());
    }

    /// Every category appears in the profile, even with no data — the
    /// view states what has not been assessed rather than omitting it.
    #[test]
    fn profile_covers_every_category_even_when_empty() {
        let worker = Uuid::new_v4();
        let profile = build_profile(worker, &[], day(2026, 7, 23), false);
        assert_eq!(profile.categories.len(), AssessmentCategory::ALL.len());
        for (slice, expected) in profile.categories.iter().zip(AssessmentCategory::ALL) {
            assert_eq!(slice.category, expected);
            assert_eq!(slice.recorded, 0);
            assert_eq!(slice.valid, 0);
            assert!(slice.scales.is_empty());
            assert_eq!(
                slice.scales_not_assessed.len(),
                expected.own_scales().len(),
                "every own scale is reported as unassessed"
            );
        }
        assert_eq!(profile.selection_suitability, None);
    }

    /// A dated administration outranks an undated one, and equal dates
    /// break on `updated_at`, so the current reading is deterministic.
    #[test]
    fn recency_ordering_is_deterministic() {
        let worker = Uuid::new_v4();
        let mut dated = Assessment::new(worker, AssessmentCategory::Aptitude, "Dated");
        dated.administered_on = Some(day(2026, 1, 1));
        let undated = Assessment::new(worker, AssessmentCategory::Aptitude, "Undated");

        assert!(is_more_recent(&dated, &undated), "a date beats no date");
        assert!(!is_more_recent(&undated, &dated));

        let mut same_day = dated.clone();
        same_day.updated_at = dated.updated_at + chrono::Duration::seconds(1);
        assert!(
            is_more_recent(&same_day, &dated),
            "same date ⇒ the later update wins"
        );
    }
}
