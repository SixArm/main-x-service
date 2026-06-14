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

/// Lightweight response shape: a stored organization reduced to its
/// public id and name. Returned by create/update/list/search so callers
/// get a stable handle without the full payload.
#[derive(Debug, Serialize)]
struct OrgRef {
    /// The organization's public id (UUID, as a string).
    pid: String,
    /// The organization's denormalised name.
    name: String,
}

impl OrgRef {
    /// Project a stored [`OrgModel`] row down to its `{pid, name}` ref.
    fn of(m: &OrgModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            name: m.name.clone(),
        }
    }
}

/// Request body for `POST /match`: a query plus an explicit candidate
/// list to rank it against (stateless — nothing is persisted).
#[derive(Debug, Deserialize)]
struct MatchRequest {
    /// The organization to score against each candidate.
    query: Organization,
    /// The candidate organizations to rank.
    candidates: Vec<Organization>,
}

/// Request body for `POST /merge`: which duplicate folds into which
/// survivor, plus an optional reason.
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

/// A duplicate-check hit: a stored organization that matched the query,
/// with its score and classification. Returned by `check-duplicates`.
#[derive(Debug, Serialize)]
struct ScoredRef {
    /// The matched organization's public id.
    pid: String,
    /// The matched organization's name.
    name: String,
    /// The match score in `[0.0, 1.0]` (higher is more similar).
    score: f64,
    /// The confidence band (matcher's `Confidence` enum, `Debug`-rendered).
    confidence: String,
    /// Whether the score cleared the match threshold.
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
///
/// `POST /api/organizations`. Body: an `Organization`. On success returns
/// `200` with an `OrgRef` (`{pid, name}`); a blank name is `422`. Writes
/// an audit row and publishes a `Created` event (both best-effort,
/// stamped with the caller `actor` when a token was presented).
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
///
/// `GET /api/organizations/{pid}`. Returns `200` with the stored
/// `Organization` payload, or `404` when the pid is unknown (or
/// soft-deleted, or malformed — all map to not-found via [`http_err`]).
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    let org = model.to_org()?;
    format::json(org)
}

/// Replace an organization's payload.
///
/// `PUT /api/organizations/{pid}`. Body: a full `Organization` (replace,
/// not patch). Returns `200` with an `OrgRef`; `422` for a blank name,
/// `404` for an unknown pid. Audits and publishes an `Updated` event.
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
///
/// `DELETE /api/organizations/{pid}`. Marks the row inactive and stamps
/// `deleted_at` (the row is retained for audit). Returns `200` with an
/// empty JSON body; `404` for an unknown pid. Audits and publishes a
/// `Deleted` event. The name is captured before delete so the event
/// still carries a label.
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
///
/// `GET /api/organizations`. Returns `200` with an array of `OrgRef`,
/// newest first, soft-deleted rows excluded. The 100 cap is a deliberate
/// guard against unbounded responses (full listing is out of scope).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = OrgModel::list(&ctx.db, 100).await?;
    let refs: Vec<OrgRef> = rows.iter().map(OrgRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
///
/// `POST /api/organizations/match`. Body: a [`MatchRequest`]. Returns
/// `200` with the engine's ranked results. Stateless: it touches neither
/// the database nor the index, so it is a pure scoring utility.
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
///
/// `POST /api/organizations/check-duplicates`. Body: an `Organization`.
/// Loads up to [`CHECK_DUPLICATES_SCAN_CAP`] active rows and scores each
/// against the query, returning `200` with the matching [`ScoredRef`]s
/// sorted by descending score. An in-memory full scan (no blocking yet),
/// so it logs a `WARN` when it hits the cap (results may be truncated).
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
    // Highest score first. `partial_cmp` returns None only on NaN scores;
    // treat those as equal so the sort stays total and never panics.
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    format::json(hits)
}

/// Query string for the name-search endpoint: the optional `q` term.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// The case-insensitive substring to search names for.
    q: Option<String>,
}

/// Case-insensitive name search: `GET /api/organizations/search?q=acme`.
///
/// Returns `200` with up to 50 matching `OrgRef`s (`ILIKE '%q%'` over
/// active rows). A missing or blank `q` is `400` — an empty search would
/// match everything, which is treated as a malformed request.
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
///
/// `POST /api/organizations/merge`. Body: a [`MergeRequest`]. Returns
/// `200` with `{main_pid, duplicate_pid, main}`; `422` when the two pids
/// are equal (self-merge); `404` when either pid is unknown. The pure
/// fold lives in [`merge_orgs`]; this handler does the DB orchestration.
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
///
/// `GET /api/organizations/merges/recent`. Returns `200` with up to 100
/// `merge_records` rows (newest first).
#[debug_handler]
async fn recent_merges(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Echo the verified claims of the bearer token — `401` when the token is
/// missing or fails verification. Proves peer JWT verification against
/// the auth-service JWKS end to end (spec §13 T-9).
///
/// `GET /api/organizations/whoami`. The [`AuthUser`] extractor enforces a
/// valid token (its rejection is the `401`), so reaching the body means
/// the caller is authenticated; returns `200` with the claims.
#[debug_handler]
async fn whoami(AuthUser(claims): AuthUser) -> Result<Response> {
    format::json(claims)
}

/// Recent audit-log entries across all organizations.
///
/// `GET /api/organizations/audit/recent`. Returns `200` with up to 100
/// `audit_logs` rows (newest first).
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single organization.
///
/// `GET /api/organizations/{pid}/audit`. Returns `200` with that
/// organization's audit rows (newest first); `400` when the path `pid`
/// is not a valid UUID.
#[debug_handler]
async fn entity_audit(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    // Validate the pid shape here so a typo is a clear 400, not an empty
    // result that looks like "no audit history".
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::for_entity(&ctx.db, uuid).await?;
    format::json(rows)
}

/// Recent events from the in-memory event stream.
///
/// `GET /api/organizations/events/recent`. Returns `200` with up to 100
/// `EventView`s (`{kind, pid, name, seq}`) from the process-wide ring
/// buffer — no DB access. The buffer is per-process and not durable.
#[debug_handler]
async fn recent_events() -> Result<Response> {
    format::json(streaming::recent(100))
}

/// All organization routes, mounted under `/api/organizations`: CRUD,
/// name search, matching, duplicate-check, and audit / event endpoints.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/organizations")
        // Literal sub-paths (`/search`, `/merge`, `/whoami`, …) are added
        // before the `/{pid}` captures so they are not shadowed by the
        // dynamic segment.
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

/// DB-free unit pins for the controller's pure helpers (validation +
/// the scan-cap constant). The request-level behaviour is exercised by
/// the Postgres-gated suite in `tests/requests/organizations.rs`.
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

    /// The companion to the 422 pin: a real name passes validation.
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
