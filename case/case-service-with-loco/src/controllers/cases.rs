//! Case CRUD + matching endpoints.
//!
//! The API DTO is `case_matcher::Case` itself — the service stores it
//! verbatim (as JSON) and matches with the canonical `case-matcher`
//! engine, so there is no separate model or adapter to drift.

use authentication_verifier::Action;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use case_matcher::{Case, MatchConfig, MatchingEngine};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;

use crate::compliance::checkpoint::Checkpoint;
use serde::{Deserialize, Serialize};

use crate::auth::{AuthUser, MaybeAuthUser};
use crate::compliance::disclosure::{self, AccessContext};
use crate::merge::merge_cases;
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::cases::Model as CaseModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::streaming;

/// Maximum number of stored cases scanned in-memory by
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

// ─── Pagination (agents/share/restful.md) ───────────────────────────────

/// Default page size for `GET /api/cases` — the cap this endpoint
/// applied before pagination, so omitting `limit` returns what it always
/// did.
pub const LIST_DEFAULT_LIMIT: u64 = 100;

/// Default page size for `GET /api/cases/search`.
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
                    format!("offset must not exceed {MAX_OFFSET}; narrow the query instead"),
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

/// Stamp `X-Total-Count` / `X-Limit` / `X-Offset` onto a response.
///
/// The total is the **collection's** match count, taken before the
/// per-record concealment this controller applies: a caller-specific
/// total would leak exactly what concealment hides — how many records
/// they are not allowed to see.
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
    /// Page size; absent ⇒ [`LIST_DEFAULT_LIMIT`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

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
        ErrorDetail::new("validation", problems.join("; ")),
    ))
}

/// A lightweight reference to a stored case: just its public id and
/// title. Returned by create/update/list/search instead of the full
/// payload, since case data is personal data and these endpoints only
/// need to identify the row.
#[derive(Debug, Serialize)]
struct CaseRef {
    /// The case's public id (UUID v4), as a string.
    pid: String,
    /// The case's denormalised title.
    title: String,
}

impl CaseRef {
    /// Project a stored [`CaseModel`] down to its `{pid, title}` reference.
    fn of(m: &CaseModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            title: m.title.clone(),
        }
    }
}

/// Body of `POST /api/cases/match`: a query plus the explicit candidate
/// list to rank it against (no persistence is touched).
#[derive(Debug, Deserialize)]
struct MatchRequest {
    /// The case to score against each candidate.
    query: Case,
    /// The candidate cases to rank.
    candidates: Vec<Case>,
}

/// Query string for `GET /api/cases/search`: the `q` title term.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// The full-text query. Required in practice — a missing/blank `q`
    /// is rejected with `400`.
    q: Option<String>,
    /// Typo-tolerant retrieval (Levenshtein ≤ 2) instead of exact terms.
    #[serde(default)]
    fuzzy: bool,
    /// Phonetic (Soundex) retrieval; takes precedence over `fuzzy`.
    #[serde(default)]
    phonetic: bool,
    /// Page size; absent ⇒ [`SEARCH_DEFAULT_LIMIT`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

/// Body of `POST /api/cases/merge`: which duplicate folds into which
/// survivor, and (optionally) why.
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

/// A scored `check-duplicates` hit: a stored case that matched the query,
/// with its score and confidence. Serialized into the response array.
#[derive(Debug, Serialize)]
struct ScoredRef {
    /// The matched case's public id (UUID v4), as a string.
    pid: String,
    /// The matched case's denormalised title.
    title: String,
    /// The match score in `[0.0, 1.0]`.
    score: f64,
    /// The confidence band (the matcher's `Confidence`, `Debug`-formatted).
    confidence: String,
    /// Whether the score cleared the match threshold (always `true` for a
    /// returned hit — only matches are collected).
    is_match: bool,
}

/// Descending-by-score comparator for `check-duplicates` ranking.
///
/// Sorts hits best-score-first. `partial_cmp` is `None` only on a NaN
/// score (matcher scores are finite in practice), so a NaN is treated as
/// "equal" rather than panicking on `unwrap`. Pure and unit-tested so the
/// ranking order is pinned without a database.
fn score_desc(a: f64, b: f64) -> std::cmp::Ordering {
    b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
}

/// Create a case. `POST /api/cases`.
///
/// Request body: a `case_matcher::Case`. Validates the payload (blank
/// `title` etc. ⇒ `422`), inserts the row, writes a `created` audit entry
/// stamped with the caller (when a token was presented), and publishes a
/// `Created` event. Responds `200` with a [`CaseRef`] (`{pid, title}`) —
/// not the full payload, since case data is personal data.
///
/// # Errors
///
/// `422` on validation failure; a DB error on insert/audit failure
/// (auditing itself is best-effort and never fails the request).
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(case): Json<Case>,
) -> Result<Response> {
    validate(&case)?;
    // Write the row and emit its `Created` event atomically under the
    // active transport (`streaming::create_and_emit`); `outbox` writes the
    // row and its `event_outbox` row on one transaction.
    let model = streaming::create_and_emit(&ctx.db, &case, caller.actor()).await?;
    // Audit is written by `streaming::create_and_emit` (atomic with the
    // entity + event under `outbox`, best-effort under `memory`).
    Metrics::global().case_created_total.inc();
    format::json(CaseRef::of(&model))
}

/// Map a record-level authorization rejection (`(status, reason)`) to a
/// loco error response. `403` = the policy denied the action given the
/// record's attributes; `401` = a fail-safe when claims are missing
/// behind the guard.
fn record_rejection((status, reason): (StatusCode, String)) -> Error {
    let code = if status == StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    Error::CustomError(status, ErrorDetail::new(code, &reason))
}

/// A **masked** view of a case, for a `mask`-obligation allow: drops the
/// fields that most directly expose who the case is about — the
/// involved-party `subjects`, external `identifiers`, `same_as` URLs,
/// and the agency `case_number` — keeping the descriptive shell (title,
/// type, status, …). Case data is personal data, so a policy can attach
/// the `mask` obligation to a conditional read (e.g. cross-department).
pub(crate) fn mask_case(case: &Case) -> Case {
    let mut masked = case.clone();
    masked.subjects = Vec::new();
    masked.identifiers = Vec::new();
    masked.same_as = Vec::new();
    masked.case_number = None;
    masked
}

/// Fetch a case by public id. `GET /api/cases/{pid}`.
///
/// Responds `200` with the stored `case_matcher::Case`. When the
/// record-level ABAC decision carries the `mask` **obligation**, the
/// **masked** view is returned instead of the full record (see
/// [`mask_case`]).
///
/// # Errors
///
/// `404` when no active case has that `pid` (or `pid` is not a UUID);
/// `403` when the record-level ABAC policy denies reading this specific
/// case (e.g. an `resource.case_type`-gated rule) with enforcement on.
#[debug_handler]
async fn get_one(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let model = CaseModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    let case = model.to_case()?;
    let obligations = crate::auth::authorize_record(
        &caller,
        Action::Read,
        &crate::auth::case_resource_attrs(&case),
    )
    .map_err(record_rejection)?;
    // Audited only once authorization has allowed the read: a denied
    // request disclosed nothing, and recording it would pollute the
    // §164.528 accounting with accesses that never happened.
    disclosure::record_access(
        &ctx.db,
        model.pid,
        disclosure::action::READ,
        caller.actor(),
        &access,
    )
    .await
    .map_err(audit_unavailable)?;
    // mask-on-allow: honour a `mask` obligation by redacting the record.
    if obligations.iter().any(|o| o == "mask") {
        format::json(mask_case(&case))
    } else {
        format::json(case)
    }
}

/// The masked view of a case.
///
/// `GET /api/cases/{pid}/masked`. Returns `200` with the record redacted
/// per [`mask_case`] — involved-party `subjects`, external
/// `identifiers`, `same_as` URLs, and the agency `case_number` —
/// regardless of the caller's policy. `404` for an unknown pid.
///
/// Distinct from the `mask` obligation on `GET /{pid}`: that one is the
/// deployment deciding what a caller may see, while this endpoint is a
/// caller *asking* for the redacted form (a screen share, a support
/// call, an export preview).
#[debug_handler]
async fn get_masked(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = CaseModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    format::json(mask_case(&model.to_case()?))
}

/// Build the GDPR right-of-access payload for one case.
///
/// An **envelope**, not the bare record: an access request has to be
/// answerable as "here is everything held about this subject, as of
/// then", so the response states what was exported, when, and whether
/// any of it was redacted. `masked` is the honest part — a masked
/// export is a *partial* answer, and the caller must be able to tell
/// which they received.
fn export_case(case: &Case, pid: &str, masked: bool) -> serde_json::Value {
    let record = if masked {
        mask_case(case)
    } else {
        case.clone()
    };
    serde_json::json!({
        "entity": "case",
        "pid": pid,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "masked": masked,
        "record": serde_json::to_value(&record).unwrap_or(serde_json::Value::Null),
        "note": if masked {
            "Partial export: subjects, identifiers, same_as, and case_number are \
             redacted for this caller."
        } else {
            "Complete export of the stored case record."
        },
    })
}

/// GDPR right-of-access export for one case.
///
/// `GET /api/cases/{pid}/export`. Returns `200` with the envelope from
/// [`export_case`]; `404` for an unknown pid.
///
/// **Every export is audited** as a disclosure (HIPAA §164.528), the
/// same accounting `GET /{pid}` feeds — extracting a case record is
/// itself a compliance event whether or not it is masked.
///
/// A caller the policy would mask gets a **masked** export, and the
/// envelope says so — an access request answered with redactions must
/// not look like a complete answer.
///
/// # Errors
///
/// `404` when no active case has that `pid`; `403` when the
/// record-level ABAC policy denies reading this specific case;
/// otherwise DB/parse errors, or a `503` when the export could not be
/// recorded on the audit trail.
#[debug_handler]
async fn get_export(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let model = CaseModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    let case = model.to_case()?;
    let obligations = crate::auth::authorize_record(
        &caller,
        Action::Read,
        &crate::auth::case_resource_attrs(&case),
    )
    .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");
    disclosure::record_access(
        &ctx.db,
        model.pid,
        disclosure::action::EXPORT,
        caller.actor(),
        &access,
    )
    .await
    .map_err(audit_unavailable)?;
    format::json(export_case(&case, &model.pid.to_string(), masked))
}

/// Replace a case's payload. `PUT /api/cases/{pid}`.
///
/// Request body: a full `case_matcher::Case`. Validates it, replaces the
/// stored payload (and denormalised title), writes an `updated` audit
/// entry, and publishes an `Updated` event. Responds `200` with a
/// [`CaseRef`].
///
/// # Errors
///
/// `422` on validation failure; `404` when the `pid` is unknown; `403`
/// when the record-level ABAC policy denies writing this specific case
/// (evaluated against the *stored* case's attributes) with enforcement on.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(case): Json<Case>,
) -> Result<Response> {
    validate(&case)?;
    let model = CaseModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    // Record-level check uses the *existing* stored case's attributes
    // (e.g. deny modifying a case whose stored status is `closed`).
    crate::auth::authorize_record(
        &caller,
        Action::Write,
        &crate::auth::case_resource_attrs(&model.to_case()?),
    )
    .map_err(record_rejection)?;
    let updated = streaming::update_and_emit(&ctx.db, model, &case, caller.actor()).await?;
    // Audit is written by `streaming::update_and_emit` (see `create`).
    Metrics::global().case_updated_total.inc();
    format::json(CaseRef::of(&updated))
}

/// Soft-delete a case. `DELETE /api/cases/{pid}`.
///
/// Marks the row inactive and stamps `deleted_at` (the row is retained
/// for the audit trail, since case data is personal data). Writes a
/// `deleted` audit entry and publishes a `Deleted` event. Responds `200`
/// with an empty JSON body.
///
/// # Errors
///
/// `404` when the `pid` is unknown; `403` when the record-level ABAC
/// policy denies deleting this specific case with enforcement on.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = CaseModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    crate::auth::authorize_record(
        &caller,
        Action::Delete,
        &crate::auth::case_resource_attrs(&model.to_case()?),
    )
    .map_err(record_rejection)?;
    // Audit is written by `streaming::delete_and_emit` (see `create`).
    streaming::delete_and_emit(&ctx.db, model, caller.actor()).await?;
    Metrics::global().case_deleted_total.inc();
    format::empty_json()
}

/// List active cases (capped at 100). `GET /api/cases`.
///
/// Responds `200` with an array of [`CaseRef`] (`{pid, title}`),
/// most-recent first. The 100 cap is a pragmatic page size; pagination is
/// not yet modelled.
///
/// # Errors
///
/// A DB error when the query fails.
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
    let (limit, offset) = page.resolve(LIST_DEFAULT_LIMIT);
    let rows = CaseModel::list_paged(&ctx.db, limit, offset).await?;
    let total = CaseModel::count(&ctx.db).await?;
    // A collection read is recorded against the nil `pid`: it disclosed
    // many records, not one, so attributing it to any single case would
    // corrupt that case's §164.528 accounting.
    disclosure::record_access(
        &ctx.db,
        uuid::Uuid::nil(),
        disclosure::action::LIST,
        caller.actor(),
        &access,
    )
    .await
    .map_err(audit_unavailable)?;
    // SEC-G3: drop rows the caller may not read, so the list never leaks the
    // existence (pid + title) of a concealed case. No-op when enforcement off.
    let refs: Vec<CaseRef> = rows
        .iter()
        .filter(|row| {
            row.to_case()
                .is_ok_and(|c| crate::auth::read_visibility(&caller, &c).is_some())
        })
        .map(CaseRef::of)
        .collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
}

/// Full-text case search: `GET /api/cases/search?q=housing`.
/// Tantivy-backed (spec §13 T-6, landed) over title, alternate titles,
/// agency name, identifiers, keywords, and subjects; `?fuzzy=true`
/// tolerates typos and `?phonetic=true` matches on how the title
/// sounds.
///
/// Responds `200` with an array of [`CaseRef`]. A missing/blank `q` is a
/// `400`; an unavailable search index is a `503` (never disguised as
/// "no matches").
///
/// # Errors
///
/// `400` when `q` is missing or blank; `503` when the search index is
/// unavailable; a DB error when the query fails.
#[debug_handler]
async fn search(
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
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
    let rows = CaseModel::find_by_pids(&ctx.db, &parse_pids(&pids)).await?;
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
    // SEC-G3: drop rows the caller may not read (same concealment as `list`).
    let refs: Vec<CaseRef> = rows
        .iter()
        .filter(|row| {
            row.to_case()
                .is_ok_and(|c| crate::auth::read_visibility(&caller, &c).is_some())
        })
        .map(CaseRef::of)
        .collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
}

/// Score a query against an explicit candidate list (no persistence).
/// `POST /api/cases/match`.
///
/// Request body: a [`MatchRequest`] (`{query, candidates}`). Ranks the
/// candidates with the default-config `case-matcher` engine and responds
/// `200` with the ranked `(index, MatchResult)` results. Stateless — it
/// never touches the database.
///
/// # Errors
///
/// Propagates a response-serialization error (none expected in practice).
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored cases that match the query above the threshold.
/// `POST /api/cases/check-duplicates`.
///
/// Request body: a `case_matcher::Case` query. The search index supplies
/// a **blocked** candidate set (spec §13 T-6, landed) — up to
/// [`CHECK_DUPLICATES_CANDIDATE_LIMIT`] plausible matches via fuzzy
/// title, exact identifier, and phonetic retrieval — each scored against
/// the query, and responds `200` with the matching [`ScoredRef`]s sorted
/// by descending score.
///
/// # Errors
///
/// `503` when the search index is unavailable, so duplicates cannot be
/// checked; a DB error when the row fetch fails; a JSON parse error when
/// a stored payload cannot be deserialized.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(query): Json<Case>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    // Blocking, not scanning: the index supplies plausible candidates —
    // fuzzy title, exact identifier, and phonetic routes — so a
    // duplicate's reachability depends on how similar it is rather than
    // on how recently it was inserted. An unavailable index is surfaced
    // rather than silently answering "no duplicates", which would let a
    // caller create a duplicate believing it had been checked.
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
    let rows = CaseModel::find_by_pids(&ctx.db, &parse_pids(&candidates)).await?;
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_case()?;
        // SEC-G3: a caller must not discover a concealed case via dedup —
        // skip candidates they may not read.
        if crate::auth::read_visibility(&caller, &candidate).is_none() {
            continue;
        }
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
    // Best score first (see [`score_desc`]).
    hits.sort_by(|a, b| score_desc(a.score, b.score));
    format::json(hits)
}

/// Merge a confirmed-duplicate case into a surviving (main) case:
/// union the duplicate's data into main, keep the duplicate's title as an
/// alternate title, soft-delete the duplicate, record the merge history,
/// and publish a `Merged` event (plus a `Deleted` for the duplicate).
/// `POST /api/cases/merge`.
///
/// Request body: a [`MergeRequest`] (`{main_pid, duplicate_pid,
/// reason?}`). The pure fold lives in [`merge_cases`]; this handler does
/// the DB orchestration and auditing. Responds `200` with
/// `{main_pid, duplicate_pid, main}` (the survivor's merged payload).
///
/// # Errors
///
/// `422` when `main_pid == duplicate_pid` (self-merge); `404` when either
/// pid is unknown; a DB error on the update/soft-delete.
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
    let main = CaseModel::find_by_pid(&ctx.db, &req.main_pid)
        .await
        .map_err(super::model_not_found)?;
    let duplicate = CaseModel::find_by_pid(&ctx.db, &req.duplicate_pid)
        .await
        .map_err(super::model_not_found)?;

    let outcome = merge_cases(&main.to_case()?, &duplicate.to_case()?);

    // Update the survivor + retire the duplicate + emit the `Merged` /
    // `Deleted` pair, all atomic under the active transport (one
    // transaction for `outbox`).
    let (merged, dup_pid, _dup_title) =
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
    // Both audit rows (`merged` + `merged_into`) are written by
    // `streaming::merge_and_emit` (see `create`).
    Metrics::global().case_merged_total.inc();

    format::json(serde_json::json!({
        "main_pid": merged.pid.to_string(),
        "duplicate_pid": dup_pid.to_string(),
        "main": merged.to_case()?,
    }))
}

/// Recent merge-history records, newest first (cap 100).
/// `GET /api/cases/merges/recent`. Responds `200` with the merge rows.
///
/// # Errors
///
/// A DB error when the query fails.
#[debug_handler]
async fn recent_merges(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = MergeRecordModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Recent audit-log entries across all cases (cap 100).
/// `GET /api/cases/audit/recent`. Responds `200` with the audit rows.
///
/// # Errors
///
/// A DB error when the query fails.
#[debug_handler]
async fn recent_audit(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = AuditModel::recent(&ctx.db, 100).await?;
    format::json(rows)
}

/// Audit trail for a single case. `GET /api/cases/{pid}/audit`.
/// Responds `200` with the case's audit rows, newest first.
///
/// # Errors
///
/// `400` when `pid` is not a UUID; a DB error when the query fails.
#[debug_handler]
async fn entity_audit(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    // Parse the path pid up front so a malformed id is a clean 400 rather
    // than an empty result set.
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    let rows = AuditModel::for_entity(&ctx.db, uuid).await?;
    format::json(rows)
}

/// Recent events from the active transport (cap 100).
/// `GET /api/cases/events/recent`. Responds `200` with the flat
/// `EventView` projection (`{kind, pid, name, seq}`) — served from the
/// ring buffer (`memory`) or the `event_outbox` table (`outbox`), with an
/// identical wire shape either way.
///
/// # Errors
///
/// Propagates the outbox query error under the `outbox` transport
/// (`memory` never errors).
#[debug_handler]
async fn recent_events(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(streaming::recent_events(&ctx.db, 100).await?)
}

/// Echo the verified claims of the bearer token. `GET /api/cases/whoami`.
/// Responds `200` with the [`Claims`]; `401` when the token is missing or
/// fails verification. Proves peer JWT verification against the
/// auth-service JWKS end to end (spec §13 T-7). Taking [`AuthUser`] is
/// what makes this the one always-protected route.
///
/// [`Claims`]: authentication_verifier::Claims
///
/// # Errors
///
/// `401` when the bearer token is missing or fails verification (returned
/// by the [`AuthUser`] extractor before this body runs).
#[debug_handler]
async fn whoami(AuthUser(claims): AuthUser) -> Result<Response> {
    format::json(claims)
}

/// Verify the tamper-evident audit hash chain.
///
/// `GET /api/cases/audit/verify?limit=1000` — recomputes the trailing
/// rows and reports every linkage or content break (HIPAA §164.312(c)).
/// Attests to the audit trail only, not to the `cases` rows.
///
/// # Errors
///
/// Propagates DB query errors.
#[debug_handler]
async fn verify_audit_chain(
    axum::extract::Query(params): axum::extract::Query<VerifyParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let limit = params.limit();
    let report = AuditModel::verify_chain(&ctx.db, limit).await?;
    format::json(serde_json::json!({
        "limit": limit,
        "verified": report.verified,
        "rows": report.rows,
        "intact": report.intact,
        "redacted": report.redacted,
        "unchained": report.unchained,
        "head": report.head,
        "breaks": report.breaks,
        "interpretation": if report.verified {
            "no break detected in the verified window; this attests to the audit trail only, \
             not to the cases rows"
        } else {
            "a break means rows were inserted, deleted, reordered, or edited since they were \
             written — investigate the named ids; under CASE_EVENT_TRANSPORT=memory a linkage \
             break may instead mean two concurrent audit writes raced"
        },
    }))
}

/// Accounting of disclosures for one case (HIPAA §164.528).
///
/// `GET /api/cases/{pid}/audit/disclosures` — every audit row for this
/// case classified as an outward **disclosure** rather than an internal
/// access, newest first.
///
/// Gated by the same record-level authorization as reading the case:
/// learning who a case was disclosed to reveals that the case exists, so
/// the accounting cannot be more open than the record it describes
/// (`agents/share/cross-service-linking.md` §10).
///
/// # Errors
///
/// `400` when `pid` is not a UUID; `403` when the policy denies reading
/// this case; `404` when it is unknown.
#[debug_handler]
async fn entity_disclosures(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let Ok(uuid) = uuid::Uuid::parse_str(&pid) else {
        return bad_request("invalid pid");
    };
    // Concealment first: an unauthorised caller must not learn that the
    // case exists by asking for its disclosure history.
    let model = CaseModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(super::model_not_found)?;
    let case = model.to_case()?;
    crate::auth::authorize_record(
        &caller,
        Action::Read,
        &crate::auth::case_resource_attrs(&case),
    )
    .map_err(record_rejection)?;
    let rows = AuditModel::disclosures_for_entity(&ctx.db, uuid).await?;
    format::json(serde_json::json!({
        "pid": pid,
        "read_auditing_enabled": crate::compliance::audit_reads(),
        "count": rows.len(),
        "caveat": if crate::compliance::audit_reads() {
            "complete for the period read-auditing has been enabled"
        } else {
            "INCOMPLETE — CASE_AUDIT_READS is off, so read disclosures are not being \
             recorded; only disclosure-flagged mutations appear here"
        },
        "disclosures": rows,
    }))
}

/// Erase a case under GDPR Art. 17.
///
/// `POST /api/cases/{pid}/erase` — replaces the payload with a tombstone,
/// retires the record, withdraws its cross-service links, destroys the
/// content of every audit row about it, and appends a chained `erased`
/// accountability row. The audit hash chain keeps verifying, because
/// redaction preserves each row's stored hash and linkage (see
/// [`crate::compliance::erasure`]).
///
/// This is **not** the soft delete. `DELETE /{pid}` retires a record and
/// keeps its data; this destroys the data and is irreversible — which is
/// why it is a **destructive** action under ABAC
/// ([`crate::auth::DESTRUCTIVE_POST_SUFFIXES`]) and requires
/// `access=admin`.
///
/// The link withdrawal is what makes this meaningful for a case rather
/// than merely correct. A `subject_of` edge asserts that a named person
/// is the subject of a benefits, legal, or investigative proceeding
/// (`agents/share/cross-service-linking.md` §10). Tombstoning the payload
/// while leaving that edge standing would erase the details and keep the
/// accusation — the opposite of what the subject asked for.
///
/// Idempotent: erasing an already-erased or already-deleted `pid` still
/// sweeps any audit content and links held about it, because the
/// subject's right does not lapse when the record is soft-deleted.
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
    let outcome = match CaseModel::find_by_pid(&ctx.db, &pid).await {
        Ok(model) => {
            crate::compliance::erasure::erase(&ctx.db, model, caller.actor(), &access).await?
        }
        // No live record: still sweep the audit content held about it.
        Err(_) => {
            crate::compliance::erasure::erase_audit_only(&ctx.db, uuid, caller.actor(), &access)
                .await?
        }
    };
    format::json(outcome)
}

/// Verify row-level record integrity across recent case records.
///
/// `GET /api/cases/records/verify?limit=1000` — recomputes each record's
/// content hash and names any row changed outside the service.
///
/// The **complement** to `/api/cases/audit/verify`, not a duplicate: the
/// chain proves the trail was not rewritten, this proves the records were
/// not edited out of band. An attacker with SQL access who edits a stored
/// case and writes no audit row defeats the first and is caught by this.
///
/// Rows written before the `content_hash` column existed report as
/// `unhashed` — neither verified nor a mismatch — so adopting the control
/// on a populated table does not produce a wall of false positives.
///
/// # Errors
///
/// Propagates the query error.
#[debug_handler]
async fn verify_records(
    Query(params): Query<VerifyParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let limit = params.limit();
    let report = CaseModel::verify_records(&ctx.db, limit).await?;
    format::json(serde_json::json!({
        "limit": limit,
        "verified": report.verified,
        "records": report.records,
        "intact": report.intact,
        "unhashed": report.unhashed,
        "mismatched": report.mismatched,
        "interpretation": if report.verified {
            "no record in the verified window differs from its stored hash; this attests to \
             the case records, not to the audit trail — see /api/cases/audit/verify for that"
        } else {
            "a mismatch means the record's content changed without the service rehashing it — \
             either an out-of-band SQL edit, or a write path that forgot to rehash; \
             investigate the named pids against the audit trail"
        },
    }))
}

/// Take a chain checkpoint — the external witness against wholesale
/// deletion.
///
/// `GET /api/cases/checkpoint` returns a signed statement of the chain's
/// current head, position and depth. **Store it outside this service's
/// database.** A checkpoint kept alongside the data it witnesses is
/// worthless: whoever can delete audit rows can delete the checkpoint in
/// the same transaction, and its MAC prevents forgery, not deletion.
///
/// It is also emitted as an `INFO` log line on the `audit_checkpoint`
/// target, so a deployment that already ships logs off the host has a
/// witness without building anything further.
///
/// `204` when the chain is empty: there is nothing to witness, and a
/// checkpoint any future state satisfies would be worse than none.
///
/// # Errors
///
/// Propagates the query error.
#[debug_handler]
async fn take_checkpoint(State(ctx): State<AppContext>) -> Result<Response> {
    let now = chrono::Utc::now().timestamp_micros();
    let Some(checkpoint) = Checkpoint::take(&ctx.db, now).await? else {
        return Ok((StatusCode::NO_CONTENT, ()).into_response());
    };
    tracing::info!(
        target: "audit_checkpoint",
        version = %checkpoint.version,
        anchor_id = checkpoint.anchor_id,
        head = %checkpoint.head,
        rows_at_or_before = checkpoint.rows_at_or_before,
        taken_at_micros = checkpoint.taken_at_micros,
        mac = checkpoint.mac.as_deref().unwrap_or("none"),
        "audit chain checkpoint"
    );
    format::json(checkpoint)
}

/// Check whether the chain still honours a recorded checkpoint.
///
/// `POST /api/cases/checkpoint/verify`. This is the half that detects
/// deletion: the chain cannot see rows removed from its tail, because
/// nothing is left to break.
///
/// # Errors
///
/// Propagates the query error.
#[debug_handler]
async fn verify_checkpoint(
    State(ctx): State<AppContext>,
    Json(checkpoint): Json<Checkpoint>,
) -> Result<Response> {
    let verdict = checkpoint.verify_against(&ctx.db).await?;
    let honoured = verdict == crate::compliance::checkpoint::CheckpointVerdict::Honoured;
    if !honoured {
        tracing::error!(
            target: "audit_checkpoint",
            anchor_id = checkpoint.anchor_id,
            ?verdict,
            "audit chain does not honour a recorded checkpoint"
        );
    }
    format::json(serde_json::json!({ "honoured": honoured, "verdict": verdict }))
}

/// Query string for the chain-verification endpoint.
#[derive(Debug, serde::Deserialize)]
struct VerifyParams {
    /// How many trailing rows to verify; clamped to `[1, 10_000]`.
    limit: Option<u64>,
}

impl VerifyParams {
    /// The effective row limit. Verification is O(rows) with a SHA-256
    /// each, so an unbounded `limit` is a CPU denial-of-service.
    fn limit(&self) -> u64 {
        self.limit.unwrap_or(1000).clamp(1, 10_000)
    }
}

/// Map a refused read-audit write to `503 Service Unavailable`.
///
/// `503` rather than `500`: nothing is wrong with the request, and
/// nothing was disclosed — the service is temporarily unable to account
/// for a read of personal data, so it declines to serve one. Only
/// reachable when `CASE_AUDIT_FAIL_CLOSED` is on.
fn audit_unavailable(_: disclosure::AuditWriteRefused) -> Error {
    Error::CustomError(
        StatusCode::SERVICE_UNAVAILABLE,
        ErrorDetail::new(
            "audit_unavailable",
            "the access could not be recorded in the audit trail, so the read was refused",
        ),
    )
}

/// Build the `/api/cases` route table (CRUD + match +
/// check-duplicates + merge + audit / event + whoami endpoints).
///
/// Order matters: the literal sub-paths (`/search`, `/match`, …,
/// `/audit/recent`) are registered before the `/{pid}` catch-alls so they
/// are not shadowed by the path parameter.
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
        .add("/audit/verify", get(verify_audit_chain))
        .add("/records/verify", get(verify_records))
        .add("/checkpoint", get(take_checkpoint))
        .add("/checkpoint/verify", post(verify_checkpoint))
        .add("/events/recent", get(recent_events))
        .add("/{pid}", get(get_one))
        .add("/{pid}", put(update))
        .add("/{pid}", delete(remove))
        .add("/{pid}/masked", get(get_masked))
        .add("/{pid}/export", get(get_export))
        .add("/{pid}/audit", get(entity_audit))
        .add("/{pid}/audit/disclosures", get(entity_disclosures))
        .add("/{pid}/erase", post(erase))
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

    /// The `mask` obligation's redaction drops the involved-party
    /// linkages (`subjects` / `identifiers` / `same_as` / case number)
    /// and keeps the descriptive shell.
    #[test]
    fn mask_case_redacts_involved_parties() {
        let mut case = Case::new("Housing benefit appeal");
        case.subjects = vec!["person:123".to_string()];
        case.case_number = Some("HB-2026-01".to_string());
        case.same_as = vec!["https://court.example/rec/1".to_string()];
        let masked = mask_case(&case);
        assert!(masked.subjects.is_empty());
        assert!(masked.same_as.is_empty());
        assert_eq!(masked.case_number, None);
        // Descriptive shell preserved.
        assert_eq!(masked.title, "Housing benefit appeal");
    }

    /// The 422 must survive loco's error-to-response conversion.
    #[test]
    fn blank_title_validation_response_status_is_422() {
        let err = validate(&Case::new("")).expect_err("blank title must fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// The complement: a real title passes validation (no false positives).
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

    /// Pins the `check-duplicates` ranking comparator: it orders hits
    /// best-score-first. Runs without a database, so the ordering the
    /// handler applies is pinned un-gated.
    #[test]
    fn score_desc_orders_higher_scores_first() {
        use std::cmp::Ordering;
        assert_eq!(score_desc(0.9, 0.4), Ordering::Less); // 0.9 sorts before 0.4
        assert_eq!(score_desc(0.4, 0.9), Ordering::Greater);
        assert_eq!(score_desc(0.5, 0.5), Ordering::Equal);
    }

    /// A NaN score must not panic the sort — `score_desc` treats it as
    /// `Equal` (matcher scores are finite, but this guards the `unwrap`).
    #[test]
    fn score_desc_treats_nan_as_equal() {
        assert_eq!(score_desc(f64::NAN, 0.5), std::cmp::Ordering::Equal);
        assert_eq!(score_desc(0.5, f64::NAN), std::cmp::Ordering::Equal);
    }

    /// Pins the `check-duplicates` ranking end to end *without a database*:
    /// the per-row scoring + `score_desc` sort that the handler runs
    /// in-memory must place a deterministic docket twin (score 1.0,
    /// `is_match`) ahead of an unrelated case, mirroring the
    /// Postgres-gated `can_check_duplicates_against_stored_cases` request
    /// test (spec §11; testGap: the in-memory scan + `ScoredRef` sort had
    /// no DB-free coverage).
    #[test]
    fn check_duplicates_ranking_is_deterministic_and_sorted() {
        let engine = MatchingEngine::new(MatchConfig::default());

        // The query shares a docket with the first stored row only.
        let query = Case {
            identifiers: vec![case_matcher::CaseIdentifier {
                scheme: case_matcher::IdentifierScheme::Docket,
                value: "cv-2024-001234".into(),
            }],
            ..Case::new("HB appeal — J. Smith")
        };

        // Two "stored" rows: a docket twin and an unrelated case.
        let twin = Case {
            identifiers: vec![case_matcher::CaseIdentifier {
                scheme: case_matcher::IdentifierScheme::Docket,
                value: "CV-2024-001234".into(), // same docket, different case
            }],
            ..Case::new("Housing benefit appeal")
        };
        let unrelated = Case::new("Tax credit overpayment");
        let stored = [("twin", &twin), ("unrelated", &unrelated)];

        // Reproduce the handler's in-memory scan: score each row, keep
        // matches, then sort with the shared comparator.
        let mut hits: Vec<(&str, f64, bool)> = stored
            .iter()
            .map(|(label, c)| {
                let r = engine.match_cases(&query, c);
                (*label, r.score, r.is_match)
            })
            .filter(|(_, _, is_match)| *is_match)
            .collect();
        hits.sort_by(|a, b| score_desc(a.1, b.1));

        assert_eq!(hits.len(), 1, "only the docket twin should match");
        assert_eq!(hits[0].0, "twin");
        assert!(
            (hits[0].1 - 1.0).abs() < f64::EPSILON,
            "same docket is a deterministic 1.0"
        );
        assert!(hits[0].2, "the twin is a match");
    }
}
