//! Case CRUD + matching endpoints.
//!
//! The API DTO is `case_matcher::Case` itself — the service stores it
//! verbatim (as JSON) and matches with the canonical `case-matcher`
//! engine, so there is no separate model or adapter to drift.

use authentication_verifier::Action;
use axa::cursor_pagination::{CursorLimits, CursorPage};
use axa::error::ErrorDetail;
use axa::extract::{Json, Path, Query};
use axa::prelude::*;
use axum::extract::OriginalUri;
use axum::http::StatusCode;
use case_matcher::{Case, Evidence, MatchConfig, MatchStrength, MatchingEngine};

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

/// Historical cap on how many stored cases `check-duplicates` once
/// scanned in-memory, before spec §13 T-6's search-blocked candidates
/// landed.
///
/// `check-duplicates` no longer scans at all — it blocks on the search
/// index instead (see [`check_duplicates`] and
/// [`CHECK_DUPLICATES_CANDIDATE_LIMIT`]); a unit test still pins its
/// historical value. [`deduplicate`] (spec §13 T-7) reuses it for a
/// different purpose: the maximum number of active cases loaded as
/// **scan seeds** for the batch dedup pass.
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

/// Largest page any collection read will serve. A bigger `limit` is a
/// `422`, never clamped, so a caller is not handed less than it asked
/// for without being told.
pub const MAX_LIMIT: u64 = 500;

/// The page sizes of a collection read whose default page is
/// `default_limit`. There is no offset bound: a position comes only from
/// a cursor this service signed, so reaching a deep page means fetching
/// every page before it.
const fn limits(default_limit: u64) -> CursorLimits {
    CursorLimits {
        default_limit,
        max_limit: MAX_LIMIT,
    }
}

/// `rows` as a bare JSON array, with the page's `Link: rel="next"` when
/// `more` says rows follow.
///
/// `more` is decided by the caller from the batch it fetched, before any
/// row is dropped by concealment or by resolving an index hit, so a page
/// shortened by either never ends the walk early.
fn with_next<T: Serialize>(page: &CursorPage, more: bool, rows: Vec<T>) -> Response {
    let mut response = Json(rows).into_response();
    if more && let Some(link) = page.next_link() {
        response
            .headers_mut()
            .insert(axum::http::header::LINK, link);
    }
    response
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

/// Validate an incoming `Case` payload.
///
/// Family convention (OQ-1 resolution): validation failures return
/// `422 Unprocessable Entity`, matching the person/place services.
/// axa has no `unprocessable_entity` helper, so this uses
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

/// Query string for `GET /api/cases/search`: the `q` title term and the
/// retrieval-mode flags. The page, `limit` and `cursor`, is read from the
/// URI by [`CursorPage::new`], and ignored here.
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
    /// The matcher's evidence for the pair (`agents/share/match-evidence.md`);
    /// reduced to its aggregate when the caller may read the candidate
    /// only masked ([`redact_evidence`]).
    evidence: Evidence,
}

/// Default score at or above which a batch-scan pair may be marked
/// `automerged` — the matcher's "certain" band. Score alone is never
/// enough: see [`auto_merge_allowed`].
pub const DEFAULT_AUTO_MERGE_THRESHOLD: f64 = 0.95;

/// The auto-merge gate (`agents/share/match-evidence.md` §4): a pair may
/// be marked or executed as an automatic merge only when its score
/// reaches `auto_merge_threshold` **and** its evidence is
/// [`MatchStrength::Proven`]. A `consistent` pair (nothing contradicts,
/// nothing proves — two different appeals filed under one template
/// title) and a `contradicted` one (two different dockets) go to review
/// as `pending`, whatever their score.
#[must_use]
pub fn auto_merge_allowed(score: f64, auto_merge_threshold: f64, evidence: &Evidence) -> bool {
    score >= auto_merge_threshold && evidence.strength == MatchStrength::Proven
}

/// Evidence for a pair the caller may read only masked, or not at all:
/// the aggregate (`weight_bits`, `fields_compared`, `strength`) with
/// `contradictions` and `components` emptied.
///
/// Evidence holds component names and bits, never field values, but on a
/// case those names still disclose what a masked view hides: a
/// `contradiction:identifier:docket` term says the hidden case holds a
/// docket, and a `subjects` or `case_number` term says the hidden
/// subjects or case number were compared. The aggregate is no more than
/// the score already discloses, and it keeps the strength the auto-merge
/// gate and a reviewer need.
#[must_use]
pub fn redact_evidence(evidence: &Evidence) -> Evidence {
    Evidence {
        weight_bits: evidence.weight_bits,
        fields_compared: evidence.fields_compared,
        contradictions: Vec::new(),
        strength: evidence.strength,
        components: Vec::new(),
    }
}

/// Whether a record-level read decision's obligations ask for the
/// masked view.
fn is_masked(obligations: &[String]) -> bool {
    obligations.iter().any(|o| o == "mask")
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
/// axa error response. `403` = the policy denied the action given the
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

/// List active cases, one page at a time. `GET /api/cases`.
///
/// `GET /api/cases[?limit=&cursor=]`. Responds `200` with an array of
/// [`CaseRef`] (`{pid, title}`), most-recent first, and a
/// `Link: rel="next"` header while more rows follow
/// (`agents/share/restful.md`). Omitting `limit` serves
/// [`LIST_DEFAULT_LIMIT`] rows, the cap this endpoint applied before it
/// was paginated. There is no total.
///
/// # Errors
///
/// `422` when `limit` exceeds [`MAX_LIMIT`] or the cursor is not one this
/// service signed for this query; a DB error when the query fails.
#[debug_handler]
async fn list(
    OriginalUri(uri): OriginalUri,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let page = CursorPage::new(crate::cursor_key::key(), limits(LIST_DEFAULT_LIMIT), &uri)?;
    let rows = CaseModel::list_paged(&ctx.db, page.limit() + 1, page.offset()).await?;
    // Whether more follow is the table's answer, decided on the fetched
    // batch before concealment below drops any row: a page shortened by
    // concealment must not end the walk, and a caller-specific answer
    // would leak how many records they may not see.
    let (rows, more_follow) = page.split(rows);
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
    Ok(with_next(&page, more_follow, refs))
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
    OriginalUri(uri): OriginalUri,
    Query(params): Query<SearchParams>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let q = params.q.unwrap_or_default();
    if q.trim().is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let page = CursorPage::new(crate::cursor_key::key(), limits(SEARCH_DEFAULT_LIMIT), &uri)?;
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
    let (pids, _total) = engine.search_page(
        q.trim(),
        mode,
        usize::try_from(page.limit() + 1).unwrap_or(usize::MAX),
        usize::try_from(page.offset()).unwrap_or(usize::MAX),
    )?;
    // Whether more follow is the index's answer, decided before the hits
    // resolve and before concealment: a hit whose row has since been
    // deleted drops out on resolve (see `find_by_pids`), a concealed row
    // drops out below, and neither may end the walk.
    let (pids, more_follow) = page.split(pids);
    let rows = CaseModel::find_by_pids(&ctx.db, &parse_pids(&pids)).await?;
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
    Ok(with_next(&page, more_follow, refs))
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
    // The cached registry-wide value frequencies (match-evidence.md §7).
    let frequencies = crate::frequencies::current(&ctx.db).await;
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_case()?;
        // SEC-G3: a caller must not discover a concealed case via dedup —
        // skip candidates they may not read.
        let Some(obligations) = crate::auth::read_visibility(&caller, &candidate) else {
            continue;
        };
        let r = engine.match_cases_with_frequencies(&query, &candidate, frequencies.as_ref());
        if r.is_match {
            let evidence = if is_masked(&obligations) {
                redact_evidence(&r.evidence)
            } else {
                r.evidence
            };
            hits.push(ScoredRef {
                pid: row.pid.to_string(),
                title: row.title.clone(),
                score: r.score,
                confidence: format!("{:?}", r.confidence),
                is_match: r.is_match,
                evidence,
            });
        }
    }
    // Best score first (see [`score_desc`]).
    hits.sort_by(|a, b| score_desc(a.score, b.score));
    format::json(hits)
}

/// Request body for the batch scan. With no `threshold` the matcher's
/// own `is_match` verdict decides; with one, `score >= threshold` does.
#[derive(Debug, Deserialize)]
struct BatchDeduplicationRequest {
    /// Optional score threshold overriding the matcher's verdict.
    #[serde(default)]
    threshold: Option<f64>,
    /// Score at or above which a `proven` pair is marked `automerged`
    /// (default [`DEFAULT_AUTO_MERGE_THRESHOLD`]); see
    /// [`auto_merge_allowed`].
    #[serde(default = "default_auto_merge_threshold")]
    auto_merge_threshold: f64,
}

impl Default for BatchDeduplicationRequest {
    fn default() -> Self {
        Self {
            threshold: None,
            auto_merge_threshold: DEFAULT_AUTO_MERGE_THRESHOLD,
        }
    }
}

/// serde default for [`BatchDeduplicationRequest::auto_merge_threshold`].
const fn default_auto_merge_threshold() -> f64 {
    DEFAULT_AUTO_MERGE_THRESHOLD
}

/// Response for the batch scan, in the family's report shape
/// (`agents/share/match-search-merge.md` "Duplicate detection").
#[derive(Debug, Serialize)]
struct BatchDeduplicationResponse {
    /// Number of cases loaded as scan seeds.
    cases_scanned: usize,
    /// Number of duplicate pairs found (stored rows reported).
    duplicates_found: usize,
    /// Stored pairs marked `automerged` (score at or above
    /// `auto_merge_threshold` and evidence `proven`). A mark only: the
    /// scan never executes a merge.
    auto_merged: usize,
    /// Number of stored pairs currently `pending` review.
    queued_for_review: usize,
    /// The stored candidate pairs (stable ids across re-scans).
    review_items: Vec<ReviewQueueItem>,
}

/// The review-queue row for one batch-scan pair, with the auto-merge gate
/// applied: `automerged` only when [`auto_merge_allowed`], else `pending`.
fn batch_review_item(
    pair: (uuid::Uuid, uuid::Uuid),
    r: &case_matcher::MatchResult,
    auto_merge_threshold: f64,
) -> crate::models::review_queue::NewReviewItem {
    let status = if auto_merge_allowed(r.score, auto_merge_threshold, &r.evidence) {
        ReviewStatus::AutoMerged
    } else {
        ReviewStatus::Pending
    };
    crate::models::review_queue::NewReviewItem {
        record_id_a: pair.0,
        record_id_b: pair.1,
        match_score: r.score,
        match_quality: format!("{:?}", r.confidence).to_lowercase(),
        detection_method: "batch_deduplication".to_string(),
        score_breakdown: serde_json::to_value(&r.breakdown).ok(),
        status: review_status_token(status).to_string(),
        provenance: "operator".to_string(),
        evidence: serde_json::to_value(&r.evidence).ok(),
    }
}

/// Scan the stored cases for likely duplicates and queue them for review.
/// `POST /api/cases/deduplicate` (a destructive-classed POST under ABAC,
/// like merge).
///
/// Loads up to [`CHECK_DUPLICATES_SCAN_CAP`] active cases as scan seeds,
/// and for each seed asks the search index for its own **blocked**
/// candidates — the same fuzzy-title/exact-identifier/phonetic routes,
/// capped at [`CHECK_DUPLICATES_CANDIDATE_LIMIT`], that [`check_duplicates`]
/// scores a single query against — rather than comparing every pair in
/// the corpus. This is the same scale-cliff fix `check-duplicates`
/// already applies (spec §13 T-6), extended to a full scan: a full O(n²)
/// pairwise compare would reintroduce exactly the "record 1001 is
/// unreachable" cliff Tantivy blocking was built to remove. A candidate
/// named by the index but outside the scan-seed page is fetched on
/// demand (the same batch fetch [`check_duplicates`] uses), so a corpus
/// larger than the cap still finds pairs where at least one side is a
/// seed. Each unordered pair is scored at most once per request; hits
/// above threshold are **persisted** in the stored `review_queue`
/// (normalized-pair upsert: re-scans refresh scores, decided rows keep
/// their decision, ids stay stable). The response reports the STORED
/// rows. Does not merge anything.
///
/// # Errors
///
/// `503` when the search index is unavailable; a DB error on the
/// row/candidate fetch or the queue upsert; a JSON parse error when a
/// stored payload cannot be deserialized.
#[debug_handler]
async fn deduplicate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<BatchDeduplicationRequest>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let Some(index) = crate::search::engine() else {
        return Err(Error::CustomError(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorDetail::new(
                "search_unavailable",
                "the search index is unavailable, so duplicates cannot be checked",
            ),
        ));
    };
    let rows = CaseModel::list(&ctx.db, CHECK_DUPLICATES_SCAN_CAP).await?;
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "deduplicate scan hit the row cap; cases beyond the cap are \
             not used as scan seeds (though one can still surface as a \
             candidate of a seed within the cap). Run it more often, or \
             narrow the corpus."
        );
    }
    let mut known: std::collections::HashMap<uuid::Uuid, Case> = std::collections::HashMap::new();
    let mut seeds = Vec::with_capacity(rows.len());
    for row in &rows {
        let case = row.to_case()?;
        known.insert(row.pid, case.clone());
        seeds.push((row.pid, case));
    }
    // Value frequencies from the records this scan loaded
    // (match-evidence.md §7); `NoFrequencies` below the population floor.
    let frequencies = crate::frequencies::CaseFrequencies::from_cases(seeds.iter().map(|(_, c)| c));

    let mut seen_pairs = std::collections::HashSet::new();
    let mut new_items = Vec::new();
    for (pid, case) in &seeds {
        let candidate_ids = index.candidates(case, CHECK_DUPLICATES_CANDIDATE_LIMIT)?;
        for cand_pid in parse_pids(&candidate_ids) {
            if cand_pid == *pid {
                continue; // never pair a case with itself
            }
            // Normalize so (a, b) and (b, a) are the same unordered pair.
            let pair = if *pid <= cand_pid {
                (*pid, cand_pid)
            } else {
                (cand_pid, *pid)
            };
            if !seen_pairs.insert(pair) {
                continue; // already scored this pair earlier in this scan
            }
            let candidate = if let Some(c) = known.get(&cand_pid) {
                c.clone()
            } else {
                // The index can name a row outside the scan-seed page;
                // fetch it on demand (same batch fetch check_duplicates
                // uses).
                let Some(fetched) = CaseModel::find_by_pids(&ctx.db, &[cand_pid])
                    .await?
                    .into_iter()
                    .next()
                else {
                    continue; // deleted / reindex lag; skip rather than fail the scan
                };
                let c = fetched.to_case()?;
                known.insert(cand_pid, c.clone());
                c
            };
            let r = engine.match_cases_with_frequencies(case, &candidate, &frequencies);
            if req.threshold.map_or(r.is_match, |t| r.score >= t) {
                new_items.push(batch_review_item(pair, &r, req.auto_merge_threshold));
            }
        }
    }
    let stored = crate::models::review_queue::upsert(&ctx.db, &new_items).await?;
    // Every case in a stored pair is in `known` (seed or fetched).
    let hidden = hidden_pids(&caller, &stored, |pid| known.get(&pid).cloned());
    let review_items: Vec<ReviewQueueItem> = stored
        .iter()
        .map(|row| review_row_to_item(row, &hidden))
        .collect();
    let queued_for_review = review_items
        .iter()
        .filter(|i| i.status == ReviewStatus::Pending)
        .count();
    let auto_merged = review_items
        .iter()
        .filter(|i| i.status == ReviewStatus::AutoMerged)
        .count();
    format::json(BatchDeduplicationResponse {
        cases_scanned: seeds.len(),
        duplicates_found: review_items.len(),
        auto_merged,
        queued_for_review,
        review_items,
    })
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
    // `Deleted` pair in one transaction that locks both rows and
    // re-checks both are active, so a merge that lost a race to another
    // answers 404 and writes nothing (SEC-B5, CS-M1).
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
    Query(params): Query<VerifyParams>,
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
    let found = CaseModel::find_by_pid(&ctx.db, &pid).await;
    // CS-G1: one transaction for every step, so a failure part-way leaves
    // the case exactly as it was rather than half erased (tombstoned with
    // its `subject_of` edge still standing, or redacted with no `erased`
    // row saying why). On an early return the transaction drops and rolls
    // back; the chained `erased` row is inside it, so it commits only with
    // the erasure it records.
    let mut txn = ctx.db.begin().await?;
    let outcome = match found {
        Ok(model) => {
            crate::compliance::erasure::erase(&mut txn, model, caller.actor(), &access).await?
        }
        // No live record: still sweep the audit content held about it.
        Err(_) => {
            crate::compliance::erasure::erase_audit_only(&mut txn, uuid, caller.actor(), &access)
                .await?
        }
    };
    txn.commit().await?;
    // CS-E1: the index holds its own copy of the title, subjects and
    // identifiers, so the erased content must leave it too. After the
    // commit, as for soft delete: the database is the source of truth, and
    // a failed removal is logged and repaired by `search_reindex`, which
    // indexes only live rows.
    crate::streaming::deindex_best_effort(uuid);
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

// ─── Stored review queue (T-8) ──────────────────────────────────────────────
//
// `src/models/review_queue.rs` already has upsert/list/decide (added for
// the BLK-5 bulk-import pipeline, which routes a keyless import row's
// likely duplicate here with `provenance = "import"`), but nothing
// exposed it over the API. Every sibling entity that has a review queue
// (person / worker / place / thing / organization,
// `agents/share/match-search-merge.md` "Review queue") exposes
// `GET /api/<plural>/review-queue` and
// `POST /api/<plural>/review-queue/{id}/decision`; this closes the same
// gap for case. [`deduplicate`] (T-7, above) is the other writer of this
// queue — before it landed, these two endpoints only surfaced rows the
// bulk-import pipeline had written.

/// Review disposition of one queued duplicate pair, in the family's
/// lowercase wire tokens (matching person/worker/place/thing/organization).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReviewStatus {
    /// Awaiting manual review.
    Pending,
    /// Confirmed as a duplicate — ready for merge.
    Confirmed,
    /// Rejected — not a duplicate.
    Rejected,
    /// Marked as an automatic merge by the batch scan: score at or above
    /// `auto_merge_threshold` **and** evidence `proven`
    /// ([`auto_merge_allowed`]). A mark only; no merge is executed.
    AutoMerged,
}

/// The lowercase wire token for a review status.
fn review_status_token(status: ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::Pending => "pending",
        ReviewStatus::Confirmed => "confirmed",
        ReviewStatus::Rejected => "rejected",
        ReviewStatus::AutoMerged => "automerged",
    }
}

/// Parse a stored status token (unknown tokens read as `pending`).
fn parse_review_status(token: &str) -> ReviewStatus {
    match token {
        "confirmed" => ReviewStatus::Confirmed,
        "rejected" => ReviewStatus::Rejected,
        "automerged" => ReviewStatus::AutoMerged,
        _ => ReviewStatus::Pending,
    }
}

/// One stored review-queue item, in the family's person/worker shape
/// (`detection_method` included) so the front-ends render one queue.
#[derive(Debug, Serialize)]
struct ReviewQueueItem {
    /// Stable review-item id (survives re-scans).
    id: String,
    /// First case in the candidate pair (public id).
    case_id_a: String,
    /// Second case in the candidate pair (public id).
    case_id_b: String,
    /// Overall match score for the pair, in `[0.0, 1.0]`.
    match_score: f64,
    /// Confidence band label (matcher's `Confidence`, lowercased).
    match_quality: String,
    /// How the pair was detected.
    detection_method: String,
    /// Per-component score breakdown, as stored at detection time.
    score_breakdown: Option<serde_json::Value>,
    /// Current review state.
    status: ReviewStatus,
    /// How the pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested`; BLK-5).
    provenance: String,
    /// The matcher's evidence for the pair, refreshed on each re-scan;
    /// `null` for a row queued before evidence was recorded. Reduced to
    /// its aggregate ([`redact_evidence`]) when either case is one the
    /// caller may read only masked, or not at all.
    evidence: Option<Evidence>,
    /// Reviewer identity recorded by the decision endpoint, if decided.
    reviewed_by: Option<String>,
    /// When the pair was first queued.
    created_at: chrono::DateTime<chrono::Utc>,
    /// When the decision was recorded, if decided.
    reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// The cases, among those named by `rows`, whose evidence must be
/// redacted for `caller`: those it may read only masked, may not read,
/// or that `load` cannot supply (deleted since queued). Empty when
/// record-level enforcement is off, so the default posture is unchanged.
fn hidden_pids(
    caller: &MaybeAuthUser,
    rows: &[crate::models::review_queue::ReviewQueueRow],
    load: impl Fn(uuid::Uuid) -> Option<Case>,
) -> std::collections::HashSet<uuid::Uuid> {
    let mut hidden = std::collections::HashSet::new();
    if !crate::auth::require_auth() {
        return hidden;
    }
    for pid in rows.iter().flat_map(|r| [r.record_id_a, r.record_id_b]) {
        let visible = load(pid)
            .and_then(|case| crate::auth::read_visibility(caller, &case))
            .is_some_and(|obligations| !is_masked(&obligations));
        if !visible {
            hidden.insert(pid);
        }
    }
    hidden
}

/// [`hidden_pids`] for rows whose cases are not loaded yet: one batch
/// fetch of every case they name, skipped when enforcement is off.
async fn hidden_pids_from_db(
    db: &sqlx::PgPool,
    caller: &MaybeAuthUser,
    rows: &[crate::models::review_queue::ReviewQueueRow],
) -> Result<std::collections::HashSet<uuid::Uuid>> {
    if !crate::auth::require_auth() || rows.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let pids: Vec<uuid::Uuid> = rows
        .iter()
        .flat_map(|r| [r.record_id_a, r.record_id_b])
        .collect();
    let mut cases = std::collections::HashMap::new();
    for model in CaseModel::find_by_pids(db, &pids).await? {
        if let Ok(case) = model.to_case() {
            cases.insert(model.pid, case);
        }
    }
    Ok(hidden_pids(caller, rows, |pid| cases.get(&pid).cloned()))
}

/// Map a stored review-queue row onto the wire item shape. `hidden` is
/// [`hidden_pids`]: when either case is in it, the evidence is reduced to
/// its aggregate.
fn review_row_to_item(
    row: &crate::models::review_queue::ReviewQueueRow,
    hidden: &std::collections::HashSet<uuid::Uuid>,
) -> ReviewQueueItem {
    let evidence = row
        .evidence
        .clone()
        .and_then(|v| serde_json::from_value::<Evidence>(v).ok())
        .map(|e| {
            if hidden.contains(&row.record_id_a) || hidden.contains(&row.record_id_b) {
                redact_evidence(&e)
            } else {
                e
            }
        });
    ReviewQueueItem {
        id: row.id.to_string(),
        case_id_a: row.record_id_a.to_string(),
        case_id_b: row.record_id_b.to_string(),
        match_score: row.match_score,
        match_quality: row.match_quality.clone(),
        detection_method: row.detection_method.clone(),
        score_breakdown: row.score_breakdown.clone(),
        status: parse_review_status(&row.status),
        provenance: row.provenance.clone(),
        evidence,
        reviewed_by: row.reviewed_by.clone(),
        created_at: row.created_at,
        reviewed_at: row.reviewed_at,
    }
}

/// Query parameters for the review-queue list endpoint. The page,
/// `limit` and `cursor`, is read from the URI by [`CursorPage::new`], and
/// ignored here.
#[derive(Debug, Deserialize)]
struct ReviewQueueListQuery {
    /// Optional status-token filter (`pending` / `confirmed` /
    /// `rejected` / `automerged`).
    status: Option<String>,
}

/// Response for the review-queue list endpoint.
#[derive(Debug, Serialize)]
struct ReviewQueueListResponse {
    /// The stored review-queue items on this page (newest first).
    items: Vec<ReviewQueueItem>,
    /// Number of items on this page, not in the queue: whether more
    /// follow is the `Link: rel="next"` header's answer.
    total: usize,
}

/// List the stored duplicate review queue (newest first).
///
/// `GET /api/cases/review-queue[?status=&limit=&cursor=]`, one page at a
/// time by signed cursor like `GET /api/cases`: a `Link: rel="next"`
/// header while more rows follow, and a `422` for a `limit` above
/// [`MAX_LIMIT`], a cursor this service did not sign, or an unknown
/// status token. Whether more follow is decided before any record id is
/// concealed.
#[debug_handler]
async fn get_review_queue(
    OriginalUri(uri): OriginalUri,
    Query(query): Query<ReviewQueueListQuery>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    if let Some(status) = query.status.as_deref()
        && !matches!(status, "pending" | "confirmed" | "rejected" | "automerged")
    {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new(
                "unprocessable_entity",
                format!("unknown review status `{status}`"),
            ),
        ));
    }
    let page = CursorPage::new(crate::cursor_key::key(), limits(LIST_DEFAULT_LIMIT), &uri)?;
    let rows = crate::models::review_queue::list(
        &ctx.db,
        query.status.as_deref(),
        page.offset(),
        page.limit() + 1,
    )
    .await?;
    let (rows, more_follow) = page.split(rows);
    let hidden = hidden_pids_from_db(&ctx.db, &caller, &rows).await?;
    let items: Vec<ReviewQueueItem> = rows
        .iter()
        .map(|row| review_row_to_item(row, &hidden))
        .collect();
    let total = items.len();
    let mut response = Json(ReviewQueueListResponse { items, total }).into_response();
    if more_follow && let Some(link) = page.next_link() {
        response
            .headers_mut()
            .insert(axum::http::header::LINK, link);
    }
    Ok(response)
}

/// One operator verdict for a `pending` review item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReviewDecision {
    /// Confirm the pair as a duplicate (ready for merge).
    Confirmed,
    /// Reject the pair (not a duplicate).
    Rejected,
}

/// Request body for the review-decision endpoint.
#[derive(Debug, Deserialize)]
struct ReviewDecisionRequest {
    /// The verdict (`confirmed` or `rejected`).
    status: ReviewDecision,
}

/// Decide one `pending` review item (`confirmed` or `rejected`).
///
/// `POST /api/cases/review-queue/{id}/decision`. The transition guard is
/// first-writer-wins in SQL: only a `pending` item can be decided; an
/// already-decided item is `422`, an unknown id `404`. The reviewer
/// identity is the verified bearer `sub` when present, and each decision
/// writes a `review_decision` audit row.
#[debug_handler]
async fn review_decision(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<ReviewDecisionRequest>,
) -> Result<Response> {
    let token = review_status_token(match req.status {
        ReviewDecision::Confirmed => ReviewStatus::Confirmed,
        ReviewDecision::Rejected => ReviewStatus::Rejected,
    });
    let reviewed_by = caller.0.as_ref().map(|c| c.sub.clone());
    match crate::models::review_queue::decide(&ctx.db, id, token, reviewed_by.as_deref()).await? {
        crate::models::review_queue::DecideOutcome::Decided(row) => {
            // A decision is a review-state mutation: record it on the
            // audit trail (best-effort, same posture as CRUD audits).
            if let Err(err) = AuditModel::record(
                &ctx.db,
                id,
                "review_decision",
                reviewed_by.as_deref(),
                Some(serde_json::json!({ "status": token })),
            )
            .await
            {
                tracing::warn!("review-decision audit write failed: {err}");
            }
            let rows = [*row];
            let hidden = hidden_pids_from_db(&ctx.db, &caller, &rows).await?;
            format::json(review_row_to_item(&rows[0], &hidden))
        }
        crate::models::review_queue::DecideOutcome::NotFound => Err(Error::NotFound),
        crate::models::review_queue::DecideOutcome::AlreadyDecided(current) => {
            Err(Error::CustomError(
                StatusCode::UNPROCESSABLE_ENTITY,
                ErrorDetail::new(
                    "unprocessable_entity",
                    format!("item is `{current}`; only `pending` items can be decided"),
                ),
            ))
        }
    }
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
        .add("/deduplicate", post(deduplicate))
        .add("/merge", post(merge))
        .add("/merges/recent", get(recent_merges))
        .add("/review-queue", get(get_review_queue))
        .add("/review-queue/{id}/decision", post(review_decision))
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

    /// This crate's bounds: no `limit` is the default, a `limit` up to
    /// [`MAX_LIMIT`] is taken, and anything else is refused, never clamped.
    #[test]
    fn a_page_takes_its_limit_or_refuses_it() {
        let page = |query: &str, default_limit: u64| {
            let uri = format!("/api/cases{query}").parse().unwrap();
            CursorPage::new(crate::cursor_key::key(), limits(default_limit), &uri)
        };
        assert_eq!(
            page("", LIST_DEFAULT_LIMIT).unwrap().limit(),
            LIST_DEFAULT_LIMIT
        );
        assert_eq!(
            page("", SEARCH_DEFAULT_LIMIT).unwrap().limit(),
            SEARCH_DEFAULT_LIMIT
        );
        assert_eq!(
            page(&format!("?limit={MAX_LIMIT}"), LIST_DEFAULT_LIMIT)
                .unwrap()
                .limit(),
            MAX_LIMIT
        );
        for refused in [
            "?limit=0",
            "?limit=lots",
            &format!("?limit={}", MAX_LIMIT + 1),
        ] {
            assert!(page(refused, LIST_DEFAULT_LIMIT).is_err(), "{refused}");
        }
    }

    /// A cursor this service did not sign, such as a caller's own offset
    /// in disguise, is refused rather than read.
    #[test]
    fn an_unsigned_cursor_is_refused() {
        let uri = "/api/cases?cursor=AQAAAAAAAAAK".parse().unwrap();
        assert!(
            CursorPage::new(crate::cursor_key::key(), limits(LIST_DEFAULT_LIMIT), &uri).is_err()
        );
    }

    /// The next link is only sent while more rows follow.
    #[test]
    fn the_next_link_follows_more() {
        let uri = "/api/cases/search?q=housing&limit=2".parse().unwrap();
        let page =
            CursorPage::new(crate::cursor_key::key(), limits(SEARCH_DEFAULT_LIMIT), &uri).unwrap();
        let more = with_next(&page, true, vec!["a", "b"]);
        let link = more.headers()["link"].to_str().unwrap();
        assert!(
            link.starts_with("</api/cases/search?q=housing&limit=2&cursor="),
            "{link}"
        );
        assert!(
            with_next(&page, false, vec!["a"])
                .headers()
                .get("link")
                .is_none()
        );
    }

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

    /// The 422 must survive axa's error-to-response conversion.
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

    fn docket(value: &str) -> case_matcher::CaseIdentifier {
        case_matcher::CaseIdentifier {
            scheme: case_matcher::IdentifierScheme::Docket,
            value: value.to_string(),
        }
    }

    /// The auto-merge gate (match-evidence.md §4): title-only agreement
    /// scores above the threshold but is `consistent`, so it stays
    /// pending; a shared docket is `proven` and may be marked; two
    /// different dockets are `contradicted` and stay pending.
    #[test]
    fn auto_merge_needs_proven_evidence_not_just_score() {
        let engine = MatchingEngine::new(MatchConfig::default());
        let pair = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());

        let title_only = engine.match_cases(
            &Case::new("Housing benefit appeal"),
            &Case::new("Housing benefit appeal"),
        );
        assert!(title_only.score >= DEFAULT_AUTO_MERGE_THRESHOLD);
        assert_eq!(title_only.evidence.strength, MatchStrength::Consistent);
        assert!(!auto_merge_allowed(
            title_only.score,
            DEFAULT_AUTO_MERGE_THRESHOLD,
            &title_only.evidence
        ));
        assert_eq!(
            batch_review_item(pair, &title_only, DEFAULT_AUTO_MERGE_THRESHOLD).status,
            "pending"
        );

        let with = |d: &str| Case {
            identifiers: vec![docket(d)],
            ..Case::new("Housing benefit appeal")
        };
        let shared = engine.match_cases(&with("CV-1"), &with("cv-1"));
        assert_eq!(shared.evidence.strength, MatchStrength::Proven);
        let item = batch_review_item(pair, &shared, DEFAULT_AUTO_MERGE_THRESHOLD);
        assert_eq!(item.status, "automerged");
        assert_eq!(item.evidence.expect("evidence")["strength"], "proven");
        // Proven, but below the score threshold: still pending.
        assert_eq!(batch_review_item(pair, &shared, 1.01).status, "pending");

        let conflict = engine.match_cases(&with("CV-1"), &with("CV-2"));
        assert_eq!(conflict.evidence.strength, MatchStrength::Contradicted);
        assert!(conflict.score >= DEFAULT_AUTO_MERGE_THRESHOLD);
        assert_eq!(
            batch_review_item(pair, &conflict, DEFAULT_AUTO_MERGE_THRESHOLD).status,
            "pending"
        );
    }

    /// Masked evidence keeps the aggregate and drops every term that
    /// names a masked field, so it never reveals that a hidden case
    /// holds a docket.
    #[test]
    fn redacted_evidence_discloses_no_scheme() {
        let engine = MatchingEngine::new(MatchConfig::default());
        let with = |d: &str| Case {
            identifiers: vec![docket(d)],
            subjects: vec!["person:x".into()],
            ..Case::new("Housing benefit appeal")
        };
        let full = engine.match_cases(&with("CV-1"), &with("CV-2")).evidence;
        assert!(!full.contradictions.is_empty());
        let masked = redact_evidence(&full);
        assert!(masked.contradictions.is_empty() && masked.components.is_empty());
        assert_eq!(masked.strength, full.strength);
        assert!((masked.weight_bits - full.weight_bits).abs() < 1e-12);
        assert_eq!(masked.fields_compared, full.fields_compared);
        let json = serde_json::to_string(&masked).expect("serialises");
        assert!(
            !json.contains("docket") && !json.contains("subjects"),
            "{json}"
        );
    }

    /// A stored row from before the evidence column reads back with
    /// `evidence: null`, and stored evidence JSON written by an older
    /// matcher (no `components`) still deserialises.
    #[test]
    fn rows_without_evidence_still_read() {
        let row = crate::models::review_queue::ReviewQueueRow {
            id: uuid::Uuid::new_v4(),
            record_id_a: uuid::Uuid::new_v4(),
            record_id_b: uuid::Uuid::new_v4(),
            match_score: 0.9,
            match_quality: "probable".into(),
            detection_method: "batch_deduplication".into(),
            score_breakdown: None,
            status: "pending".into(),
            provenance: "operator".into(),
            reviewed_by: None,
            created_at: chrono::Utc::now(),
            reviewed_at: None,
            evidence: None,
        };
        let none = std::collections::HashSet::new();
        assert!(review_row_to_item(&row, &none).evidence.is_none());

        let old = crate::models::review_queue::ReviewQueueRow {
            evidence: Some(serde_json::json!({
                "weight_bits": 7.5, "fields_compared": 1,
                "contradictions": [], "strength": "consistent"
            })),
            ..row
        };
        let item = review_row_to_item(&old, &none);
        let evidence = item.evidence.expect("old evidence parses");
        assert_eq!(evidence.strength, MatchStrength::Consistent);
        assert!(evidence.components.is_empty());

        // Either side hidden ⇒ the aggregate only.
        let hidden: std::collections::HashSet<_> = [old.record_id_b].into_iter().collect();
        let with_terms = crate::models::review_queue::ReviewQueueRow {
            evidence: serde_json::to_value(
                MatchingEngine::new(MatchConfig::default())
                    .match_cases(&Case::new("A b c"), &Case::new("A b c"))
                    .evidence,
            )
            .ok(),
            ..old
        };
        assert!(
            review_row_to_item(&with_terms, &hidden)
                .evidence
                .expect("evidence")
                .components
                .is_empty()
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
