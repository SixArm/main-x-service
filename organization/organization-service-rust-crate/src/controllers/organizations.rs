//! Organization CRUD + matching endpoints.
//!
//! The API DTO is `organization_matcher::Organization` itself — the
//! service stores it verbatim (as JSON) and matches with the canonical
//! `organization-matcher` engine, so there is no separate model or
//! adapter to drift.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use organization_matcher::{MatchConfig, MatchingEngine, Organization};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthUser, MaybeAuthUser};
use crate::merge::merge_orgs;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::models::organizations::Model as OrgModel;
use crate::streaming::{self, EventKind};

#[derive(Debug, Serialize)]
struct OrgRef {
    pid: String,
    name: String,
}

impl OrgRef {
    fn of(m: &OrgModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            name: m.name.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MatchRequest {
    query: Organization,
    candidates: Vec<Organization>,
}

#[derive(Debug, Deserialize)]
struct MergeRequest {
    /// The surviving organization's public id.
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
    name: String,
    score: f64,
    confidence: String,
    is_match: bool,
}

/// Validate an incoming `Organization` payload.
///
/// Family convention: validation failures are `422 Unprocessable
/// Entity` (loco's `bad_request` is reserved for malformed requests).
///
/// # Errors
///
/// `Error::CustomError(422)` when `name` is blank.
fn validate(org: &Organization) -> Result<()> {
    if org.name.trim().is_empty() {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("unprocessable_entity", "name is required"),
        ));
    }
    Ok(())
}

/// Map model-layer errors to HTTP errors: an unknown `pid` is `404
/// Not Found` (loco's default mapping for `ModelError::EntityNotFound`
/// is a 500, which would break the spec'd `404` contract).
fn http_err(err: ModelError) -> Error {
    match err {
        ModelError::EntityNotFound => Error::NotFound,
        other => Error::Model(other),
    }
}

/// Create an organization.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(org): Json<Organization>,
) -> Result<Response> {
    validate(&org)?;
    let model = OrgModel::create(&ctx.db, &org).await?;
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
    format::json(OrgRef::of(&model))
}

/// Fetch an organization by public id.
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    let org = model.to_org()?;
    format::json(org)
}

/// Replace an organization's payload.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(org): Json<Organization>,
) -> Result<Response> {
    validate(&org)?;
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    let updated = model.into_active_model().update_data(&ctx.db, &org).await?;
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
    format::json(OrgRef::of(&updated))
}

/// Soft-delete an organization.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    let (entity_pid, name) = (model.pid, model.name.clone());
    model.into_active_model().soft_delete(&ctx.db).await?;
    audit(&ctx, entity_pid, "deleted", caller.actor(), None).await;
    streaming::publish_with_actor(
        EventKind::Deleted,
        &entity_pid.to_string(),
        &name,
        caller.actor(),
    );
    format::empty_json()
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

/// List active organizations (capped at 100).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = OrgModel::list(&ctx.db, 100).await?;
    let refs: Vec<OrgRef> = rows.iter().map(OrgRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Maximum number of stored organizations scanned per `check-duplicates`
/// request.
///
/// `check-duplicates` does an in-memory full scan: it loads up to this
/// many rows and scores the query against each. The cap bounds the
/// request's memory and latency, but it is a known scale cliff — beyond
/// this many active organizations the scan silently misses candidates.
/// When the scan returns exactly this many rows the handler logs a
/// `WARN` so the truncation is observable rather than silent. Lifting
/// the cap requires blocking / candidate pre-selection (spec §6 R-DUP,
/// task T-7); until then this constant is the single source of truth for
/// the limit and is asserted by tests.
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// Find stored organizations that match the query above the threshold.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<Organization>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = OrgModel::list(&ctx.db, CHECK_DUPLICATES_SCAN_CAP).await?;
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "check-duplicates scan hit the row cap; results may be truncated \
             (silent miss of candidates beyond the cap). See task T-7."
        );
    }
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_org()?;
        let r = engine.match_organizations(&query, &candidate);
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
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    format::json(hits)
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
}

/// Case-insensitive name search: `GET /api/organizations/search?q=acme`.
#[debug_handler]
async fn search(
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let rows = OrgModel::search(&ctx.db, q.trim(), 50).await?;
    let refs: Vec<OrgRef> = rows.iter().map(OrgRef::of).collect();
    format::json(refs)
}

/// Merge a confirmed-duplicate organization into a surviving (main)
/// organization: union the duplicate's data into main, keep the
/// duplicate's name as an alternate name, soft-delete the duplicate,
/// record the merge history, and publish a `Merged` event (plus a
/// `Deleted` for the duplicate).
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
    let main = OrgModel::find_by_pid(&ctx.db, &req.main_pid)
        .await
        .map_err(http_err)?;
    let duplicate = OrgModel::find_by_pid(&ctx.db, &req.duplicate_pid)
        .await
        .map_err(http_err)?;

    let outcome = merge_orgs(&main.to_org()?, &duplicate.to_org()?);

    let merged = main
        .into_active_model()
        .update_data(&ctx.db, &outcome.merged)
        .await?;
    let (dup_pid, dup_name) = (duplicate.pid, duplicate.name.clone());
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
        "main": merged.to_org()?,
    }))
}

/// Recent merge-history records, newest first.
#[debug_handler]
async fn recent_merges(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Echo the verified claims of the bearer token — `401` when the token is
/// missing or fails verification. Proves peer JWT verification against
/// the auth-service JWKS end to end (spec §13 T-9).
#[debug_handler]
async fn whoami(AuthUser(claims): AuthUser) -> Result<Response> {
    format::json(claims)
}

/// Recent audit-log entries across all organizations.
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single organization.
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

/// All organization routes, mounted under `/api/organizations`: CRUD,
/// name search, matching, duplicate-check, and audit / event endpoints.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/organizations")
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

    /// DB-free pin: blank-name validation must map to HTTP 422
    /// (family convention; spec §6/§9, entity spec T-2).
    #[test]
    fn blank_name_validation_is_422() {
        let org = Organization::new("   ");
        let err = validate(&org).expect_err("blank name must fail validation");
        match err {
            Error::CustomError(status, _) => {
                assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            }
            other => panic!("expected CustomError(422), got {other:?}"),
        }
    }

    #[test]
    fn non_blank_name_passes_validation() {
        let org = Organization::new("Acme, Inc.");
        assert!(validate(&org).is_ok());
    }

    /// DB-free pin for task T-7 (observable cap): the `check-duplicates`
    /// scan cap is the named constant the handler queries with, so a
    /// scan returning exactly this many rows is the truncation trigger.
    /// Pinning the value here keeps the constant the single source of
    /// truth (request-level truncation behaviour is exercised by the
    /// Postgres-gated suite).
    #[test]
    fn check_duplicates_scan_cap_is_pinned() {
        assert_eq!(CHECK_DUPLICATES_SCAN_CAP, 1000);
    }
}
