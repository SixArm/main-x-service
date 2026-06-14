//! Care-pathway CRUD + matching endpoints.
//!
//! The API DTO is `care_pathway_matcher::CarePathway` itself — the
//! service stores it verbatim (as JSON) and matches with the canonical
//! `care-pathway-matcher` engine, so there is no separate model or
//! adapter to drift.

use axum::http::StatusCode;
use care_pathway_matcher::{CarePathway, MatchConfig, MatchingEngine};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::{AuthUser, MaybeAuthUser};
use crate::merge::merge_pathways;
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::streaming::{self, EventKind};

/// Maximum number of stored pathways scanned in-memory by
/// `check-duplicates`.
///
/// `check-duplicates` has no search-backed candidate blocking yet
/// (deferred — spec §13 T-6), so it loads up to this many active rows
/// and matches each against the query. When the scan reaches this cap
/// the result may be incomplete; the handler emits a `WARN`. Raising
/// the cap is a stop-gap — the real fix is search-blocked candidates.
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// Validate an incoming `CarePathway` payload.
///
/// Family convention (OQ-1 resolution): validation failures return
/// `422 Unprocessable Entity`, matching the person/place services.
/// loco has no `unprocessable_entity` helper, so this uses
/// `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`.
///
/// The concrete rules — required `name`, and ICD-10 / ICD-11 / SNOMED CT
/// code-format checks on `condition_codes` — live in
/// [`crate::validation`]; every problem found is reported in one
/// response so the caller can fix them in a single round-trip.
///
/// # Errors
///
/// Returns a `422` error when `name` is blank or any `condition_codes`
/// entry is malformed for its declared coding system.
pub fn validate(pathway: &CarePathway) -> Result<()> {
    let problems = crate::validation::problems(pathway);
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", &problems.join("; ")),
    ))
}

/// Lightweight reference to a stored pathway: the public id plus the
/// denormalised name. Returned by create/update/list/search instead of
/// the full payload, so callers can render a link without a second fetch.
#[derive(Debug, Serialize)]
struct PathwayRef {
    /// The pathway's public id (UUID), as a string.
    pid: String,
    /// The pathway's denormalised display name.
    name: String,
}

impl PathwayRef {
    /// Project a stored [`PathwayModel`] row down to its `{pid, name}`
    /// reference. Used wherever a handler returns rows without the body.
    fn of(m: &PathwayModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            name: m.name.clone(),
        }
    }
}

/// Request body for `POST /match`: a query plus the explicit candidate
/// list to rank it against. Nothing is persisted — this is pure scoring.
#[derive(Debug, Deserialize)]
struct MatchRequest {
    /// The pathway to score against each candidate.
    query: CarePathway,
    /// The candidate pathways to rank by match score.
    candidates: Vec<CarePathway>,
}

/// Query string for `GET /search`: the (optional) `q` term. Absent or
/// blank `q` is rejected by the handler as a `400`.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// The case-insensitive substring to match against pathway names.
    q: Option<String>,
}

/// Request body for `POST /merge`: which duplicate folds into which
/// survivor, with an optional reason recorded in the merge history.
#[derive(Debug, Deserialize)]
struct MergeRequest {
    /// The surviving pathway's public id.
    main_pid: String,
    /// The duplicate to merge in and soft-delete.
    duplicate_pid: String,
    /// Optional operator-supplied reason, recorded in the merge history.
    #[serde(default)]
    reason: Option<String>,
}

/// A scored `check-duplicates` hit: a stored pathway that matched the
/// query above the engine threshold, with its score and classification.
#[derive(Debug, Serialize)]
struct ScoredRef {
    /// The matched pathway's public id (UUID), as a string.
    pid: String,
    /// The matched pathway's display name.
    name: String,
    /// Overall match score in `[0.0, 1.0]`.
    score: f64,
    /// Human-readable confidence band (the matcher's `Confidence` debug).
    confidence: String,
    /// Whether the engine classified this as a match (always `true` here,
    /// since non-matches are filtered out before this struct is built).
    is_match: bool,
}

/// Create a care pathway.
///
/// `POST /api/care-pathways` — request body is a [`CarePathway`]; response
/// is a [`PathwayRef`] (`{pid, name}`). Validates first (`422` on blank
/// name / malformed codes), inserts, audits, and publishes a `Created`
/// event. `caller` is the optional bearer identity, stamped as the audit
/// `actor` and event actor when present.
///
/// # Errors
///
/// `422` on validation failure; otherwise propagates DB/serialization
/// errors.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(pathway): Json<CarePathway>,
) -> Result<Response> {
    validate(&pathway)?;
    let model = PathwayModel::create(&ctx.db, &pathway).await?;
    Metrics::global().care_pathway_created_total.inc();
    // Audit + event are best-effort side channels; the create itself has
    // already committed by the time we record them.
    audit(
        &ctx,
        model.pid,
        "created",
        caller.actor(),
        Some(model.data.clone()),
    )
    .await;
    streaming::publish_with_actor(
        EventKind::Created,
        &model.pid.to_string(),
        &model.name,
        caller.actor(),
    );
    format::json(PathwayRef::of(&model))
}

/// Best-effort audit write: log on failure but never fail the request.
/// `actor` is the verified caller `sub` when a token was presented.
///
/// `action` is the verb recorded (`"created"`, `"updated"`, `"deleted"`,
/// `"merged"`, `"merged_into"`); `snapshot` is the post-change payload
/// (or `None` for deletes). A failure here is logged at `WARN` and
/// swallowed — the audit trail is secondary to completing the request.
async fn audit(
    ctx: &AppContext,
    entity_pid: uuid::Uuid,
    action: &str,
    actor: Option<&str>,
    snapshot: Option<serde_json::Value>,
) {
    if let Err(err) = AuditModel::record(&ctx.db, entity_pid, action, actor, snapshot).await {
        tracing::warn!(error = %err, action, "failed to write audit log");
    }
}

/// Fetch a care pathway by public id.
///
/// `GET /api/care-pathways/{pid}` — response is the full stored
/// [`CarePathway`]. `404` when `pid` is unknown or soft-deleted.
///
/// # Errors
///
/// `404` when no active row has that `pid`; otherwise DB/parse errors.
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = PathwayModel::find_by_pid(&ctx.db, &pid).await?;
    format::json(model.to_pathway()?)
}

/// Replace a care pathway's payload.
///
/// `PUT /api/care-pathways/{pid}` — request body is a [`CarePathway`];
/// response is a [`PathwayRef`]. Validates, replaces the stored payload,
/// audits, and publishes an `Updated` event.
///
/// # Errors
///
/// `422` on validation failure, `404` on unknown `pid`.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(pathway): Json<CarePathway>,
) -> Result<Response> {
    validate(&pathway)?;
    let model = PathwayModel::find_by_pid(&ctx.db, &pid).await?;
    let updated = model
        .into_active_model()
        .update_data(&ctx.db, &pathway)
        .await?;
    Metrics::global().care_pathway_updated_total.inc();
    audit(
        &ctx,
        updated.pid,
        "updated",
        caller.actor(),
        Some(updated.data.clone()),
    )
    .await;
    streaming::publish_with_actor(
        EventKind::Updated,
        &updated.pid.to_string(),
        &updated.name,
        caller.actor(),
    );
    format::json(PathwayRef::of(&updated))
}

/// Soft-delete a care pathway.
///
/// `DELETE /api/care-pathways/{pid}` — marks the row inactive and stamps
/// `deleted_at`; the row is retained for audit. Audits with `None`
/// snapshot and publishes a `Deleted` event. Response is empty JSON.
///
/// # Errors
///
/// `404` when `pid` is unknown.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = PathwayModel::find_by_pid(&ctx.db, &pid).await?;
    // Capture identity before consuming the model into an active model.
    let (entity_pid, name) = (model.pid, model.name.clone());
    model.into_active_model().soft_delete(&ctx.db).await?;
    Metrics::global().care_pathway_deleted_total.inc();
    audit(&ctx, entity_pid, "deleted", caller.actor(), None).await;
    streaming::publish_with_actor(
        EventKind::Deleted,
        &entity_pid.to_string(),
        &name,
        caller.actor(),
    );
    format::empty_json()
}

/// List active care pathways (capped at 100).
///
/// `GET /api/care-pathways` — response is a `[PathwayRef]` array, newest
/// first. The 100 cap is a pragmatic bound until pagination lands.
///
/// # Errors
///
/// Propagates DB query errors.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = PathwayModel::list(&ctx.db, 100).await?;
    let refs: Vec<PathwayRef> = rows.iter().map(PathwayRef::of).collect();
    format::json(refs)
}

/// Case-insensitive name search: `GET /api/care-pathways/search?q=stroke`.
/// Pragmatic Postgres `ILIKE` over the denormalised `name` (cap 50);
/// full-text / fuzzy search is deferred (spec §13 T-6). Response is a
/// `[PathwayRef]` array; a missing or blank `q` is a `400`.
///
/// # Errors
///
/// `400` when `q` is absent or whitespace-only; otherwise DB errors.
#[debug_handler]
async fn search(
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let q = params.q.unwrap_or_default();
    // Reject an absent/blank term rather than ILIKE-ing on `%%`.
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let rows = PathwayModel::search(&ctx.db, q.trim(), 50).await?;
    let refs: Vec<PathwayRef> = rows.iter().map(PathwayRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
///
/// `POST /api/care-pathways/match` — request body is a [`MatchRequest`];
/// response is the matcher engine's ranked results (`[index, MatchResult]`
/// pairs). Stateless: nothing is read from or written to the database.
///
/// # Errors
///
/// None beyond response serialization (the engine is infallible here).
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored care pathways that match the query above the threshold.
///
/// `POST /api/care-pathways/check-duplicates` — request body is a
/// [`CarePathway`] query; response is a score-sorted `[ScoredRef]` array.
/// In-memory scan (no search-backed blocking yet): loads up to
/// [`CHECK_DUPLICATES_SCAN_CAP`] active rows and matches each, logging a
/// `WARN` if the cap is hit (results may then be incomplete — spec §13 T-6).
///
/// # Errors
///
/// Propagates DB query and payload-parse errors.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<CarePathway>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = PathwayModel::list(&ctx.db, CHECK_DUPLICATES_SCAN_CAP).await?;
    // Hitting the cap means the scan was truncated; the result set may
    // miss matches beyond the first `CAP` rows.
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "check-duplicates in-memory scan hit its row cap; results may be \
             incomplete until search-backed candidate blocking lands (spec §13 T-6)"
        );
    }
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        // Each stored row is parsed back into a `CarePathway` and scored
        // against the query; only classified matches are surfaced.
        let candidate = row.to_pathway()?;
        let r = engine.match_care_pathways(&query, &candidate);
        if r.is_match {
            hits.push(ScoredRef {
                pid: row.pid.to_string(),
                name: row.name.clone(),
                score: r.score,
                confidence: format!("{:?}", r.confidence),
                is_match: r.is_match,
            });
        }
    }
    // Best matches first. `f64` is only partially ordered, so NaN scores
    // (which the engine never emits) fall back to "equal" rather than panic.
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    format::json(hits)
}

/// Merge a confirmed-duplicate pathway into a surviving (main) pathway:
/// union the duplicate's data into main, keep the duplicate's title as an
/// alternate name, soft-delete the duplicate, record the merge history,
/// and publish a `Merged` event (plus a `Deleted` for the duplicate).
///
/// `POST /api/care-pathways/merge` — request body is a [`MergeRequest`];
/// response is `{main_pid, duplicate_pid, main}` (the survivor's merged
/// payload). The pure folding logic lives in [`merge_pathways`]; this
/// handler does the DB orchestration and side effects.
///
/// # Errors
///
/// `422` when `main_pid == duplicate_pid`; `404` when either pid is
/// unknown; otherwise DB/parse errors.
#[debug_handler]
async fn merge(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<MergeRequest>,
) -> Result<Response> {
    // Reject self-merge up front: folding a record into itself is a no-op
    // that would still soft-delete the only copy.
    if req.main_pid == req.duplicate_pid {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("validation", "main_pid and duplicate_pid must differ"),
        ));
    }
    // Both must exist and be active; either missing is the `find_by_pid`
    // `404`.
    let main = PathwayModel::find_by_pid(&ctx.db, &req.main_pid).await?;
    let duplicate = PathwayModel::find_by_pid(&ctx.db, &req.duplicate_pid).await?;

    let outcome = merge_pathways(&main.to_pathway()?, &duplicate.to_pathway()?);

    // Update the survivor, then retire the duplicate.
    let merged = main
        .into_active_model()
        .update_data(&ctx.db, &outcome.merged)
        .await?;
    let (dup_pid, dup_name) = (duplicate.pid, duplicate.name.clone());
    duplicate.into_active_model().soft_delete(&ctx.db).await?;
    Metrics::global().care_pathway_merged_total.inc();

    if let Err(err) = MergeRecordModel::record(
        &ctx.db,
        merged.pid,
        dup_pid,
        req.reason.as_deref(),
        caller.actor(),
        Some(outcome.transferred),
    )
    .await
    {
        // The merge-history row is best-effort, like the audit log; a
        // failure here must not roll back the already-committed merge.
        tracing::warn!(error = %err, "failed to write merge record");
    }
    // Two audit entries: one on the survivor ("merged"), one on the
    // retired duplicate ("merged_into") so both pids carry the trail.
    audit(
        &ctx,
        merged.pid,
        "merged",
        caller.actor(),
        Some(merged.data.clone()),
    )
    .await;
    audit(&ctx, dup_pid, "merged_into", caller.actor(), None).await;
    streaming::publish_with_actor(
        EventKind::Merged,
        &merged.pid.to_string(),
        &merged.name,
        caller.actor(),
    );
    streaming::publish_with_actor(
        EventKind::Deleted,
        &dup_pid.to_string(),
        &dup_name,
        caller.actor(),
    );

    format::json(serde_json::json!({
        "main_pid": merged.pid.to_string(),
        "duplicate_pid": dup_pid.to_string(),
        "main": merged.to_pathway()?,
    }))
}

/// Recent merge-history records, newest first.
///
/// `GET /api/care-pathways/merges/recent` — capped at 100 rows.
///
/// # Errors
///
/// Propagates DB query errors.
#[debug_handler]
async fn recent_merges(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Recent audit-log entries across all care pathways.
///
/// `GET /api/care-pathways/audit/recent` — system-wide trail, newest
/// first, capped at 100 rows.
///
/// # Errors
///
/// Propagates DB query errors.
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single care pathway.
///
/// `GET /api/care-pathways/{pid}/audit` — every audit row for one pathway,
/// newest first. A `pid` that is not a UUID is a `400`.
///
/// # Errors
///
/// `400` when `pid` is not a valid UUID; otherwise DB query errors.
#[debug_handler]
async fn entity_audit(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    // Audit rows key on the UUID `entity_pid`, so a non-UUID path segment
    // can never match — reject it as a `400` rather than scanning.
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::for_entity(&ctx.db, uuid).await?;
    format::json(rows)
}

/// Recent events from the in-memory event stream.
///
/// `GET /api/care-pathways/events/recent` — the last 100
/// [`streaming::EventView`]s from the process-local ring buffer (not
/// durable across restarts).
///
/// # Errors
///
/// None — reads the in-memory buffer.
#[debug_handler]
async fn recent_events() -> Result<Response> {
    format::json(streaming::recent(100))
}

/// Echo the verified claims of the bearer token — `401` when the token is
/// missing or fails verification. Proves peer JWT verification against
/// the auth-service JWKS end to end (spec §13 T-7).
///
/// `GET /api/care-pathways/whoami` — the [`AuthUser`] extractor performs
/// the verification, so the handler is reached only with valid [`Claims`];
/// the `401` is produced by the extractor's rejection before this runs.
///
/// # Errors
///
/// `401` (via the extractor) when the bearer token is absent or invalid.
#[debug_handler]
async fn whoami(AuthUser(claims): AuthUser) -> Result<Response> {
    format::json(claims)
}

/// Build the `/api/care-pathways` route table (CRUD + match +
/// check-duplicates + audit / event endpoints).
///
/// Loco matches routes most-specific-first, so the fixed sub-paths
/// (`/search`, `/match`, `/merge`, …) are registered before the `/{pid}`
/// catch-alls to avoid the parameter route shadowing them.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/", post(create))
        .add("/", get(list))
        .add("/search", get(search))
        .add("/match", post(match_against))
        .add("/check-duplicates", post(check_duplicates))
        .add("/merge", post(merge))
        .add("/merges/recent", get(recent_merges))
        .add("/whoami", get(whoami))
        .add("/audit/recent", get(recent_audit))
        .add("/events/recent", get(recent_events))
        .add("/{pid}", get(get_one))
        .add("/{pid}", put(update))
        .add("/{pid}", delete(remove))
        .add("/{pid}/audit", get(entity_audit))
}

/// DB-free controller-level tests. These exercise [`validate`] and the
/// scan-cap constant directly (no `AppContext`/database), so they run on
/// the default `cargo test` and pin the validation→status contract that
/// the DB-gated request tests assert end to end.
#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the OQ-1 / T-2 decision: blank-name validation failure is
    /// `422 Unprocessable Entity` (family convention), not `400`.
    /// Runs without a database, so the pin holds on default `cargo test`.
    #[test]
    fn blank_name_validation_returns_422() {
        for name in ["", "   ", "\t\n"] {
            let err = validate(&CarePathway::new(name)).expect_err("blank name must fail");
            match err {
                Error::CustomError(status, _) => {
                    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                }
                other => panic!("expected CustomError(422), got {other:?}"),
            }
        }
    }

    /// The 422 must survive loco's error-to-response conversion.
    #[test]
    fn blank_name_validation_response_status_is_422() {
        let err = validate(&CarePathway::new("")).expect_err("blank name must fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// A well-formed payload (non-blank name, no codes) validates `Ok`.
    #[test]
    fn non_blank_name_passes_validation() {
        assert!(validate(&CarePathway::new("Acute Stroke Care Pathway")).is_ok());
    }

    /// A malformed clinical code (here, an ICD-10 code that is not
    /// well-formed) is a validation failure surfaced as `422`, the same
    /// status as a blank name. Runs without a database.
    #[test]
    fn malformed_condition_code_returns_422() {
        use care_pathway_matcher::{CodeSystem, ConditionCode};
        let pathway = CarePathway {
            condition_codes: vec![ConditionCode {
                system: CodeSystem::Icd10,
                code: "not-a-code".to_string(),
            }],
            ..CarePathway::new("Acute Stroke Care Pathway")
        };
        let err = validate(&pathway).expect_err("malformed code must fail");
        assert_eq!(
            err.into_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    /// Pins the documented `check-duplicates` in-memory scan cap. The
    /// handler must pass this named const to `list` (not a magic
    /// number) and WARN when the scan reaches it (spec §13 T-6).
    #[test]
    fn check_duplicates_scan_cap_is_the_documented_value() {
        assert_eq!(CHECK_DUPLICATES_SCAN_CAP, 1000);
    }
}
