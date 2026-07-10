//! Work-item CRUD + matching endpoints, shared across the four
//! collections (portfolios / projects / products / programs).
//!
//! The API DTO is `portfolio_matcher::WorkItem` itself — the service
//! stores it verbatim (as JSONB) and matches with the canonical
//! `portfolio-matcher` engine, so there is no separate model or adapter
//! to drift. A single parameterised controller serves all four
//! collections: the leading `{collection}` path segment selects the
//! `kind`, and every query, match, and merge is scoped to that one
//! collection (the matcher's kind gate is the underlying guarantee —
//! matching never crosses collections).

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use portfolio_matcher::{MatchConfig, MatchingEngine, WorkItem, WorkItemKind};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthUser, MaybeAuthUser};
use crate::merge::merge_work_items;
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::models::work_items::Model as WorkItemModel;
use crate::streaming;

/// Maximum number of stored rows scanned in-memory by `check-duplicates`
/// (per collection). No search-backed candidate blocking yet (deferred —
/// spec §13); the handler `WARN`s if the scan reaches this cap.
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// One of the four work-item collections. The leading path segment
/// (`portfolios` / `projects` / `products` / `programs`) maps to a
/// `WorkItemKind`, which is the row's `kind` column and the matcher's
/// hard gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collection {
    /// The `portfolios` collection (umbrella; `Portfolio` kind).
    Portfolios,
    /// The `projects` collection (`Project` kind).
    Projects,
    /// The `products` collection (`Product` kind).
    Products,
    /// The `programs` collection (`Program` kind).
    Programs,
}

impl Collection {
    /// Map a leading path segment to a collection, or `None` for an
    /// unknown segment (the handler turns that into a `404`).
    #[must_use]
    pub fn from_segment(seg: &str) -> Option<Self> {
        match seg {
            "portfolios" => Some(Self::Portfolios),
            "projects" => Some(Self::Projects),
            "products" => Some(Self::Products),
            "programs" => Some(Self::Programs),
            _ => None,
        }
    }

    /// The `WorkItemKind` this collection stores.
    #[must_use]
    pub fn kind(self) -> WorkItemKind {
        match self {
            Self::Portfolios => WorkItemKind::Portfolio,
            Self::Projects => WorkItemKind::Project,
            Self::Products => WorkItemKind::Product,
            Self::Programs => WorkItemKind::Program,
        }
    }

    /// The `kind` column value (the serde name of [`Collection::kind`]).
    #[must_use]
    pub fn kind_str(self) -> &'static str {
        match self {
            Self::Portfolios => "Portfolio",
            Self::Projects => "Project",
            Self::Products => "Product",
            Self::Programs => "Program",
        }
    }
}

/// Resolve a path segment to a [`Collection`], or a `404` error.
fn resolve(seg: &str) -> Result<Collection> {
    Collection::from_segment(seg).ok_or_else(|| {
        Error::CustomError(
            StatusCode::NOT_FOUND,
            ErrorDetail::new("not_found", "unknown collection"),
        )
    })
}

/// Validate an incoming `WorkItem` payload against its collection.
///
/// Returns `422` (family convention) listing every problem at once: the
/// field-level rules in [`crate::validation`] plus the cross-check that
/// the body's `kind` matches the collection it was posted to.
///
/// # Errors
///
/// `422` when any validation rule fails.
pub fn validate(collection: Collection, wi: &WorkItem) -> Result<()> {
    let mut problems = crate::validation::problems(wi);
    if wi.kind != collection.kind() {
        problems.push(format!(
            "kind must be {} for the {} collection",
            collection.kind_str(),
            collection.kind_str()
        ));
    }
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", &problems.join("; ")),
    ))
}

/// A lightweight reference to a stored work item: its public id and name.
#[derive(Debug, Serialize)]
struct WorkItemRef {
    /// The work item's public id (UUID v4), as a string.
    pid: String,
    /// The work item's denormalised name.
    name: String,
}

impl WorkItemRef {
    /// Project a stored [`WorkItemModel`] down to its `{pid, name}` ref.
    fn of(m: &WorkItemModel) -> Self {
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
    /// The work item to score against each candidate.
    query: WorkItem,
    /// The candidate work items to rank.
    candidates: Vec<WorkItem>,
}

/// Query string for `GET /{collection}`: an optional parent-portfolio
/// roll-up filter for the child collections.
#[derive(Debug, Deserialize)]
struct ListParams {
    /// When set (child collections), list only children of this portfolio.
    #[serde(default)]
    portfolio: Option<String>,
}

/// Query string for `GET /{collection}/search`: the `q` name term.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// The case-insensitive substring to match against `name`.
    q: Option<String>,
}

/// Body of `POST /{collection}/merge`: which duplicate folds into which
/// survivor, and (optionally) why.
#[derive(Debug, Deserialize)]
struct MergeRequest {
    /// The surviving work item's public id.
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
    /// The matched work item's public id (UUID v4), as a string.
    pid: String,
    /// The matched work item's denormalised name.
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

/// Create a work item in `{collection}`. `POST /api/{collection}`.
///
/// # Errors
///
/// `404` unknown collection; `422` on validation failure (incl. a `kind`
/// that does not match the collection).
#[debug_handler]
async fn create(
    Path(collection): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(wi): Json<WorkItem>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    validate(collection, &wi)?;
    let model =
        streaming::create_and_emit(&ctx.db, collection.kind_str(), &wi, caller.actor()).await?;
    // Audit is written inside `create_and_emit` (see `streaming`).
    Metrics::global().work_item_created_total.inc();
    format::json(WorkItemRef::of(&model))
}

/// Fetch a work item by public id. `GET /api/{collection}/{pid}`.
///
/// # Errors
///
/// `404` unknown collection or pid.
#[debug_handler]
async fn get_one(
    Path((collection, pid)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    let model = WorkItemModel::find_by_pid(&ctx.db, collection.kind_str(), &pid).await?;
    format::json(model.to_work_item()?)
}

/// Replace a work item's payload. `PUT /api/{collection}/{pid}`.
///
/// # Errors
///
/// `404` unknown collection or pid; `422` on validation failure.
#[debug_handler]
async fn update(
    Path((collection, pid)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(wi): Json<WorkItem>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    validate(collection, &wi)?;
    let model = WorkItemModel::find_by_pid(&ctx.db, collection.kind_str(), &pid).await?;
    let updated = streaming::update_and_emit(&ctx.db, model, &wi, caller.actor()).await?;
    // Audit is written inside `update_and_emit` (see `streaming`).
    Metrics::global().work_item_updated_total.inc();
    format::json(WorkItemRef::of(&updated))
}

/// Soft-delete a work item. `DELETE /api/{collection}/{pid}`.
///
/// # Errors
///
/// `404` unknown collection or pid.
#[debug_handler]
async fn remove(
    Path((collection, pid)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    let model = WorkItemModel::find_by_pid(&ctx.db, collection.kind_str(), &pid).await?;
    let (_entity_pid, _name) = streaming::delete_and_emit(&ctx.db, model, caller.actor()).await?;
    // Audit is written inside `delete_and_emit` (see `streaming`).
    Metrics::global().work_item_deleted_total.inc();
    format::empty_json()
}

/// List active work items in `{collection}` (cap 100). Child collections
/// accept `?portfolio=<pid>` to roll up one portfolio's children.
/// `GET /api/{collection}`.
///
/// # Errors
///
/// `404` unknown collection; a DB error when the query fails.
#[debug_handler]
async fn list(
    Path(collection): Path<String>,
    axum::extract::Query(params): axum::extract::Query<ListParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    let rows = match params.portfolio.as_deref() {
        Some(parent) if !parent.trim().is_empty() => {
            WorkItemModel::list_children(&ctx.db, collection.kind_str(), parent.trim(), 100).await?
        }
        _ => WorkItemModel::list(&ctx.db, collection.kind_str(), 100).await?,
    };
    let refs: Vec<WorkItemRef> = rows.iter().map(WorkItemRef::of).collect();
    format::json(refs)
}

/// Case-insensitive name search within a collection (Postgres `ILIKE`,
/// cap 50). `GET /api/{collection}/search?q=`.
///
/// # Errors
///
/// `404` unknown collection; `400` when `q` is missing or blank.
#[debug_handler]
async fn search(
    Path(collection): Path<String>,
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let rows = WorkItemModel::search(&ctx.db, collection.kind_str(), q.trim(), 50).await?;
    let refs: Vec<WorkItemRef> = rows.iter().map(WorkItemRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
/// `POST /api/{collection}/match`. The matcher's kind gate makes any
/// cross-kind candidate score `0.0`.
///
/// # Errors
///
/// `404` unknown collection.
#[debug_handler]
async fn match_against(
    Path(collection): Path<String>,
    Json(req): Json<MatchRequest>,
) -> Result<Response> {
    let _ = resolve(&collection)?;
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored work items in `{collection}` matching the query above the
/// threshold. `POST /api/{collection}/check-duplicates`.
///
/// # Errors
///
/// `404` unknown collection; a DB error when the row scan fails.
#[debug_handler]
async fn check_duplicates(
    Path(collection): Path<String>,
    State(ctx): State<AppContext>,
    Json(query): Json<WorkItem>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows =
        WorkItemModel::list(&ctx.db, collection.kind_str(), CHECK_DUPLICATES_SCAN_CAP).await?;
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "check-duplicates in-memory scan hit its row cap; results may be \
             incomplete until search-backed candidate blocking lands (spec §13)"
        );
    }
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_work_item()?;
        let r = engine.match_work_items(&query, &candidate);
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

/// Merge a confirmed-duplicate work item into a surviving one (within the
/// same collection). `POST /api/{collection}/merge`.
///
/// # Errors
///
/// `404` unknown collection or pid; `422` self-merge.
#[debug_handler]
async fn merge(
    Path(collection): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<MergeRequest>,
) -> Result<Response> {
    let collection = resolve(&collection)?;
    if req.main_pid == req.duplicate_pid {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("validation", "main_pid and duplicate_pid must differ"),
        ));
    }
    let kind = collection.kind_str();
    let main = WorkItemModel::find_by_pid(&ctx.db, kind, &req.main_pid).await?;
    let duplicate = WorkItemModel::find_by_pid(&ctx.db, kind, &req.duplicate_pid).await?;

    let outcome = merge_work_items(&main.to_work_item()?, &duplicate.to_work_item()?);

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
    Metrics::global().work_item_merged_total.inc();

    format::json(serde_json::json!({
        "main_pid": merged.pid.to_string(),
        "duplicate_pid": dup_pid.to_string(),
        "main": merged.to_work_item()?,
    }))
}

/// Recent merge-history records (cap 100). `GET
/// /api/{collection}/merges/recent`.
///
/// # Errors
///
/// `404` unknown collection; a DB error when the query fails.
#[debug_handler]
async fn recent_merges(
    Path(collection): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let _ = resolve(&collection)?;
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Recent audit entries (cap 100). `GET
/// /api/{collection}/audit/recent`.
///
/// # Errors
///
/// `404` unknown collection; a DB error when the query fails.
#[debug_handler]
async fn recent_audit(
    Path(collection): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let _ = resolve(&collection)?;
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single work item. `GET
/// /api/{collection}/{pid}/audit`.
///
/// # Errors
///
/// `404` unknown collection; `400` when `pid` is not a UUID.
#[debug_handler]
async fn entity_audit(
    Path((collection, pid)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let _ = resolve(&collection)?;
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::for_entity(&ctx.db, uuid).await?;
    format::json(rows)
}

/// Recent events (cap 100), from whichever transport is active — the
/// in-memory ring buffer (`memory`) or the `event_outbox` table
/// (`outbox`); the wire shape is identical. `GET
/// /api/{collection}/events/recent`.
///
/// # Errors
///
/// `404` unknown collection; a DB error when the outbox query fails.
#[debug_handler]
async fn recent_events(
    Path(collection): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let _ = resolve(&collection)?;
    format::json(streaming::recent_events(&ctx.db, 100).await?)
}

/// Echo the verified bearer-token claims. `GET
/// /api/{collection}/whoami`; `401` without a valid token.
///
/// # Errors
///
/// `404` unknown collection; `401` missing/invalid token.
#[debug_handler]
async fn whoami(Path(collection): Path<String>, AuthUser(claims): AuthUser) -> Result<Response> {
    let _ = resolve(&collection)?;
    format::json(claims)
}

/// Build the `/api/{collection}` route table (CRUD + match +
/// check-duplicates + merge + audit / events + whoami), shared across all
/// four collections via the leading `{collection}` segment.
///
/// Order matters: the literal sub-paths (`/search`, `/match`, …) sit
/// before the `/{pid}` catch-alls so they are not shadowed.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/{collection}", post(create))
        .add("/{collection}", get(list))
        .add("/{collection}/search", get(search))
        .add("/{collection}/match", post(match_against))
        .add("/{collection}/check-duplicates", post(check_duplicates))
        .add("/{collection}/merge", post(merge))
        .add("/{collection}/merges/recent", get(recent_merges))
        .add("/{collection}/whoami", get(whoami))
        .add("/{collection}/audit/recent", get(recent_audit))
        .add("/{collection}/events/recent", get(recent_events))
        .add("/{collection}/{pid}", get(get_one))
        .add("/{collection}/{pid}", put(update))
        .add("/{collection}/{pid}", delete(remove))
        .add("/{collection}/{pid}/audit", get(entity_audit))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the collection-segment mapping (and the `404` for unknown).
    #[test]
    fn collection_segments_map_to_kinds() {
        assert_eq!(
            Collection::from_segment("portfolios").map(Collection::kind),
            Some(WorkItemKind::Portfolio)
        );
        assert_eq!(
            Collection::from_segment("projects").map(Collection::kind),
            Some(WorkItemKind::Project)
        );
        assert_eq!(
            Collection::from_segment("products").map(Collection::kind),
            Some(WorkItemKind::Product)
        );
        assert_eq!(
            Collection::from_segment("programs").map(Collection::kind),
            Some(WorkItemKind::Program)
        );
        assert!(Collection::from_segment("widgets").is_none());
    }

    /// Pins the validation status: a blank name is `422`.
    #[test]
    fn blank_name_validation_returns_422() {
        let wi = WorkItem::new(WorkItemKind::Project, "");
        let err = validate(Collection::Projects, &wi).expect_err("blank name must fail");
        match err {
            Error::CustomError(status, _) => assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY),
            other => panic!("expected CustomError(422), got {other:?}"),
        }
    }

    /// Pins the kind cross-check: a `Product` body posted to the projects
    /// collection is a `422`.
    #[test]
    fn mismatched_kind_returns_422() {
        let wi = WorkItem::new(WorkItemKind::Product, "Apollo");
        let err = validate(Collection::Projects, &wi).expect_err("kind mismatch must fail");
        assert_eq!(
            err.into_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    /// The complement: a matching kind + real name passes.
    #[test]
    fn matching_kind_and_name_passes() {
        let wi = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
        assert!(validate(Collection::Projects, &wi).is_ok());
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
