//! Plan CRUD + matching endpoints over the single recursive `plans`
//! collection.
//!
//! The API DTO is `project_portfolio_management_matcher::Plan` itself — the service
//! stores it verbatim (as JSONB) and matches with the canonical
//! `project-portfolio-management-matcher` engine, so there is no separate model or adapter
//! to drift. The former kinds (Portfolio / Project / Product / Program /
//! Practice / Process / Purpose / Pathway) were unified into one
//! recursive tree: any plan may contain
//! any other via `parent_ref`, `kind` is an optional descriptive label,
//! and matching is not gated by kind.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use project_portfolio_management_matcher::{MatchConfig, MatchingEngine, Plan, PlanKind};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthUser, MaybeAuthUser};
use crate::merge::merge_plans;
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::models::plans::Model as PlanModel;
use crate::streaming;

/// Maximum number of stored rows scanned in-memory by `check-duplicates`.
/// No search-backed candidate blocking yet (deferred — spec §13); the
/// handler `WARN`s if the scan reaches this cap.
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// Parse an optional descriptive `kind` label from a string, accepting
/// both the singular `PlanKind` variant name (`"Project"`) and the legacy
/// plural collection segment (`"projects"`). `None` for anything else —
/// `kind` is optional metadata, so an unrecognised value simply means "no
/// kind".
#[must_use]
pub fn parse_kind_label(s: &str) -> Option<PlanKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "portfolio" | "portfolios" => Some(PlanKind::Portfolio),
        "project" | "projects" => Some(PlanKind::Project),
        "product" | "products" => Some(PlanKind::Product),
        "program" | "programs" => Some(PlanKind::Program),
        "practice" | "practices" => Some(PlanKind::Practice),
        "process" | "processes" => Some(PlanKind::Process),
        "purpose" | "purposes" => Some(PlanKind::Purpose),
        "pathway" | "pathways" => Some(PlanKind::Pathway),
        "proposal" | "proposals" => Some(PlanKind::Proposal),
        _ => None,
    }
}

/// Validate an incoming `Plan` payload.
///
/// Returns `422` (family convention) listing every field-level problem at
/// once (see [`crate::validation`]). `kind` is optional descriptive
/// metadata and is no longer cross-checked against a collection.
///
/// # Errors
///
/// `422` when any validation rule fails.
pub fn validate(wi: &Plan) -> Result<()> {
    let problems = crate::validation::problems(wi);
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", &problems.join("; ")),
    ))
}

/// Reject a `parent_ref` that would create a containment cycle: the new
/// parent must not be the plan itself, nor any of the plan's own
/// descendants (i.e. the plan must not appear among the parent's
/// ancestors). A no-op when the payload has no valid-UUID `parent_ref`.
///
/// `self_pid` is the plan being created/updated (`None` on create, since
/// a fresh pid cannot yet be anyone's ancestor).
///
/// # Errors
///
/// `422` when the edge would introduce a cycle.
async fn check_no_cycle(ctx: &AppContext, self_pid: Option<uuid::Uuid>, wi: &Plan) -> Result<()> {
    let Some(parent) = wi
        .parent_ref
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s.trim()).ok())
    else {
        return Ok(());
    };
    if let Some(self_pid) = self_pid {
        if parent == self_pid {
            return cycle_error();
        }
        // Walk the parent's ancestor chain; if we are in it, this edge
        // closes a loop.
        let ancestors = PlanModel::ancestor_pids(&ctx.db, parent)
            .await
            .map_err(super::model_not_found)?;
        if ancestors.contains(&self_pid) {
            return cycle_error();
        }
    }
    Ok(())
}

/// The shared `422` for a containment cycle.
fn cycle_error() -> Result<()> {
    Err(Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", "parent_ref would create a containment cycle"),
    ))
}

/// A lightweight reference to a stored plan: its public id and name.
#[derive(Debug, Serialize)]
struct PlanRef {
    /// The plan's public id (UUID v4), as a string.
    pid: String,
    /// The plan's denormalised name.
    name: String,
}

impl PlanRef {
    /// Project a stored [`PlanModel`] down to its `{pid, name}` ref.
    fn of(m: &PlanModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            name: m.name.clone(),
        }
    }
}

/// Body of `POST /{collection}/match`: a query plus the explicit
/// candidate list to rank it against (no persistence is touched).
#[derive(Debug, Deserialize)]
struct MatchRequest {
    /// The plan to score against each candidate.
    query: Plan,
    /// The candidate plans to rank.
    candidates: Vec<Plan>,
}

/// Query string for `GET /plans`: an optional parent roll-up filter.
#[derive(Debug, Deserialize)]
struct ListParams {
    /// When set, list only the direct children of this parent plan.
    #[serde(default)]
    parent: Option<String>,
    /// Page size; absent ⇒ [`LIST_DEFAULT_LIMIT`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

// ─── Pagination (agents/share/restful.md) ───────────────────────────────

/// Default page size for `GET /api/plans` — the cap this endpoint
/// applied before pagination.
pub const LIST_DEFAULT_LIMIT: u64 = 100;

/// Default page size for `GET /api/plans/search`.
pub const SEARCH_DEFAULT_LIMIT: u64 = 50;

/// Largest page any collection read will serve; a bigger `limit` is
/// clamped rather than refused.
pub const MAX_LIMIT: u64 = 500;

/// Largest accepted `offset`; past this a request is a `400` (SEC-G7).
pub const MAX_OFFSET: u64 = 10_000;

/// `?limit=` / `?offset=` on a collection read. Declared inline on each
/// query struct rather than `#[serde(flatten)]`-ed: a flattened struct
/// deserializes from a string-keyed map, so `limit=2` arrives as the
/// string `"2"` and fails to parse as a `u64`.
#[derive(Debug, Default, Clone, Copy)]
struct Page {
    /// Page size; `None`/zero ⇒ the endpoint default.
    limit: Option<u64>,
    /// Rows to skip; `None` ⇒ 0.
    offset: Option<u64>,
}

impl Page {
    /// The clamped `(limit, offset)` this request will use.
    fn resolve(self, default_limit: u64) -> (u64, u64) {
        let limit = self
            .limit
            .filter(|l| *l > 0)
            .unwrap_or(default_limit)
            .min(MAX_LIMIT);
        (limit, self.offset.unwrap_or(0))
    }

    /// Reject an out-of-bound offset before it reaches the database.
    fn check_offset(self) -> Result<()> {
        if self.offset.unwrap_or(0) > MAX_OFFSET {
            return Err(Error::CustomError(
                StatusCode::BAD_REQUEST,
                ErrorDetail::new(
                    "offset_too_large",
                    &format!("offset must not exceed {MAX_OFFSET}; narrow the query instead"),
                ),
            ));
        }
        Ok(())
    }
}

/// Stamp `X-Total-Count` / `X-Limit` / `X-Offset` onto a response.
fn with_page_headers(mut response: Response, total: u64, limit: u64, offset: u64) -> Response {
    let headers = response.headers_mut();
    for (name, value) in [
        ("x-total-count", total),
        ("x-limit", limit),
        ("x-offset", offset),
    ] {
        if let Ok(value) = axum::http::HeaderValue::from_str(&value.to_string()) {
            headers.insert(name, value);
        }
    }
    response
}

/// Query string for `GET /plans/search`: the `q` name term.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// Page size; absent ⇒ [`SEARCH_DEFAULT_LIMIT`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
    /// The case-insensitive substring to match against `name`.
    q: Option<String>,
}

/// Body of `POST /{collection}/merge`: which duplicate folds into which
/// survivor, and (optionally) why.
#[derive(Debug, Deserialize)]
struct MergeRequest {
    /// The surviving plan's public id.
    main_pid: String,
    /// The duplicate to merge in and soft-delete.
    duplicate_pid: String,
    /// Optional operator-supplied reason, recorded in the merge history.
    #[serde(default)]
    reason: Option<String>,
}

/// A scored `check-duplicates` hit.
#[derive(Debug, Serialize)]
struct ScoredRef {
    /// The matched plan's public id (UUID v4), as a string.
    pid: String,
    /// The matched plan's denormalised name.
    name: String,
    /// The match score in `[0.0, 1.0]`.
    score: f64,
    /// The confidence band (the matcher's `Confidence`, `Debug`-formatted).
    confidence: String,
    /// Whether the score cleared the match threshold (always `true` here).
    is_match: bool,
}

/// Descending-by-score comparator for `check-duplicates` ranking. A NaN
/// score (not produced in practice) is treated as `Equal`, never panics.
fn score_desc(a: f64, b: f64) -> std::cmp::Ordering {
    b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
}

/// Create a plan. `POST /api/plans`.
///
/// # Errors
///
/// `422` on validation failure or a containment cycle.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(wi): Json<Plan>,
) -> Result<Response> {
    validate(&wi)?;
    check_no_cycle(&ctx, None, &wi).await?;
    let model = streaming::create_and_emit(&ctx.db, &wi, caller.actor()).await?;
    // Audit is written inside `create_and_emit` (see `streaming`).
    Metrics::global().plan_created_total.inc();
    format::json(PlanRef::of(&model))
}

/// Fetch a plan by public id. `GET /api/plans/{pid}`.
///
/// # Errors
///
/// `404` unknown pid.
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = PlanModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    format::json(model.to_plan()?)
}

/// Replace a plan's payload. `PUT /api/plans/{pid}`.
///
/// # Errors
///
/// `404` unknown pid; `422` on validation failure or a containment cycle.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(wi): Json<Plan>,
) -> Result<Response> {
    validate(&wi)?;
    let model = PlanModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    check_no_cycle(&ctx, Some(model.pid), &wi).await?;
    // Record-level ABAC (PPM-3): the item's phase-gate stage is a
    // `resource.stage` attribute, so "deny write past gate N unless
    // access=admin" is a policy statement. A no-op while enforcement
    // is off.
    crate::auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &crate::auth::plan_resource_attrs(model.stage.as_deref()),
    )
    .map_err(super::record_rejection)?;
    let updated = streaming::update_and_emit(&ctx.db, model, &wi, caller.actor()).await?;
    // Audit is written inside `update_and_emit` (see `streaming`).
    Metrics::global().plan_updated_total.inc();
    format::json(PlanRef::of(&updated))
}

/// Soft-delete a plan. `DELETE /api/plans/{pid}`.
///
/// # Errors
///
/// `404` unknown pid.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = PlanModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    let (_entity_pid, _name) = streaming::delete_and_emit(&ctx.db, model, caller.actor()).await?;
    // Audit is written inside `delete_and_emit` (see `streaming`).
    Metrics::global().plan_deleted_total.inc();
    format::empty_json()
}

/// List active plans (cap 100). Accepts `?parent=<pid>` to roll up one
/// plan's direct children. `GET /api/plans`.
///
/// # Errors
///
/// A DB error when the query fails.
#[debug_handler]
async fn list(
    axum::extract::Query(params): axum::extract::Query<ListParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(LIST_DEFAULT_LIMIT);
    // The parent scope is applied to the page and the count alike, so the
    // total always describes the rows actually being paged over.
    let parent = params
        .parent
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    let rows = PlanModel::list_paged(&ctx.db, parent, limit, offset).await?;
    let total = PlanModel::count_for(&ctx.db, parent).await?;
    let refs: Vec<PlanRef> = rows.iter().map(PlanRef::of).collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
}

/// Case-insensitive name search (Postgres `ILIKE`, cap 50).
/// `GET /api/plans/search?q=`.
///
/// # Errors
///
/// `400` when `q` is missing or blank.
#[debug_handler]
async fn search(
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(SEARCH_DEFAULT_LIMIT);
    let rows = PlanModel::search_paged(&ctx.db, q.trim(), limit, offset).await?;
    let total = PlanModel::search_count(&ctx.db, q.trim()).await?;
    let refs: Vec<PlanRef> = rows.iter().map(PlanRef::of).collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
}

/// Score a query against an explicit candidate list (no persistence).
/// `POST /api/plans/match`.
///
/// # Errors
///
/// Never (infallible matcher); returns the ranked results.
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored plans matching the query above the threshold.
/// `POST /api/plans/check-duplicates`.
///
/// # Errors
///
/// A DB error when the row scan fails.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<Plan>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = PlanModel::list(&ctx.db, CHECK_DUPLICATES_SCAN_CAP).await?;
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "check-duplicates in-memory scan hit its row cap; results may be \
             incomplete until search-backed candidate blocking lands (spec §13)"
        );
    }
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_plan()?;
        let r = engine.match_plans(&query, &candidate);
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
    hits.sort_by(|a, b| score_desc(a.score, b.score));
    format::json(hits)
}

/// Merge a confirmed-duplicate plan into a surviving one.
/// `POST /api/plans/merge`.
///
/// # Errors
///
/// `404` unknown pid; `422` self-merge.
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
    let main = PlanModel::find_by_pid(&ctx.db, &req.main_pid)
        .await
        .map_err(super::model_not_found)?;
    let duplicate = PlanModel::find_by_pid(&ctx.db, &req.duplicate_pid)
        .await
        .map_err(super::model_not_found)?;

    let outcome = merge_plans(&main.to_plan()?, &duplicate.to_plan()?);

    let (merged, dup_pid, _dup_name) =
        streaming::merge_and_emit(&ctx.db, main, duplicate, &outcome.merged, caller.actor())
            .await?;

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
    // The two audit entries (survivor "merged", duplicate "merged_into")
    // are written inside `merge_and_emit` (see `streaming`).
    Metrics::global().plan_merged_total.inc();

    format::json(serde_json::json!({
        "main_pid": merged.pid.to_string(),
        "duplicate_pid": dup_pid.to_string(),
        "main": merged.to_plan()?,
    }))
}

/// Recent merge-history records (cap 100). `GET /api/plans/merges/recent`.
///
/// # Errors
///
/// A DB error when the query fails.
#[debug_handler]
async fn recent_merges(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Recent audit entries (cap 100). `GET /api/plans/audit/recent`.
///
/// # Errors
///
/// A DB error when the query fails.
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single plan. `GET /api/plans/{pid}/audit`.
///
/// # Errors
///
/// `400` when `pid` is not a UUID.
#[debug_handler]
async fn entity_audit(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::for_entity(&ctx.db, uuid).await?;
    format::json(rows)
}

/// Recent events (cap 100), from whichever transport is active — the
/// in-memory ring buffer (`memory`) or the `event_outbox` table
/// (`outbox`); the wire shape is identical. `GET /api/plans/events/recent`.
///
/// # Errors
///
/// A DB error when the outbox query fails.
#[debug_handler]
async fn recent_events(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(streaming::recent_events(&ctx.db, 100).await?)
}

/// Echo the verified bearer-token claims. `GET /api/plans/whoami`;
/// `401` without a valid token.
///
/// # Errors
///
/// `401` missing/invalid token.
#[debug_handler]
async fn whoami(AuthUser(claims): AuthUser) -> Result<Response> {
    format::json(claims)
}

/// Build the `/api/plans` route table (CRUD + match + check-duplicates +
/// merge + audit / events + whoami) over the single recursive collection.
///
/// Order matters: the literal sub-paths (`/search`, `/match`, …) sit
/// before the `/{pid}` catch-alls so they are not shadowed.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/plans")
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
    use project_portfolio_management_matcher::Plan;

    /// Pins the validation status: a blank name is `422`.
    #[test]
    fn blank_name_validation_returns_422() {
        let wi = Plan::new("");
        let err = validate(&wi).expect_err("blank name must fail");
        match err {
            Error::CustomError(status, _) => assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY),
            other => panic!("expected CustomError(422), got {other:?}"),
        }
    }

    /// The complement: a real name passes (kind is optional now).
    #[test]
    fn valid_name_passes() {
        let wi = Plan::new("Apollo platform migration");
        assert!(validate(&wi).is_ok());
    }

    /// Every `PlanKind` label parses from both its singular variant
    /// name and its legacy plural collection segment, case-insensitively.
    #[test]
    fn parse_kind_label_accepts_every_kind_singular_and_plural() {
        let cases = [
            ("portfolio", "portfolios", PlanKind::Portfolio),
            ("project", "projects", PlanKind::Project),
            ("product", "products", PlanKind::Product),
            ("program", "programs", PlanKind::Program),
            ("practice", "practices", PlanKind::Practice),
            ("process", "processes", PlanKind::Process),
            ("purpose", "purposes", PlanKind::Purpose),
            ("pathway", "pathways", PlanKind::Pathway),
            ("proposal", "proposals", PlanKind::Proposal),
        ];
        for (singular, plural, want) in cases {
            assert_eq!(parse_kind_label(singular), Some(want), "{singular}");
            assert_eq!(parse_kind_label(plural), Some(want), "{plural}");
            assert_eq!(
                parse_kind_label(&format!("  {} ", singular.to_uppercase())),
                Some(want),
                "{singular} (trimmed / upper)"
            );
        }
        assert_eq!(parse_kind_label("plan"), None);
        assert_eq!(parse_kind_label(""), None);
    }

    /// Pins the documented scan cap.
    #[test]
    fn check_duplicates_scan_cap_is_the_documented_value() {
        assert_eq!(CHECK_DUPLICATES_SCAN_CAP, 1000);
    }

    /// Pins the ranking comparator: best score first; NaN treated equal.
    #[test]
    fn score_desc_orders_and_handles_nan() {
        use std::cmp::Ordering;
        assert_eq!(score_desc(0.9, 0.4), Ordering::Less);
        assert_eq!(score_desc(0.4, 0.9), Ordering::Greater);
        assert_eq!(score_desc(0.5, 0.5), Ordering::Equal);
        assert_eq!(score_desc(f64::NAN, 0.5), Ordering::Equal);
    }
}
