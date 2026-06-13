//! Case CRUD + matching endpoints.
//!
//! The API DTO is `case_matcher::Case` itself — the service stores it
//! verbatim (as JSON) and matches with the canonical `case-matcher`
//! engine, so there is no separate model or adapter to drift.

use axum::http::StatusCode;
use case_matcher::{Case, MatchConfig, MatchingEngine};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::{AuthUser, MaybeAuthUser};
use crate::merge::merge_cases;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::cases::Model as CaseModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::streaming::{self, EventKind};

/// Maximum number of stored cases scanned in-memory by
/// `check-duplicates`.
///
/// `check-duplicates` has no search-backed candidate blocking yet
/// (deferred — spec §13 T-6), so it loads up to this many active rows
/// and matches each against the query. When the scan reaches this cap
/// the result may be incomplete; the handler emits a `WARN`. Raising
/// the cap is a stop-gap — the real fix is search-blocked candidates.
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// Validate an incoming `Case` payload.
///
/// Family convention (OQ-1 resolution): validation failures return
/// `422 Unprocessable Entity`, matching the person/place services.
/// loco has no `unprocessable_entity` helper, so this uses
/// `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`.
///
/// The concrete rules — required `title`, ISO-8601 `opened_date`,
/// non-blank identifier values, and non-blank `subjects` / `keywords`
/// entries — live in [`crate::validation`]; every problem found is
/// reported in one response so the caller can fix them in a single
/// round-trip.
///
/// # Errors
///
/// Returns a `422` error when `title` is blank, `opened_date` is not a
/// valid ISO-8601 date, an identifier value is blank, or a `subjects` /
/// `keywords` entry is blank.
pub fn validate(case: &Case) -> Result<()> {
    let problems = crate::validation::problems(case);
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", &problems.join("; ")),
    ))
}

#[derive(Debug, Serialize)]
struct CaseRef {
    pid: String,
    title: String,
}

impl CaseRef {
    fn of(m: &CaseModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            title: m.title.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MatchRequest {
    query: Case,
    candidates: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MergeRequest {
    /// The surviving case's public id.
    main_pid: String,
    /// The duplicate to merge in and soft-delete.
    duplicate_pid: String,
    /// Optional operator-supplied reason, recorded in the merge history.
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScoredRef {
    pid: String,
    title: String,
    score: f64,
    confidence: String,
    is_match: bool,
}

/// Create a case.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(case): Json<Case>,
) -> Result<Response> {
    validate(&case)?;
    let model = CaseModel::create(&ctx.db, &case).await?;
    audit(
        &ctx,
        model.pid,
        "created",
        caller.actor(),
        Some(model.data.clone()),
    )
    .await;
    streaming::publish(EventKind::Created, &model.pid.to_string(), &model.title);
    format::json(CaseRef::of(&model))
}

/// Best-effort audit write: log on failure but never fail the request.
/// `actor` is the verified caller `sub` when a token was presented.
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

/// Fetch a case by public id.
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = CaseModel::find_by_pid(&ctx.db, &pid).await?;
    format::json(model.to_case()?)
}

/// Replace a case's payload.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(case): Json<Case>,
) -> Result<Response> {
    validate(&case)?;
    let model = CaseModel::find_by_pid(&ctx.db, &pid).await?;
    let updated = model
        .into_active_model()
        .update_data(&ctx.db, &case)
        .await?;
    audit(
        &ctx,
        updated.pid,
        "updated",
        caller.actor(),
        Some(updated.data.clone()),
    )
    .await;
    streaming::publish(EventKind::Updated, &updated.pid.to_string(), &updated.title);
    format::json(CaseRef::of(&updated))
}

/// Soft-delete a case.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = CaseModel::find_by_pid(&ctx.db, &pid).await?;
    let (entity_pid, title) = (model.pid, model.title.clone());
    model.into_active_model().soft_delete(&ctx.db).await?;
    audit(&ctx, entity_pid, "deleted", caller.actor(), None).await;
    streaming::publish(EventKind::Deleted, &entity_pid.to_string(), &title);
    format::empty_json()
}

/// List active cases (capped at 100).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = CaseModel::list(&ctx.db, 100).await?;
    let refs: Vec<CaseRef> = rows.iter().map(CaseRef::of).collect();
    format::json(refs)
}

/// Case-insensitive title search: `GET /api/cases/search?q=housing`.
/// Pragmatic Postgres `ILIKE` over the denormalised `title` (cap 50);
/// full-text / fuzzy search is deferred (spec §13 T-6).
#[debug_handler]
async fn search(
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let rows = CaseModel::search(&ctx.db, q.trim(), 50).await?;
    let refs: Vec<CaseRef> = rows.iter().map(CaseRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored cases that match the query above the threshold.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<Case>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = CaseModel::list(&ctx.db, CHECK_DUPLICATES_SCAN_CAP).await?;
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "check-duplicates in-memory scan hit its row cap; results may be \
             incomplete until search-backed candidate blocking lands (spec §13 T-6)"
        );
    }
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_case()?;
        let r = engine.match_cases(&query, &candidate);
        if r.is_match {
            hits.push(ScoredRef {
                pid: row.pid.to_string(),
                title: row.title.clone(),
                score: r.score,
                confidence: format!("{:?}", r.confidence),
                is_match: r.is_match,
            });
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    format::json(hits)
}

/// Merge a confirmed-duplicate case into a surviving (main) case:
/// union the duplicate's data into main, keep the duplicate's title as an
/// alternate title, soft-delete the duplicate, record the merge history,
/// and publish a `Merged` event (plus a `Deleted` for the duplicate).
#[debug_handler]
async fn merge(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<MergeRequest>,
) -> Result<Response> {
    if req.main_pid == req.duplicate_pid {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("validation", "main_pid and duplicate_pid must differ"),
        ));
    }
    let main = CaseModel::find_by_pid(&ctx.db, &req.main_pid).await?;
    let duplicate = CaseModel::find_by_pid(&ctx.db, &req.duplicate_pid).await?;

    let outcome = merge_cases(&main.to_case()?, &duplicate.to_case()?);

    // Update the survivor, then retire the duplicate.
    let merged = main
        .into_active_model()
        .update_data(&ctx.db, &outcome.merged)
        .await?;
    let (dup_pid, dup_title) = (duplicate.pid, duplicate.title.clone());
    duplicate.into_active_model().soft_delete(&ctx.db).await?;

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
        tracing::warn!(error = %err, "failed to write merge record");
    }
    audit(
        &ctx,
        merged.pid,
        "merged",
        caller.actor(),
        Some(merged.data.clone()),
    )
    .await;
    audit(&ctx, dup_pid, "merged_into", caller.actor(), None).await;
    streaming::publish(EventKind::Merged, &merged.pid.to_string(), &merged.title);
    streaming::publish(EventKind::Deleted, &dup_pid.to_string(), &dup_title);

    format::json(serde_json::json!({
        "main_pid": merged.pid.to_string(),
        "duplicate_pid": dup_pid.to_string(),
        "main": merged.to_case()?,
    }))
}

/// Recent merge-history records, newest first.
#[debug_handler]
async fn recent_merges(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Recent audit-log entries across all cases.
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single case.
#[debug_handler]
async fn entity_audit(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::for_entity(&ctx.db, uuid).await?;
    format::json(rows)
}

/// Recent events from the in-memory event stream.
#[debug_handler]
async fn recent_events() -> Result<Response> {
    format::json(streaming::recent(100))
}

/// Echo the verified claims of the bearer token — `401` when the token is
/// missing or fails verification. Proves peer JWT verification against
/// the auth-service JWKS end to end (spec §13 T-7).
#[debug_handler]
async fn whoami(AuthUser(claims): AuthUser) -> Result<Response> {
    format::json(claims)
}

/// Build the `/api/cases` route table (CRUD + match +
/// check-duplicates + audit / event endpoints).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/cases")
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the OQ-1 / T-2 decision: blank-title validation failure is
    /// `422 Unprocessable Entity` (family convention), not `400`.
    /// Runs without a database, so the pin holds on default `cargo test`.
    #[test]
    fn blank_title_validation_returns_422() {
        for title in ["", "   ", "\t\n"] {
            let err = validate(&Case::new(title)).expect_err("blank title must fail");
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
    fn blank_title_validation_response_status_is_422() {
        let err = validate(&Case::new("")).expect_err("blank title must fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn non_blank_title_passes_validation() {
        assert!(validate(&Case::new("Housing benefit appeal")).is_ok());
    }

    /// A malformed `opened_date` is a validation failure surfaced as
    /// `422`, the same status as a blank title. Runs without a database.
    #[test]
    fn malformed_opened_date_returns_422() {
        let case = Case {
            opened_date: Some("2024-13-99".to_string()),
            ..Case::new("Housing benefit appeal")
        };
        let err = validate(&case).expect_err("malformed date must fail");
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
