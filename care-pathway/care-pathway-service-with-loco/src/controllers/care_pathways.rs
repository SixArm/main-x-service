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
use crate::compliance::disclosure::{self, AccessContext};
use crate::compliance::erasure;
use crate::merge::merge_pathways;
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::streaming;

/// Maximum number of stored pathways scanned in-memory by
/// `check-duplicates`.
///
/// `check-duplicates` has no search-backed candidate blocking yet
/// (deferred — spec §13 T-6), so it loads up to this many active rows
/// and matches each against the query. When the scan reaches this cap
/// the result may be incomplete; the handler emits a `WARN`. Raising
/// the cap is a stop-gap — the real fix is search-blocked candidates.
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// Maximum number of **blocked candidates** `check-duplicates` scores a
/// query against.
///
/// The search index supplies these, so this bounds how many *plausible*
/// records are scored rather than how much of the table is read — which
/// is what removes the old scale cliff, where record 1001 was
/// unreachable however obvious a duplicate it was.
pub const CHECK_DUPLICATES_CANDIDATE_LIMIT: usize = 200;

/// Maximum number of rows returned by `GET /api/care-pathways` (spec §6.2).
///
/// A pragmatic bound until pagination lands; the handler must pass this
/// named const to `list` rather than a magic number.
pub const LIST_CAP: u64 = 100;

/// Maximum number of rows returned by `GET /api/care-pathways/search`
/// (spec §6.2). A pragmatic bound on the `ILIKE` scan.
pub const SEARCH_CAP: u64 = 50;

// ─── Pagination (agents/share/restful.md) ───────────────────────────────

/// Largest page any collection read will serve. A bigger `limit` is
/// **clamped** to this rather than refused.
pub const MAX_LIMIT: u64 = 500;

/// Largest accepted `offset`; past this a request is a `400`, because
/// the database would otherwise materialise and discard arbitrarily many
/// rows (SEC-G7). Deep paging past this wants a cursor.
pub const MAX_OFFSET: u64 = 10_000;

/// `?limit=` / `?offset=` on a collection read.
///
/// Declared inline on each query struct rather than `#[serde(flatten)]`-ed:
/// a flattened struct deserializes from a string-keyed map, so `limit=2`
/// arrives as the string `"2"` and fails to parse as a `u64` — a `400`
/// on a valid request.
#[derive(Debug, Default, Clone, Copy)]
struct Page {
    /// Page size; `None`, zero or unparseable ⇒ the endpoint default.
    limit: Option<u64>,
    /// Rows to skip; `None` ⇒ 0.
    offset: Option<u64>,
}

impl Page {
    /// The clamped `(limit, offset)` this request will actually use. A
    /// zero `limit` falls back to the default: an empty page and an empty
    /// collection look identical to a client, and only one is an answer.
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

/// Parse index hits into UUIDs, dropping any that will not parse.
///
/// A malformed stored id can only come from a corrupted index, and the
/// right response is to ignore that hit rather than fail a search that
/// has other perfectly good results.
fn parse_pids(hits: &[String]) -> Vec<uuid::Uuid> {
    hits.iter()
        .filter_map(|s| match uuid::Uuid::parse_str(s) {
            Ok(id) => Some(id),
            Err(err) => {
                tracing::warn!(hit = %s, error = %err, "index hit is not a UUID; ignoring");
                None
            }
        })
        .collect()
}

/// Stamp `X-Total-Count` / `X-Limit` / `X-Offset` onto a response
/// (`agents/share/restful.md`).
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

/// `?limit=` / `?offset=` for the list endpoint.
#[derive(Debug, Default, Deserialize)]
struct ListParams {
    /// Page size; absent ⇒ [`LIST_CAP`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

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

/// Map a refused read-audit write to `503 Service Unavailable`.
///
/// `503` rather than `500`: nothing is wrong with the request, and
/// nothing was disclosed — the service is temporarily unable to account
/// for a read, so it declines to serve one. The status is also
/// retryable, which is the correct signal for a transient audit-store
/// failure. Only reachable when `CARE_PATHWAY_AUDIT_FAIL_CLOSED` is on
/// (see [`crate::compliance::disclosure::fail_closed`]).
fn audit_unavailable(_: crate::compliance::disclosure::AuditWriteRefused) -> Error {
    Error::CustomError(
        StatusCode::SERVICE_UNAVAILABLE,
        ErrorDetail::new(
            "audit_unavailable",
            "the access could not be recorded in the audit trail, so the read was refused",
        ),
    )
}

/// Whether a merge request folds a record into itself.
///
/// `POST /merge` rejects this with `422` (spec §6.8): folding a record
/// into itself is a no-op that would still soft-delete the only copy.
/// Pure string comparison so the guard is unit-testable without a
/// database; the `404`-on-unknown-pid path still requires the DB.
#[must_use]
fn is_self_merge(main_pid: &str, duplicate_pid: &str) -> bool {
    main_pid == duplicate_pid
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
    /// The full-text query.
    q: Option<String>,
    /// Typo-tolerant retrieval (Levenshtein ≤ 2) instead of exact terms.
    #[serde(default)]
    fuzzy: bool,
    /// Phonetic (Soundex) retrieval; takes precedence over `fuzzy`.
    #[serde(default)]
    phonetic: bool,
    /// Page size; absent ⇒ [`SEARCH_CAP`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
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
    // Write + `Created` event, atomic under the active transport (`outbox`
    // shares one transaction; `memory` keeps today's ring-buffer path).
    let model = streaming::create_and_emit(&ctx.db, &pathway, caller.actor()).await?;
    Metrics::global().care_pathway_created_total.inc();
    // Audit is written inside `create_and_emit` (in the outbox transaction
    // under `outbox`; best-effort under `memory`) — see `streaming`.
    format::json(PathwayRef::of(&model))
}

/// Fetch a care pathway by public id.
///
/// `GET /api/care-pathways/{pid}` — response is the full stored
/// [`CarePathway`]. `404` when `pid` is unknown or soft-deleted.
///
/// **Audited as a read** (HIPAA §164.312(b)) when
/// `CARE_PATHWAY_AUDIT_READS` is on, carrying the caller's declared
/// purpose-of-use and disclosure recipient. The audit row is written only
/// on a successful read: a `404` disclosed nothing, and recording it would
/// pollute the §164.528 accounting with accesses that never happened.
///
/// # Errors
///
/// `404` when no active row has that `pid`; otherwise DB/parse errors.
#[debug_handler]
async fn get_one(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let model = PathwayModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    disclosure::record_access(
        &ctx.db,
        model.pid,
        disclosure::action::READ,
        caller.actor(),
        &access,
    )
    .await
    .map_err(audit_unavailable)?;
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
    let model = PathwayModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    // Update + `Updated` event, atomic under the active transport.
    let updated = streaming::update_and_emit(&ctx.db, model, &pathway, caller.actor()).await?;
    Metrics::global().care_pathway_updated_total.inc();
    // Audit is written inside `update_and_emit` (see `streaming`).
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
    let model = PathwayModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    // Soft-delete + `Deleted` event, atomic under the active transport.
    let (_entity_pid, _name) = streaming::delete_and_emit(&ctx.db, model, caller.actor()).await?;
    Metrics::global().care_pathway_deleted_total.inc();
    // Audit is written inside `delete_and_emit` (see `streaming`).
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
async fn list(
    axum::extract::Query(params): axum::extract::Query<ListParams>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(LIST_CAP);
    let rows = PathwayModel::list_paged(&ctx.db, limit, offset).await?;
    let total = PathwayModel::count(&ctx.db).await?;
    // A collection read is recorded against the nil `pid`: it disclosed
    // many records, not one, so attributing it to any single record would
    // corrupt that record's §164.528 accounting.
    disclosure::record_access(
        &ctx.db,
        uuid::Uuid::nil(),
        disclosure::action::LIST,
        caller.actor(),
        &access,
    )
    .await
    .map_err(audit_unavailable)?;
    let refs: Vec<PathwayRef> = rows.iter().map(PathwayRef::of).collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
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
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let q = params.q.unwrap_or_default();
    // Reject an absent/blank term rather than ILIKE-ing on `%%`.
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(SEARCH_CAP);
    // An unavailable index is reported, never disguised as "no matches":
    // an operator must be able to tell a broken index from an empty one.
    let Some(engine) = crate::search::engine() else {
        return Err(Error::CustomError(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorDetail::new("search_unavailable", "the search index is unavailable"),
        ));
    };
    let mode = if params.phonetic {
        crate::search::SearchMode::Phonetic
    } else if params.fuzzy {
        crate::search::SearchMode::Fuzzy
    } else {
        crate::search::SearchMode::Exact
    };
    let (pids, index_total) = engine.search_page(
        q.trim(),
        mode,
        usize::try_from(limit).unwrap_or(usize::MAX),
        usize::try_from(offset).unwrap_or(usize::MAX),
    )?;
    let rows = PathwayModel::find_by_pids(&ctx.db, &parse_pids(&pids)).await?;
    let total = index_total as u64;
    disclosure::record_access(
        &ctx.db,
        uuid::Uuid::nil(),
        disclosure::action::SEARCH,
        caller.actor(),
        &access,
    )
    .await
    .map_err(audit_unavailable)?;
    let refs: Vec<PathwayRef> = rows.iter().map(PathwayRef::of).collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
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
    // Blocking, not scanning (spec §13 T-6, now landed): the index
    // supplies plausible candidates — fuzzy title, exact identifier and
    // phonetic routes — so a duplicate's reachability depends on how
    // similar it is rather than on how recently it was inserted. An
    // unavailable index is surfaced rather than silently answering "no
    // duplicates", which would let a caller create a duplicate believing
    // it had been checked.
    let Some(index) = crate::search::engine() else {
        return Err(Error::CustomError(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorDetail::new(
                "search_unavailable",
                "the search index is unavailable, so duplicates cannot be checked",
            ),
        ));
    };
    let candidates = index.candidates(&query, CHECK_DUPLICATES_CANDIDATE_LIMIT)?;
    let rows = PathwayModel::find_by_pids(&ctx.db, &parse_pids(&candidates)).await?;
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
    if is_self_merge(&req.main_pid, &req.duplicate_pid) {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("validation", "main_pid and duplicate_pid must differ"),
        ));
    }
    // Both must exist and be active; either missing is the `find_by_pid`
    // `404`.
    let main = PathwayModel::find_by_pid(&ctx.db, &req.main_pid)
        .await
        .map_err(super::model_not_found)?;
    let duplicate = PathwayModel::find_by_pid(&ctx.db, &req.duplicate_pid)
        .await
        .map_err(super::model_not_found)?;

    let outcome = merge_pathways(&main.to_pathway()?, &duplicate.to_pathway()?);

    // Update the survivor + retire the duplicate, emitting `Merged` (main)
    // and `Deleted` (duplicate) — all atomic under the active transport.
    let (merged, dup_pid, _dup_name) =
        streaming::merge_and_emit(&ctx.db, main, duplicate, &outcome.merged, caller.actor())
            .await?;
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
    // The two audit entries (survivor "merged", duplicate "merged_into")
    // are written inside `merge_and_emit` — atomic with the merge under
    // `outbox`. The merge-history row above stays a best-effort side
    // channel (merge metadata, not the §3 audit trail).

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

/// Accounting of disclosures for one care pathway (HIPAA §164.528).
///
/// `GET /api/care-pathways/{pid}/audit/disclosures` — every audit row for
/// this record that was classified as an outward **disclosure** rather
/// than an internal access, newest first. Ordinary accesses are excluded;
/// that distinction is the entire point of the accounting.
///
/// An empty array has two very different meanings — nothing was disclosed,
/// or read-auditing was never switched on — so the response says which,
/// rather than letting a reader assume the flattering one.
///
/// # Errors
///
/// `400` when `pid` is not a valid UUID; otherwise DB query errors.
#[debug_handler]
async fn entity_disclosures(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::disclosures_for_entity(&ctx.db, uuid).await?;
    format::json(serde_json::json!({
        "pid": pid,
        "read_auditing_enabled": crate::compliance::audit_reads(),
        "count": rows.len(),
        "caveat": if crate::compliance::audit_reads() {
            "complete for the period read-auditing has been enabled"
        } else {
            "INCOMPLETE — CARE_PATHWAY_AUDIT_READS is off, so read disclosures are not \
             being recorded; only disclosure-flagged mutations appear here"
        },
        "disclosures": rows,
    }))
}

/// Erase a care pathway under GDPR Art. 17.
///
/// `POST /api/care-pathways/{pid}/erase` — replaces the payload with a
/// tombstone, retires the record, destroys the content of every audit row
/// about it, and appends a chained `erased` accountability row. The audit
/// hash chain keeps verifying, because redaction preserves each row's
/// stored hash and linkage (see [`crate::compliance::erasure`]).
///
/// This is **not** the soft delete. `DELETE /{pid}` retires a record and
/// keeps its data; this destroys the data and is irreversible — which is
/// why it is a **destructive** action under ABAC
/// (`auth::DESTRUCTIVE_POST_SUFFIXES`) and requires `access=admin`.
///
/// Idempotent: erasing an already-erased or already-deleted `pid` still
/// sweeps any audit content held about it, because the subject's right
/// does not lapse when the record is soft-deleted.
///
/// # Errors
///
/// `400` when `pid` is not a valid UUID; otherwise DB errors.
#[debug_handler]
async fn erase(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let outcome = match PathwayModel::find_by_pid(&ctx.db, &pid).await {
        Ok(model) => erasure::erase(&ctx.db, model, caller.actor(), &access).await?,
        // No live record: still sweep the audit content held about it.
        Err(_) => erasure::erase_audit_only(&ctx.db, uuid, caller.actor(), &access).await?,
    };
    Metrics::global().care_pathway_deleted_total.inc();
    format::json(outcome)
}

/// Recent events from the active event transport.
///
/// `GET /api/care-pathways/events/recent` — the last 100
/// [`streaming::EventView`]s, served from the process-local ring buffer
/// (`memory`, not durable across restarts) or the `event_outbox` table
/// (`outbox`). The wire shape is identical either way.
///
/// # Errors
///
/// Propagates the outbox query error under the `outbox` transport
/// (`memory` never errors).
#[debug_handler]
async fn recent_events(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(streaming::recent_events(&ctx.db, 100).await?)
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
        .add("/{pid}/audit/disclosures", get(entity_disclosures))
        .add("/{pid}/erase", post(erase))
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

    /// The self-merge guard (spec §6.8) is a pure equal-pid check that
    /// runs without a database: equal pids fold a record into itself and
    /// must be rejected, while distinct pids are allowed through to the
    /// DB-backed `404`/merge path. This pins the `422` self-merge contract
    /// on the default `cargo test`; the request-level
    /// `merge_with_equal_pids_is_422` only runs behind `--ignored`.
    #[test]
    fn is_self_merge_detects_equal_pids() {
        let pid = "00000000-0000-4000-8000-000000000000";
        assert!(is_self_merge(pid, pid), "equal pids are a self-merge");
        assert!(
            !is_self_merge(pid, "11111111-1111-4111-8111-111111111111"),
            "distinct pids are not a self-merge"
        );
    }

    /// Pins the documented `check-duplicates` in-memory scan cap. The
    /// handler must pass this named const to `list` (not a magic
    /// number) and WARN when the scan reaches it (spec §13 T-6).
    #[test]
    fn check_duplicates_scan_cap_is_the_documented_value() {
        assert_eq!(CHECK_DUPLICATES_SCAN_CAP, 1000);
    }

    /// Pins the documented list / search caps (spec §6.2). The DB-gated
    /// request suite exercises the caps end-to-end behind `#[ignore]`, but
    /// a regression in the constant value itself must fail the default,
    /// DB-free `cargo test` — so assert the constants directly here.
    #[test]
    fn list_and_search_caps_are_the_documented_values() {
        assert_eq!(LIST_CAP, 100, "spec §6.2: list caps at 100");
        assert_eq!(SEARCH_CAP, 50, "spec §6.2: search caps at 50");
    }
}
