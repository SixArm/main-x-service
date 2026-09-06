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
use crate::metrics::Metrics;
use crate::models::audit_logs::Model as AuditModel;
use crate::models::merge_records::Model as MergeRecordModel;
use crate::models::organizations::Model as OrgModel;
use crate::streaming;

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
/// Delegates to [`crate::validation::problems`], which collects the blank
/// `name` rule, the non-blank `identifiers[i].value` rule, and the SEC-M1
/// input-size caps, and joins every problem into one response so the
/// operator sees them all at once.
///
/// # Errors
///
/// `Error::CustomError(422)` when the payload has any validation problem.
fn validate(org: &Organization) -> Result<()> {
    let problems = crate::validation::problems(org);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("unprocessable_entity", problems.join("; ")),
        ))
    }
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
/// `POST /api/organizations`. Body: an `Organization`. Runs real-time
/// duplicate detection first (ORG-T3, `agents/share/dataflow.md`'s
/// Create Flow): a likely-duplicate candidate makes this a `409`
/// carrying the candidate [`ScoredRef`]s (in `ErrorDetail.errors`)
/// rather than creating the row — `check-duplicates`/`deduplicate` stay
/// available as separate, explicit calls, but a caller can no longer
/// skip the check entirely by simply never making one. An **unavailable
/// search index degrades to "no duplicates found"** here, unlike
/// `check_duplicates`'s hard `503`: blocking every organization from
/// being created over an unrelated search-index hiccup would be a far
/// larger availability regression than refusing one explicit check
/// (mirrors person-service's `check_duplicates_internal`, which returns
/// an empty candidate list on a search failure rather than propagating
/// it). On success returns `200` with an `OrgRef` (`{pid, name}`); a
/// blank name is `422`. Writes an audit row and publishes a `Created`
/// event (both best-effort, stamped with the caller `actor` when a
/// token was presented).
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(org): Json<Organization>,
) -> Result<Response> {
    validate(&org)?;
    if let Some(index) = crate::search::engine() {
        let candidates = score_against_index(&ctx, index, &org).await?;
        if !candidates.is_empty() {
            return Err(Error::CustomError(
                StatusCode::CONFLICT,
                ErrorDetail {
                    error: Some("duplicate_detected".to_string()),
                    description: Some(
                        "potential duplicate organizations found; review matches before creating"
                            .to_string(),
                    ),
                    errors: serde_json::to_value(&candidates).ok(),
                },
            ));
        }
    }
    // Write + `Created` event, atomic under the active transport (memory
    // ring buffer, or one transaction spanning the row + `event_outbox`).
    let model = streaming::create_and_emit(&ctx.db, &org, caller.actor()).await?;
    Metrics::global().organization_created_total.inc();
    // Audit is written by `streaming::create_and_emit` (atomic under outbox).
    format::json(OrgRef::of(&model))
}

/// Fetch an organization by public id.
///
/// `GET /api/organizations/{pid}`. Returns `200` with the stored
/// `Organization` payload, or `404` when the pid is unknown (or
/// soft-deleted, or malformed — all map to not-found via [`http_err`]).
///
/// Runs the **record-level** ABAC pass once the record is loaded, so a
/// policy can decide on this organization rather than on the endpoint.
/// An allow carrying the `mask` obligation returns the redacted view
/// ([`crate::privacy::mask_organization`]) — that is how a deployment
/// grants a partial read without a second endpoint. No-op while
/// `ORGANIZATION_REQUIRE_AUTH` is off.
#[debug_handler]
async fn get_one(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    let org = model.to_org()?;
    let obligations = crate::auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &crate::auth::organization_resource_attrs(&org),
    )
    .map_err(|(status, message)| {
        Error::CustomError(status, ErrorDetail::new("forbidden", &message))
    })?;
    if obligations.iter().any(|o| o == "mask") {
        return format::json(crate::privacy::mask_organization(&org));
    }
    format::json(org)
}

/// The masked view of an organization.
///
/// `GET /api/organizations/{pid}/masked`. Returns `200` with the record
/// redacted per [`crate::privacy::mask_organization`] — telephone,
/// email, street line, and fiscal identifiers — regardless of the
/// caller's policy. `404` for an unknown pid.
///
/// Distinct from the `mask` obligation on `GET /{pid}`: that one is the
/// deployment deciding what a caller may see, while this endpoint is a
/// caller *asking* for the redacted form (a screen share, a support
/// call, an export preview).
#[debug_handler]
async fn get_masked(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    format::json(crate::privacy::mask_organization(&model.to_org()?))
}

/// GDPR right-of-access export for one organization.
///
/// `GET /api/organizations/{pid}/export`. Returns `200` with the
/// envelope from [`crate::privacy::export_organization`]; `404` for an
/// unknown pid.
///
/// **Every export is audited**, including a masked one: a bulk read of
/// personal data is itself a compliance event
/// (`agents/share/bulk-import-export.md` §8), and this is the
/// single-subject case of that machinery. The audit row is written
/// before the response so a failure to record it cannot be traded for a
/// silent disclosure.
///
/// A caller the policy would mask gets a **masked** export, and the
/// envelope says so — an access request answered with redactions must
/// not look like a complete answer.
#[debug_handler]
async fn get_export(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid)
        .await
        .map_err(http_err)?;
    let org = model.to_org()?;
    let obligations = crate::auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &crate::auth::organization_resource_attrs(&org),
    )
    .map_err(|(status, message)| {
        Error::CustomError(status, ErrorDetail::new("forbidden", &message))
    })?;
    let masked = obligations.iter().any(|o| o == "mask");

    AuditModel::record(
        &ctx.db,
        model.pid,
        "exported",
        caller.actor(),
        Some(serde_json::json!({ "masked": masked })),
    )
    .await?;

    format::json(crate::privacy::export_organization(
        &org,
        &model.pid.to_string(),
        masked,
    ))
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
    // Replace + `Updated` event, atomic under the active transport.
    let updated = streaming::update_and_emit(&ctx.db, model, &org, caller.actor()).await?;
    Metrics::global().organization_updated_total.inc();
    // Audit is written by `streaming::update_and_emit` (atomic under outbox).
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
    // Soft-delete + `Deleted` event + audit, atomic under the active
    // transport (audit is written by `streaming::delete_and_emit`).
    streaming::delete_and_emit(&ctx.db, model, caller.actor()).await?;
    Metrics::global().organization_deleted_total.inc();
    format::empty_json()
}

/// List active organizations, one page at a time.
///
/// `GET /api/organizations[?limit=&offset=]`. Returns `200` with an
/// array of `OrgRef`, newest first, soft-deleted rows excluded, plus the
/// `X-Total-Count` / `X-Limit` / `X-Offset` headers
/// (`agents/share/restful.md`). Omitting both parameters returns the
/// first [`LIST_DEFAULT_LIMIT`] rows — exactly what this endpoint
/// returned before it was paginated, so no existing caller changes. An
/// `offset` beyond [`MAX_OFFSET`] is a `400`.
#[debug_handler]
async fn list(
    axum::extract::Query(page): axum::extract::Query<PageParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    page.check_offset()?;
    let (limit, offset) = page.resolve(LIST_DEFAULT_LIMIT);
    let rows = OrgModel::list_paged(&ctx.db, limit, offset).await?;
    let total = OrgModel::count(&ctx.db).await?;
    let refs: Vec<OrgRef> = rows.iter().map(OrgRef::of).collect();
    Ok(with_page_headers(format::json(refs)?, total, limit, offset))
}

// ─── Pagination (agents/share/restful.md) ───────────────────────────────

/// Default page size for `GET /api/organizations` — the cap this
/// endpoint applied before pagination existed, so omitting `limit`
/// returns exactly what it always did.
pub const LIST_DEFAULT_LIMIT: u64 = 100;

/// Largest page any collection read will serve. A bigger `limit` is
/// **clamped** to this rather than refused: a caller asking for 100 000
/// rows wants "as many as you'll give me", and an `X-Limit` of 500
/// answers that better than a `400` it has to learn to handle.
pub const MAX_LIMIT: u64 = 500;

/// Largest accepted `offset`. Past this a request is a `400`: the
/// database would have to materialise and discard arbitrarily many rows,
/// which is a cheap denial of service (SEC-G7). Deep paging past this
/// wants a cursor, not a bigger number.
pub const MAX_OFFSET: u64 = 10_000;

/// `?limit=` / `?offset=` on a collection read.
#[derive(Debug, Default, Deserialize)]
struct PageParams {
    /// Page size; absent, zero or unparseable ⇒ the endpoint default.
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

impl PageParams {
    /// The clamped `(limit, offset)` this request will actually use.
    ///
    /// A zero `limit` falls back to the default rather than serving an
    /// empty page — an empty page and an empty collection look identical
    /// to a client, and only one of them is a real answer.
    fn resolve(&self, default_limit: u64) -> (u64, u64) {
        let limit = self
            .limit
            .filter(|l| *l > 0)
            .unwrap_or(default_limit)
            .min(MAX_LIMIT);
        (limit, self.offset.unwrap_or(0))
    }

    /// Reject an out-of-bound offset before it reaches the database.
    fn check_offset(&self) -> Result<()> {
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

/// Stamp the pagination headers onto a response
/// (`agents/share/restful.md`): the total ignoring the page window, plus
/// the limit and offset actually applied, so a client that sent neither
/// still learns the defaults.
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

/// Maximum number of stored organizations scanned per **batch
/// deduplication** request.
///
/// `deduplicate` scores every unordered pair, so it necessarily loads
/// rows in bulk; the cap bounds the request's memory and its quadratic
/// pair count. Hitting it is logged at `WARN`, because pairs beyond the
/// cap are silently missed. (`check-duplicates`, which scores one query
/// against the store, no longer scans at all — it blocks on the search
/// index instead; see [`CHECK_DUPLICATES_CANDIDATE_LIMIT`].)
pub const CHECK_DUPLICATES_SCAN_CAP: u64 = 1000;

/// Maximum number of **blocked candidates** `check-duplicates` scores a
/// query against.
///
/// The search index supplies these — fuzzy name, exact identifier, and
/// phonetic routes ([`crate::search::SearchEngine::candidates`]) — so
/// this is a bound on how many *plausible* records are scored, not on
/// how much of the table is read. That is the difference that removes
/// the old scale cliff: with a full scan, record 1001 was unreachable no
/// matter how obvious a duplicate it was; with blocking, reachability
/// depends on similarity rather than on insertion order.
pub const CHECK_DUPLICATES_CANDIDATE_LIMIT: usize = 200;

/// Score `query` against `index`'s **blocked** candidates (up to
/// [`CHECK_DUPLICATES_CANDIDATE_LIMIT`]) and return every match, highest
/// score first. `partial_cmp` returns `None` only on NaN scores; those
/// are treated as equal so the sort stays total and never panics.
///
/// Shared by [`check_duplicates`] (the explicit endpoint) and [`create`]
/// (real-time detection, ORG-T3) — each decides separately what an
/// **unavailable index** means for its own call, which is why that
/// check is not folded into this helper: `check_duplicates` refuses
/// (`503`), while `create` degrades to "no duplicates found" (see
/// `create`'s own doc comment for why).
async fn score_against_index(
    ctx: &AppContext,
    index: &crate::search::SearchEngine,
    query: &Organization,
) -> Result<Vec<ScoredRef>> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let hits = index.candidates(query, CHECK_DUPLICATES_CANDIDATE_LIMIT)?;
    let rows = OrgModel::find_by_pids(&ctx.db, &parse_pids(&hits)).await?;
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_org()?;
        let r = engine.match_organizations(query, &candidate);
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
    Ok(hits)
}

/// Find stored organizations that match the query above the threshold.
///
/// `POST /api/organizations/check-duplicates`. Body: an `Organization`.
/// Retrieves up to [`CHECK_DUPLICATES_CANDIDATE_LIMIT`] blocked
/// candidates from the search index, scores each against the query with
/// the matcher, and returns `200` with the matching [`ScoredRef`]s
/// sorted by descending score.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<Organization>,
) -> Result<Response> {
    // Blocking, not scanning. An unavailable index is surfaced rather
    // than silently answering "no duplicates" — that answer would let a
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
    format::json(score_against_index(&ctx, index, &query).await?)
}

// ─── Batch deduplication + stored review queue ──────────────────────────────

/// Review disposition of one queued duplicate pair, in the family's
/// lowercase wire tokens (matching person/worker/place/thing).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReviewStatus {
    /// Awaiting manual review.
    Pending,
    /// Confirmed as a duplicate — ready for merge.
    Confirmed,
    /// Rejected — not a duplicate.
    Rejected,
    /// Auto-merged (no auto-merge path here; present for wire parity).
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
    /// First organization in the candidate pair (public id).
    organization_id_a: String,
    /// Second organization in the candidate pair (public id).
    organization_id_b: String,
    /// Overall match score for the pair, in `[0.0, 1.0]`.
    match_score: f64,
    /// Confidence band label (matcher's `Confidence`, lowercased).
    match_quality: String,
    /// How the pair was detected (`batch_deduplication` here).
    detection_method: String,
    /// Per-component score breakdown, as stored at detection time
    /// (ORG-T1) — the matcher's own `MatchBreakdown`, JSON-encoded.
    /// `None` for a row queued before this field was populated.
    score_breakdown: Option<serde_json::Value>,
    /// Current review state.
    status: ReviewStatus,
    /// How the pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested`; BLK-5).
    provenance: String,
    /// Reviewer identity recorded by the decision endpoint, if decided.
    reviewed_by: Option<String>,
    /// When the pair was first queued.
    created_at: chrono::DateTime<chrono::Utc>,
    /// When the decision was recorded, if decided.
    reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Map a stored review-queue row onto the wire item shape.
fn review_row_to_item(row: &crate::models::review_queue::ReviewQueueRow) -> ReviewQueueItem {
    ReviewQueueItem {
        id: row.id.to_string(),
        organization_id_a: row.record_id_a.to_string(),
        organization_id_b: row.record_id_b.to_string(),
        match_score: row.match_score,
        match_quality: row.match_quality.clone(),
        detection_method: row.detection_method.clone(),
        score_breakdown: row.score_breakdown.clone(),
        status: parse_review_status(&row.status),
        provenance: row.provenance.clone(),
        reviewed_by: row.reviewed_by.clone(),
        created_at: row.created_at,
        reviewed_at: row.reviewed_at,
    }
}

/// Request body for the batch scan. With no `threshold` the matcher's
/// own `is_match` verdict decides; with one, `score >= threshold` does.
#[derive(Debug, Default, Deserialize)]
struct BatchDeduplicationRequest {
    /// Optional score threshold overriding the matcher's verdict.
    #[serde(default)]
    threshold: Option<f64>,
}

/// Response for the batch scan, in the family's report shape.
#[derive(Debug, Serialize)]
struct BatchDeduplicationResponse {
    /// Number of organizations scanned.
    organizations_scanned: usize,
    /// Number of duplicate pairs found (stored rows reported).
    duplicates_found: usize,
    /// Auto-merged count (always 0 — no auto-merge path here).
    auto_merged: usize,
    /// Number of stored pairs currently `pending` review.
    queued_for_review: usize,
    /// The stored candidate pairs (stable ids across re-scans).
    review_items: Vec<ReviewQueueItem>,
}

/// Scan the stored organizations pairwise and queue likely duplicates.
///
/// `POST /api/organizations/deduplicate` (a destructive-classed POST
/// under ABAC, like merge). Loads up to [`CHECK_DUPLICATES_SCAN_CAP`]
/// active rows, scores each unordered pair once (upper triangle), and
/// **persists** hits in the stored `review_queue` (normalized-pair
/// upsert: re-scans refresh scores, decided rows keep their decision,
/// ids stay stable). The response reports the STORED rows. Does not
/// merge anything.
#[allow(clippy::too_many_lines)] // linear scan + persist + report walk
#[debug_handler]
async fn deduplicate(
    State(ctx): State<AppContext>,
    Json(req): Json<BatchDeduplicationRequest>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = OrgModel::list(&ctx.db, CHECK_DUPLICATES_SCAN_CAP).await?;
    if rows.len() as u64 == CHECK_DUPLICATES_SCAN_CAP {
        tracing::warn!(
            cap = CHECK_DUPLICATES_SCAN_CAP,
            "deduplicate scan hit the row cap; pairs beyond the cap are \
             silently missed. Batch dedup is inherently a bulk scan; run \
             it more often, or narrow the corpus."
        );
    }
    let orgs: Vec<(uuid::Uuid, Organization)> = rows
        .iter()
        .map(|row| Ok((row.pid, row.to_org()?)))
        .collect::<Result<_>>()?;
    let mut new_items = Vec::new();
    // Upper-triangular pair iteration: j starts at i+1 so each unordered
    // pair is scored once and no record is compared with itself.
    for i in 0..orgs.len() {
        for j in (i + 1)..orgs.len() {
            let r = engine.match_organizations(&orgs[i].1, &orgs[j].1);
            let is_dup = req.threshold.map_or(r.is_match, |t| r.score >= t);
            if is_dup {
                new_items.push(crate::models::review_queue::NewReviewItem {
                    record_id_a: orgs[i].0,
                    record_id_b: orgs[j].0,
                    match_score: r.score,
                    match_quality: format!("{:?}", r.confidence).to_lowercase(),
                    detection_method: "batch_deduplication".to_string(),
                    // ORG-T1: the column existed and was read back as
                    // `None` unconditionally; store the real breakdown
                    // so a re-fetch of the queue doesn't need a second
                    // `/match` call to explain the score.
                    score_breakdown: serde_json::to_value(&r.breakdown).ok(),
                    status: review_status_token(ReviewStatus::Pending).to_string(),
                    provenance: "operator".to_string(),
                });
            }
        }
    }
    let stored = crate::models::review_queue::upsert(&ctx.db, &new_items).await?;
    let review_items: Vec<ReviewQueueItem> = stored.iter().map(review_row_to_item).collect();
    let queued_for_review = review_items
        .iter()
        .filter(|i| i.status == ReviewStatus::Pending)
        .count();
    format::json(BatchDeduplicationResponse {
        organizations_scanned: orgs.len(),
        duplicates_found: review_items.len(),
        auto_merged: 0,
        queued_for_review,
        review_items,
    })
}

/// Query parameters for the review-queue list endpoint.
#[derive(Debug, Deserialize)]
struct ReviewQueueListQuery {
    /// Optional status-token filter (`pending` / `confirmed` /
    /// `rejected` / `automerged`).
    status: Option<String>,
    /// Maximum items to return (default 100, capped at 500).
    limit: Option<u64>,
}

/// Response for the review-queue list endpoint.
#[derive(Debug, Serialize)]
struct ReviewQueueListResponse {
    /// The stored review-queue items (newest first).
    items: Vec<ReviewQueueItem>,
    /// Number of items returned.
    total: usize,
}

/// List the stored deduplication review queue (newest first).
///
/// `GET /api/organizations/review-queue[?status=&limit=]`. An unknown
/// status token is `422`.
#[debug_handler]
async fn get_review_queue(
    axum::extract::Query(query): axum::extract::Query<ReviewQueueListQuery>,
    State(ctx): State<AppContext>,
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
    let rows = crate::models::review_queue::list(
        &ctx.db,
        query.status.as_deref(),
        query.limit.unwrap_or(100),
    )
    .await?;
    let items: Vec<ReviewQueueItem> = rows.iter().map(review_row_to_item).collect();
    let total = items.len();
    format::json(ReviewQueueListResponse { items, total })
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
/// `POST /api/organizations/review-queue/{id}/decision`. The transition
/// guard is first-writer-wins in SQL: only a `pending` item can be
/// decided; an already-decided item is `422`, an unknown id `404`.
/// The reviewer identity is the verified bearer `sub` when present, and
/// each decision writes a `review_decision` audit row.
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
            format::json(review_row_to_item(&row))
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

/// Query string for the search endpoint: the `q` term, the optional
/// retrieval-mode flags, and the page window.
#[derive(Debug, Deserialize)]
struct SearchParams {
    /// Page size; absent, zero or unparseable ⇒ [`SEARCH_DEFAULT_LIMIT`].
    ///
    /// Declared here rather than by `#[serde(flatten)]`-ing
    /// [`PageParams`]: a flattened struct is deserialized from a
    /// string-keyed map, so `limit=2` arrives as the *string* `"2"` and
    /// fails to parse as a `u64` — a `400` on a valid request.
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
    /// The full-text query.
    q: Option<String>,
    /// Typo-tolerant retrieval (Levenshtein distance ≤ 2) instead of
    /// exact term matching.
    #[serde(default)]
    fuzzy: bool,
    /// Phonetic (Soundex) retrieval — names that *sound* alike.
    #[serde(default)]
    phonetic: bool,
}

/// Default page size for `GET /search` — the cap this endpoint applied
/// before pagination, so omitting `limit` returns what it always did.
pub const SEARCH_DEFAULT_LIMIT: u64 = 50;

/// Full-text search: `GET /api/organizations/search?q=acme[&fuzzy][&phonetic]`.
///
/// Tantivy-backed (spec §13): tokenised full-text over name, legal name,
/// alternate names, identifier values, keywords, address, and URL, with
/// `fuzzy=true` for typo tolerance and `phonetic=true` for Soundex.
/// Returns `200` with up to [`SEARCH_LIMIT`] `OrgRef`s, ranked by
/// relevance. A missing or blank `q` is `400` — an empty search would
/// match everything, which is treated as a malformed request.
///
/// Hits are resolved against Postgres, which is the source of truth, so
/// an index entry for a deleted record cannot surface here.
#[debug_handler]
async fn search(
    axum::extract::Query(params): axum::extract::Query<SearchParams>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let q = params.q.clone().unwrap_or_default();
    let q = q.trim();
    if q.is_empty() {
        return bad_request("query parameter `q` is required");
    }
    let page = PageParams {
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
    let (pids, total) = engine.search_page(
        q,
        mode,
        usize::try_from(limit).unwrap_or(usize::MAX),
        usize::try_from(offset).unwrap_or(usize::MAX),
    )?;
    let rows = OrgModel::find_by_pids(&ctx.db, &parse_pids(&pids)).await?;
    let refs: Vec<OrgRef> = rows.iter().map(OrgRef::of).collect();
    // The total is the index's match count, not the number of rows that
    // resolved: a hit whose row has since been deleted is dropped from
    // the page (see `find_by_pids`), and reporting the shrunken figure
    // would make the count wobble for reasons a caller cannot see.
    Ok(with_page_headers(
        format::json(refs)?,
        total as u64,
        limit,
        offset,
    ))
}

/// Parse index hits into UUIDs, dropping any that will not parse.
///
/// A malformed stored id can only come from a corrupted index, and the
/// right response is to ignore that hit rather than fail a search that
/// has other perfectly good results.
///
/// `pub(crate)` (ORG-T5) so `controllers::fhir::search` can reuse it
/// rather than duplicating the same UUID-parse-and-warn logic.
pub(crate) fn parse_pids(hits: &[String]) -> Vec<uuid::Uuid> {
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

    // Update survivor + soft-delete duplicate + `Merged`/`Deleted` events,
    // all atomic under the active transport (one transaction for `outbox`).
    let (merged, dup_pid, _dup_name) =
        streaming::merge_and_emit(&ctx.db, main, duplicate, &outcome.merged, caller.actor())
            .await?;
    Metrics::global().organization_merged_total.inc();

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
    // Audit (survivor "merged" + duplicate "merged_into") is written by
    // `streaming::merge_and_emit` (atomic under outbox).

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

/// Recent events from the active event transport.
///
/// `GET /api/organizations/events/recent`. Returns `200` with up to 100
/// `EventView`s (`{kind, pid, name, seq}`). Under the `memory` transport
/// these come from the process-wide ring buffer (no DB, not durable);
/// under `outbox` they are the most recent `event_outbox` rows. The wire
/// shape is identical either way.
#[debug_handler]
async fn recent_events(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(streaming::recent_events(&ctx.db, 100).await?)
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
        .add("/deduplicate", post(deduplicate))
        .add("/review-queue", get(get_review_queue))
        .add("/review-queue/{id}/decision", post(review_decision))
        .add("/merge", post(merge))
        .add("/merges/recent", get(recent_merges))
        .add("/whoami", get(whoami))
        .add("/audit/recent", get(recent_audit))
        .add("/events/recent", get(recent_events))
        .add("/{pid}", get(get_one))
        .add("/{pid}", put(update))
        .add("/{pid}", delete(remove))
        .add("/{pid}/masked", get(get_masked))
        .add("/{pid}/export", get(get_export))
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

    /// DB-free pin: the batch-dedup scan cap is the named constant the
    /// handler queries with, so a scan returning exactly this many rows
    /// is the truncation trigger. Pinning the value keeps the constant
    /// the single source of truth (request-level truncation behaviour is
    /// exercised by the Postgres-gated suite).
    #[test]
    fn deduplicate_scan_cap_is_pinned() {
        assert_eq!(CHECK_DUPLICATES_SCAN_CAP, 1000);
    }

    /// The blocked-candidate limit is a *candidate* bound, not a table
    /// scan: it must stay well under the batch cap, or blocking would be
    /// no cheaper than the scan it replaced.
    #[test]
    fn check_duplicates_candidate_limit_is_pinned() {
        assert_eq!(CHECK_DUPLICATES_CANDIDATE_LIMIT, 200);
        assert!(
            (CHECK_DUPLICATES_CANDIDATE_LIMIT as u64) < CHECK_DUPLICATES_SCAN_CAP,
            "blocking must consider fewer records than a full scan"
        );
    }

    /// The default page sizes are the pre-pagination caps, so omitting
    /// `limit` returns exactly what these endpoints always returned.
    #[test]
    fn page_defaults_match_the_old_caps() {
        assert_eq!(SEARCH_DEFAULT_LIMIT, 50);
        assert_eq!(LIST_DEFAULT_LIMIT, 100);
    }

    /// `limit` is clamped rather than refused, a zero or absent `limit`
    /// falls back to the default, and the offset bound is what rejects.
    #[test]
    fn page_params_clamp_rather_than_reject() {
        let huge = PageParams {
            limit: Some(100_000),
            offset: Some(0),
        };
        assert_eq!(huge.resolve(LIST_DEFAULT_LIMIT), (MAX_LIMIT, 0));
        let zero = PageParams {
            limit: Some(0),
            offset: None,
        };
        assert_eq!(zero.resolve(LIST_DEFAULT_LIMIT), (LIST_DEFAULT_LIMIT, 0));
        let absent = PageParams::default();
        assert_eq!(
            absent.resolve(SEARCH_DEFAULT_LIMIT),
            (SEARCH_DEFAULT_LIMIT, 0)
        );
        assert!(
            PageParams {
                limit: None,
                offset: Some(MAX_OFFSET)
            }
            .check_offset()
            .is_ok()
        );
        assert!(
            PageParams {
                limit: None,
                offset: Some(MAX_OFFSET + 1)
            }
            .check_offset()
            .is_err(),
            "an unbounded offset is a DoS, not a deep page"
        );
    }

    /// A corrupt (non-UUID) index hit is dropped rather than failing the
    /// whole search.
    #[test]
    fn parse_pids_drops_unparseable_hits() {
        let good = uuid::Uuid::new_v4();
        let parsed = parse_pids(&[good.to_string(), "not-a-uuid".to_string()]);
        assert_eq!(parsed, vec![good]);
    }

    /// Review statuses serialize as the family's lowercase wire tokens,
    /// and the stored-token parse round-trips them.
    #[test]
    fn review_status_wire_tokens_round_trip() {
        for status in [
            ReviewStatus::Pending,
            ReviewStatus::Confirmed,
            ReviewStatus::Rejected,
            ReviewStatus::AutoMerged,
        ] {
            let token = review_status_token(status);
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                serde_json::json!(token)
            );
            assert_eq!(parse_review_status(token), status);
        }
        // Unknown stored tokens read as pending, never panic.
        assert_eq!(parse_review_status("garbage"), ReviewStatus::Pending);
    }

    /// Only the two operator verdicts parse as decisions — `pending` /
    /// `automerged` are refused at the serde boundary.
    #[test]
    fn decision_wire_tokens() {
        let ok: ReviewDecisionRequest =
            serde_json::from_value(serde_json::json!({"status": "confirmed"})).unwrap();
        assert_eq!(ok.status, ReviewDecision::Confirmed);
        let ok: ReviewDecisionRequest =
            serde_json::from_value(serde_json::json!({"status": "rejected"})).unwrap();
        assert_eq!(ok.status, ReviewDecision::Rejected);
        assert!(
            serde_json::from_value::<ReviewDecisionRequest>(
                serde_json::json!({"status": "pending"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<ReviewDecisionRequest>(
                serde_json::json!({"status": "automerged"})
            )
            .is_err()
        );
    }
}
