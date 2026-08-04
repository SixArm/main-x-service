//! REST handlers for the Thing Service.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::state::AppState;
use crate::api::ApiResponse;
use crate::db::audit::AuditContext;
use crate::matching::confidence_label;
use crate::models::merge::{MergeRecord, MergeRequest, MergeResponse};
use crate::models::thing::Thing;
use crate::privacy::{gdpr_export, mask_thing};
use crate::streaming::{EventKind, ThingEvent};
use crate::validation::{normalize_thing, validate_thing};

/// Map a crate error to the HTTP status the REST layer returns for it.
///
/// The three caller-correctable errors get specific 4xx codes; everything
/// else (DB, search, pool, streaming, config) is an opaque 500.
fn status_for(err: &crate::Error) -> StatusCode {
    match err {
        crate::Error::NotFound => StatusCode::NOT_FOUND, // 404
        crate::Error::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY, // 422
        crate::Error::Conflict(_) => StatusCode::CONFLICT, // 409
        _ => StatusCode::INTERNAL_SERVER_ERROR,          // 500
    }
}

/// Build a failure response `(status, Json(envelope))` from a crate error.
fn fail(err: &crate::Error) -> (StatusCode, Json<ApiResponse<Thing>>) {
    (
        status_for(err),
        Json(ApiResponse::error("error", err.to_string())),
    )
}

/// Health-check payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Always `"healthy"` when the process is serving.
    pub status: String,
    /// Service name.
    pub service: String,
    /// Crate version.
    pub version: String,
}

/// Liveness probe.
#[utoipa::path(get, path = "/api/health", tag = "health",
    responses((status = 200, description = "Service healthy", body = HealthResponse)))]
pub async fn health() -> impl IntoResponse {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy".into(),
        service: "thing-service".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}

/// Prometheus metrics endpoint (text-exposition format).
///
/// Renders [`crate::metrics::METRICS`] for scraping. Mounted at the
/// root (`/metrics.prom`) — not under `/api` — so a default Prometheus
/// scrape config (`metrics_path: /metrics.prom`) finds it.
#[utoipa::path(
    get,
    path = "/metrics.prom",
    tag = "observability",
    responses(
        (status = 200, description = "Prometheus text-exposition format", content_type = "text/plain; version=0.0.4; charset=utf-8")
    )
)]
pub async fn metrics_prom() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            crate::metrics::CONTENT_TYPE,
        )],
        crate::metrics::METRICS.render(),
    )
}

/// Create a thing (with real-time duplicate detection).
#[utoipa::path(post, path = "/api/things", tag = "things",
    request_body = Thing,
    responses(
        (status = 201, description = "Created", body = Thing),
        (status = 409, description = "Duplicate detected", body = crate::api::ApiError),
        (status = 422, description = "Validation error", body = crate::api::ApiError),
    ))]
pub async fn create_thing(
    State(state): State<AppState>,
    Json(mut thing): Json<Thing>,
) -> impl IntoResponse {
    // Normalize first (scheme-lowercase URLs, dedupe lists) so validation and
    // duplicate detection operate on the canonical form the record is stored in.
    normalize_thing(&mut thing);
    let errors = validate_thing(&thing);
    if !errors.is_empty() {
        // 422: well-formed JSON, but the record violates data-quality rules.
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error_with_details(
                "validation_error",
                "thing failed validation",
                errors,
            )),
        );
    }

    // `id` is server-managed (see `Thing::id`'s docs): a client that omits
    // it — now that the field is `#[serde(default)]` — arrives here as
    // the nil UUID. Mint a fresh one, the same pattern the event service
    // uses, so a hand-written create body never has to invent an id.
    if thing.id == Uuid::nil() {
        thing.id = Uuid::new_v4();
    }

    // Real-time duplicate detection: score against existing records and reject
    // if any candidate meets the configured match threshold.
    let candidates = find_candidates(&state, &thing).await;
    let dups: Vec<ScoredCandidate> = candidates
        .into_iter()
        .filter(|c| c.score >= state.matcher.threshold())
        .collect();
    if !dups.is_empty() {
        // 409: caller should resolve/merge rather than create a second record.
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error_with_details(
                "duplicate_detected",
                "potential duplicates found",
                dups,
            )),
        );
    }

    match state.thing_repository.create(&thing).await {
        Ok(stored) => {
            // Post-write side effects are fire-and-forget: a failed index
            // update, event publish, or audit write must not fail the create
            // (the record is already durably persisted), so errors are dropped
            // with `let _ =`.
            let _ = state.search_engine.index_thing(&stored);
            let _ = state
                .event_publisher
                .publish(ThingEvent::new(
                    EventKind::ThingCreated,
                    stored.id,
                    serde_json::json!({ "name": stored.name }),
                ))
                .await;
            // Audit the post-image; only the new values exist on a create.
            if let Ok(v) = serde_json::to_value(&stored) {
                let _ = state
                    .audit_log
                    .log_create("thing", stored.id, v, &AuditContext::default())
                    .await;
            }
            // 201 Created with the stored (hydrated) record.
            (StatusCode::CREATED, Json(ApiResponse::success(stored)))
        }
        Err(e) => (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        ),
    }
}

/// Get a thing by id.
#[utoipa::path(get, path = "/api/things/{id}", tag = "things",
    params(("id" = Uuid, Path, description = "Thing id")),
    responses((status = 200, body = Thing), (status = 404, description = "Not found")))]
pub async fn get_thing(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match state.thing_repository.get_by_id(&id).await {
        Ok(Some(t)) => (StatusCode::OK, Json(ApiResponse::success(t))),
        Ok(None) => fail(&crate::Error::NotFound),
        Err(e) => fail(&e),
    }
}

/// Update a thing.
#[utoipa::path(put, path = "/api/things/{id}", tag = "things",
    params(("id" = Uuid, Path, description = "Thing id")),
    request_body = Thing,
    responses((status = 200, body = Thing), (status = 404, description = "Not found"),
        (status = 422, description = "Validation error", body = crate::api::ApiError)))]
pub async fn update_thing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut thing): Json<Thing>,
) -> impl IntoResponse {
    thing.id = id;
    normalize_thing(&mut thing);
    let errors = validate_thing(&thing);
    if !errors.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error_with_details(
                "validation_error",
                "thing failed validation",
                errors,
            )),
        );
    }
    let old = state.thing_repository.get_by_id(&id).await.ok().flatten();
    match state.thing_repository.update(&thing).await {
        Ok(stored) => {
            let _ = state.search_engine.delete_thing(&id.to_string());
            let _ = state.search_engine.index_thing(&stored);
            let _ = state
                .event_publisher
                .publish(ThingEvent::new(
                    EventKind::ThingUpdated,
                    stored.id,
                    serde_json::json!({ "name": stored.name }),
                ))
                .await;
            if let (Some(old), Ok(new_v)) = (old, serde_json::to_value(&stored))
                && let Ok(old_v) = serde_json::to_value(&old)
            {
                let _ = state
                    .audit_log
                    .log_update("thing", stored.id, old_v, new_v, &AuditContext::default())
                    .await;
            }
            (StatusCode::OK, Json(ApiResponse::success(stored)))
        }
        Err(e) => fail(&e),
    }
}

/// Soft-delete a thing.
#[utoipa::path(delete, path = "/api/things/{id}", tag = "things",
    params(("id" = Uuid, Path, description = "Thing id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_thing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let old = state.thing_repository.get_by_id(&id).await.ok().flatten();
    match state.thing_repository.soft_delete(&id).await {
        Ok(()) => {
            let _ = state.search_engine.delete_thing(&id.to_string());
            let _ = state
                .event_publisher
                .publish(ThingEvent::new(
                    EventKind::ThingDeleted,
                    id,
                    serde_json::json!({}),
                ))
                .await;
            if let Some(old) = old
                && let Ok(v) = serde_json::to_value(&old)
            {
                let _ = state
                    .audit_log
                    .log_delete("thing", id, v, &AuditContext::default())
                    .await;
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => fail(&e).into_response(),
    }
}

/// Search query parameters.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct SearchQuery {
    /// Free-text query.
    pub q: Option<String>,
    /// Max results (default 10, capped at 100).
    pub limit: Option<usize>,
    /// Use fuzzy matching.
    pub fuzzy: Option<bool>,
    /// Mask sensitive fields in the results.
    pub mask_sensitive: Option<bool>,
}

/// Search response payload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchResponse {
    /// The matched things (hydrated from the DB).
    pub results: Vec<Thing>,
    /// Number of results returned.
    pub total: usize,
}

/// Full-text / fuzzy thing search.
#[utoipa::path(get, path = "/api/things/search", tag = "search",
    params(SearchQuery),
    responses((status = 200, body = SearchResponse)))]
pub async fn search_things(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    // Clamp the caller-supplied limit (default 10) to a hard ceiling of 100
    // so a single request can never ask the index for an unbounded page.
    let limit = q.limit.unwrap_or(10).min(100);
    let query = q.q.unwrap_or_default();
    // Tantivy returns id strings; a failed query degrades to an empty page
    // rather than an error so search stays best-effort.
    let ids = if q.fuzzy.unwrap_or(false) {
        state.search_engine.fuzzy_search(&query, limit)
    } else {
        state.search_engine.search(&query, limit)
    }
    .unwrap_or_default();

    // Hydrate each hit from the DB (the index holds only ids), skipping any
    // that no longer resolve (e.g. soft-deleted between index and read).
    let mut results = Vec::new();
    for id in ids {
        if let Ok(uuid) = Uuid::parse_str(&id)
            && let Ok(Some(t)) = state.thing_repository.get_by_id(&uuid).await
        {
            results.push(if q.mask_sensitive.unwrap_or(false) {
                mask_thing(&t)
            } else {
                t
            });
        }
    }
    let total = results.len();
    (
        StatusCode::OK,
        Json(ApiResponse::success(SearchResponse { results, total })),
    )
}

/// A scored candidate thing returned by match / duplicate endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScoredCandidate {
    /// The candidate thing.
    pub thing: Thing,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Confidence band (`certain`/`probable`/`possible`/`unlikely`).
    pub confidence: String,
}

/// Score a request thing against existing records, sorted descending.
async fn find_candidates(state: &AppState, thing: &Thing) -> Vec<ScoredCandidate> {
    // Blocking step: use the name index to fetch up to 50 likely candidates
    // rather than scoring the whole table. This bounds the O(candidates) scoring
    // loop below.
    let ids = state
        .search_engine
        .search_by_name(&thing.name, 50)
        .unwrap_or_default();
    let mut out = Vec::new();
    for id in ids {
        let Ok(uuid) = Uuid::parse_str(&id) else {
            continue;
        };
        // Never score a record against itself (matters on update/dedup paths).
        if uuid == thing.id {
            continue;
        }
        if let Ok(Some(existing)) = state.thing_repository.get_by_id(&uuid).await {
            let r = state.matcher.score(thing, &existing);
            out.push(ScoredCandidate {
                thing: existing,
                score: r.score,
                confidence: confidence_label(&r.confidence).to_string(),
            });
        }
    }
    // Highest score first; treat NaN/incomparable as equal so the sort is total.
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Match a candidate thing against existing records.
#[utoipa::path(post, path = "/api/things/match", tag = "matching",
    request_body = Thing,
    responses((status = 200, body = [ScoredCandidate])))]
pub async fn match_thing(
    State(state): State<AppState>,
    Json(thing): Json<Thing>,
) -> impl IntoResponse {
    let candidates = find_candidates(&state, &thing).await;
    (StatusCode::OK, Json(ApiResponse::success(candidates)))
}

/// Duplicate-check response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DuplicateCheckResponse {
    /// Whether any candidate scored at/above the match threshold.
    pub duplicates_found: bool,
    /// The scored candidates.
    pub candidates: Vec<ScoredCandidate>,
}

/// Check for duplicates without creating a record.
#[utoipa::path(post, path = "/api/things/check-duplicates", tag = "matching",
    request_body = Thing,
    responses((status = 200, body = DuplicateCheckResponse)))]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(thing): Json<Thing>,
) -> impl IntoResponse {
    let candidates = find_candidates(&state, &thing).await;
    let threshold = state.matcher.threshold();
    let duplicates_found = candidates.iter().any(|c| c.score >= threshold);
    (
        StatusCode::OK,
        Json(ApiResponse::success(DuplicateCheckResponse {
            duplicates_found,
            candidates,
        })),
    )
}

/// Merge a duplicate thing into a surviving main thing.
#[utoipa::path(post, path = "/api/things/merge", tag = "matching",
    request_body = MergeRequest,
    responses((status = 200, body = MergeResponse), (status = 404, description = "Not found")))]
pub async fn merge_things(
    State(state): State<AppState>,
    Json(req): Json<MergeRequest>,
) -> impl IntoResponse {
    let main = match state.thing_repository.get_by_id(&req.main_thing_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("not_found", "main thing not found")),
            );
        }
        Err(e) => {
            return (
                status_for(&e),
                Json(ApiResponse::error("error", e.to_string())),
            );
        }
    };
    let dup = match state
        .thing_repository
        .get_by_id(&req.duplicate_thing_id)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("not_found", "duplicate thing not found")),
            );
        }
        Err(e) => {
            return (
                status_for(&e),
                Json(ApiResponse::error("error", e.to_string())),
            );
        }
    };

    let transferred = serde_json::to_value(&dup).ok();
    // Atomic merge: the survivor's row + the duplicate's soft-delete (and,
    // under the outbox transport, the `Merged`+`Deleted` outbox rows)
    // commit in one transaction. Merge-history, search sync, and the
    // in-memory event stay here in the handler.
    let main = match state.thing_repository.merge(&main, &dup.id).await {
        Ok(t) => t,
        Err(e) => {
            return (
                status_for(&e),
                Json(ApiResponse::error("error", e.to_string())),
            );
        }
    };
    let _ = state.search_engine.delete_thing(&dup.id.to_string());

    let record = MergeRecord {
        id: Uuid::new_v4(),
        main_thing_id: main.id,
        duplicate_thing_id: dup.id,
        merge_reason: req.merge_reason.clone(),
        transferred_data: transferred,
        merged_at: chrono::Utc::now(),
    };
    let record = match state.thing_repository.record_merge(&record).await {
        Ok(r) => r,
        Err(e) => {
            return (
                status_for(&e),
                Json(ApiResponse::error("error", e.to_string())),
            );
        }
    };
    let _ = state
        .event_publisher
        .publish(ThingEvent::new(
            EventKind::ThingMerged,
            main.id,
            serde_json::json!({ "duplicate": dup.id }),
        ))
        .await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(MergeResponse {
            merge_record: record,
            main_thing: main,
        })),
    )
}

/// Batch-deduplication request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchDeduplicationRequest {
    /// Match threshold (default 0.7).
    #[serde(default)]
    pub threshold: Option<f64>,
    /// Max candidates to scan (default 100).
    #[serde(default)]
    pub max_candidates: Option<u64>,
}

/// Review disposition of one queued duplicate pair. Serialized with the
/// family's lowercase wire tokens (`pending`, `confirmed`, `rejected`,
/// `automerged`), matching the person/worker services.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Awaiting manual review.
    Pending,
    /// Confirmed as a duplicate — ready for merge.
    Confirmed,
    /// Rejected — not a duplicate.
    Rejected,
    /// Auto-merged (score above an auto-merge threshold).
    AutoMerged,
}

/// One candidate duplicate pair emitted by the batch scan, mirroring the
/// person/worker review-item shape (`detection_method` included) so the
/// family front-ends can render one review queue. Items persist in the
/// stored `review_queue` and are decided via the decision endpoint.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReviewQueueItem {
    /// Server-generated review-item id.
    pub id: Uuid,
    /// First thing in the candidate pair.
    pub thing_id_a: Uuid,
    /// Second thing in the candidate pair.
    pub thing_id_b: Uuid,
    /// Overall match score for the pair, in `[0.0, 1.0]`.
    pub match_score: f64,
    /// Confidence band label (lowercased [`MatchConfidence`] variant).
    pub match_quality: String,
    /// How the pair was detected (always `batch_deduplication` here).
    pub detection_method: String,
    /// Current review state.
    pub status: ReviewStatus,
    /// Reviewer identity recorded by the decision endpoint, if decided.
    pub reviewed_by: Option<String>,
    /// When the item was produced.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the decision was recorded, if decided.
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Batch-deduplication response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    /// Number of things scanned.
    pub things_scanned: usize,
    /// Number of duplicate pairs found.
    pub duplicates_found: usize,
    /// Number of pairs auto-merged. Always `0`: this service has no
    /// auto-merge path; the field exists so the report shape matches
    /// the person/worker services.
    pub auto_merged: usize,
    /// Number of pairs queued for human review (all found pairs, until
    /// a review-decision endpoint exists).
    pub queued_for_review: usize,
    /// The queued candidate pairs.
    pub review_items: Vec<ReviewQueueItem>,
}

/// Batch deduplication scan over all active things.
#[utoipa::path(post, path = "/api/things/deduplicate", tag = "matching",
    request_body = BatchDeduplicationRequest,
    responses((status = 200, body = BatchDeduplicationResponse)))]
pub async fn deduplicate(
    State(state): State<AppState>,
    Json(req): Json<BatchDeduplicationRequest>,
) -> impl IntoResponse {
    // Scan at most `max_candidates` active records (default 100), then compare
    // every unordered pair. The cost is O(n^2) in the scanned set, so the cap
    // bounds the work; the threshold defaults to the matcher's configured one.
    let limit = req.max_candidates.unwrap_or(100);
    let threshold = req.threshold.unwrap_or_else(|| state.matcher.threshold());
    let things = state
        .thing_repository
        .list(limit, 0)
        .await
        .unwrap_or_default();
    let mut review_items = Vec::new();
    // Upper-triangular pair iteration: j starts at i+1 so each pair is
    // scored once and no record is compared with itself. Every pair at
    // or above the threshold becomes a pending review item (the
    // person/worker report shape).
    for i in 0..things.len() {
        for j in (i + 1)..things.len() {
            let result = state.matcher.score(&things[i], &things[j]);
            if result.score >= threshold {
                review_items.push(ReviewQueueItem {
                    id: Uuid::new_v4(),
                    thing_id_a: things[i].id,
                    thing_id_b: things[j].id,
                    match_score: result.score,
                    match_quality: confidence_label(&result.confidence).to_string(),
                    detection_method: "batch_deduplication".to_string(),
                    status: ReviewStatus::Pending,
                    reviewed_by: None,
                    created_at: chrono::Utc::now(),
                    reviewed_at: None,
                });
            }
        }
    }
    // Persist the scan into the stored review queue: normalized-pair
    // upsert refreshes scores on re-scan while a decided row keeps its
    // decision (`status` is never touched on conflict). The response
    // reports the STORED rows, so item ids are stable across scans and
    // prior decisions show through.
    let new_items: Vec<crate::db::review_queue::NewReviewItem> = review_items
        .iter()
        .map(|r| crate::db::review_queue::NewReviewItem {
            record_id_a: r.thing_id_a,
            record_id_b: r.thing_id_b,
            match_score: r.match_score,
            match_quality: r.match_quality.clone(),
            detection_method: r.detection_method.clone(),
            score_breakdown: None,
            status: review_status_token(&r.status).to_string(),
        })
        .collect();
    let rows = match crate::db::review_queue::upsert(&state.db, &new_items).await {
        Ok(rows) => rows,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<BatchDeduplicationResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to persist review queue: {e}"),
                )),
            );
        }
    };
    let review_items: Vec<ReviewQueueItem> = rows.iter().map(review_row_to_item).collect();
    let duplicates_found = review_items.len();
    let queued_for_review = review_items
        .iter()
        .filter(|r| r.status == ReviewStatus::Pending)
        .count();
    let auto_merged = review_items
        .iter()
        .filter(|r| r.status == ReviewStatus::AutoMerged)
        .count();
    (
        StatusCode::OK,
        Json(ApiResponse::success(BatchDeduplicationResponse {
            things_scanned: things.len(),
            duplicates_found,
            auto_merged,
            queued_for_review,
            review_items,
        })),
    )
}

/// One operator verdict for a `pending` review item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewDecision {
    /// Confirm the pair as a duplicate (ready for merge).
    Confirmed,
    /// Reject the pair (not a duplicate).
    Rejected,
}

/// Request body for `POST /api/things/review-queue/{id}/decision`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewDecisionRequest {
    /// The verdict (`confirmed` or `rejected`).
    pub status: ReviewDecision,
}

/// Query parameters for the review-queue list endpoint.
#[derive(Debug, Deserialize)]
pub struct ReviewQueueListQuery {
    /// Optional status-token filter (`pending` / `confirmed` /
    /// `rejected` / `automerged`).
    pub status: Option<String>,
    /// Maximum items to return (default 100, capped at 500).
    pub limit: Option<u64>,
}

/// Response envelope for `GET /api/things/review-queue`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReviewQueueListResponse {
    /// The stored review-queue items (newest first).
    pub items: Vec<ReviewQueueItem>,
    /// Number of items returned.
    pub total: usize,
}

// ─── Review queue (stored) ──────────────────────────────────────────────────

/// The lowercase wire token for a review status.
fn review_status_token(status: &ReviewStatus) -> &'static str {
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

/// Map a stored review-queue row onto the wire item shape.
fn review_row_to_item(row: &crate::db::review_queue::ReviewQueueRow) -> ReviewQueueItem {
    ReviewQueueItem {
        id: row.id,
        thing_id_a: row.record_id_a,
        thing_id_b: row.record_id_b,
        match_score: row.match_score,
        match_quality: row.match_quality.clone(),
        detection_method: row.detection_method.clone(),
        status: parse_review_status(&row.status),
        reviewed_by: row.reviewed_by.clone(),
        created_at: crate::db::convert::offset_to_ts(row.created_at),
        reviewed_at: row.reviewed_at.map(crate::db::convert::offset_to_ts),
    }
}

/// List the stored deduplication review queue (newest first).
#[utoipa::path(get, path = "/api/things/review-queue", tag = "matching",
    responses(
        (status = 200, description = "Stored review-queue items", body = ReviewQueueListResponse),
        (status = 422, description = "Unknown status token"),
        (status = 500, description = "Internal server error")))]
pub async fn get_review_queue(
    State(state): State<AppState>,
    Query(query): Query<ReviewQueueListQuery>,
) -> axum::response::Response {
    if let Some(status) = query.status.as_deref()
        && !matches!(status, "pending" | "confirmed" | "rejected" | "automerged")
    {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::<ReviewQueueListResponse>::error(
                "INVALID_STATUS",
                format!("unknown review status `{status}`"),
            )),
        )
            .into_response();
    }
    match crate::db::review_queue::list(
        &state.db,
        query.status.as_deref(),
        query.limit.unwrap_or(100),
    )
    .await
    {
        Ok(rows) => {
            let items: Vec<ReviewQueueItem> = rows.iter().map(review_row_to_item).collect();
            let total = items.len();
            (
                StatusCode::OK,
                Json(ApiResponse::success(ReviewQueueListResponse {
                    items,
                    total,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<ReviewQueueListResponse>::error(
                "DATABASE_ERROR",
                format!("Failed to list review queue: {e}"),
            )),
        )
            .into_response(),
    }
}

/// Decide one `pending` review item (`confirmed` or `rejected`).
///
/// The transition guard is first-writer-wins in SQL: only a `pending`
/// item can be decided; an already-decided item returns `422`, an
/// unknown id `404`.
#[utoipa::path(post, path = "/api/things/review-queue/{id}/decision", tag = "matching",
    request_body = ReviewDecisionRequest,
    responses(
        (status = 200, description = "The decided item", body = ReviewQueueItem),
        (status = 404, description = "No review item with that id"),
        (status = 422, description = "Item already decided"),
        (status = 500, description = "Internal server error")))]
pub async fn review_decision(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReviewDecisionRequest>,
) -> axum::response::Response {
    let token = match req.status {
        ReviewDecision::Confirmed => "confirmed",
        ReviewDecision::Rejected => "rejected",
    };
    // No optional-claims extractor exists in this service yet (accepted
    // drift from person/worker), so the reviewer identity is not
    // recorded here.
    match crate::db::review_queue::decide(&state.db, id, token, None).await {
        Ok(crate::db::review_queue::DecideOutcome::Decided(row)) => (
            StatusCode::OK,
            Json(ApiResponse::success(review_row_to_item(&row))),
        )
            .into_response(),
        Ok(crate::db::review_queue::DecideOutcome::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ReviewQueueItem>::error(
                "NOT_FOUND",
                format!("no review item {id}"),
            )),
        )
            .into_response(),
        Ok(crate::db::review_queue::DecideOutcome::AlreadyDecided(current)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::<ReviewQueueItem>::error(
                "INVALID_REVIEW_TRANSITION",
                format!("item is `{current}`; only `pending` items can be decided"),
            )),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<ReviewQueueItem>::error(
                "DATABASE_ERROR",
                format!("Failed to decide review item: {e}"),
            )),
        )
            .into_response(),
    }
}

/// GDPR data export for one thing.
#[utoipa::path(get, path = "/api/things/{id}/export", tag = "privacy",
    params(("id" = Uuid, Path, description = "Thing id")),
    responses((status = 200, description = "Full thing export"), (status = 404, description = "Not found")))]
pub async fn export_thing_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.thing_repository.get_by_id(&id).await {
        Ok(Some(t)) => (StatusCode::OK, Json(ApiResponse::success(gdpr_export(&t)))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("not_found", "thing not found")),
        ),
        Err(e) => (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        ),
    }
}

/// Masked thing view.
#[utoipa::path(get, path = "/api/things/{id}/masked", tag = "privacy",
    params(("id" = Uuid, Path, description = "Thing id")),
    responses((status = 200, body = Thing), (status = 404, description = "Not found")))]
pub async fn masked_thing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.thing_repository.get_by_id(&id).await {
        Ok(Some(t)) => (StatusCode::OK, Json(ApiResponse::success(mask_thing(&t)))),
        Ok(None) => fail(&crate::Error::NotFound),
        Err(e) => fail(&e),
    }
}

/// Audit-query parameters.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct AuditQuery {
    /// Max rows (default 50, capped at 500).
    pub limit: Option<u64>,
}

/// Audit log for one thing.
#[utoipa::path(get, path = "/api/things/{id}/audit", tag = "audit",
    params(("id" = Uuid, Path, description = "Thing id"), AuditQuery),
    responses((status = 200, body = [crate::db::audit::AuditEntry])))]
pub async fn audit_for_thing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50).min(500);
    match state.audit_log.list_for_entity(id, limit).await {
        Ok(entries) => (StatusCode::OK, Json(ApiResponse::success(entries))),
        Err(e) => (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        ),
    }
}

/// Recent system-wide audit activity.
#[utoipa::path(get, path = "/api/audit/recent", tag = "audit",
    params(AuditQuery),
    responses((status = 200, body = [crate::db::audit::AuditEntry])))]
pub async fn audit_recent(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50).min(500);
    match state.audit_log.list_recent(limit).await {
        Ok(entries) => (StatusCode::OK, Json(ApiResponse::success(entries))),
        Err(e) => (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        ),
    }
}

/// Default rows examined by the integrity endpoints.
pub const VERIFY_DEFAULT_LIMIT: u64 = 200;

/// Hard cap on rows examined in one call.
///
/// Record verification assembles each row through the repository — one
/// query per row — so an unbounded limit is a denial-of-service on a
/// large table (the SEC-M1 bound-every-input invariant). The cap is
/// lower than the sibling loco services' because their rows are a single
/// JSONB read and these are not.
pub const VERIFY_MAX_LIMIT: u64 = 1000;

/// Verify row-level record integrity.
///
/// `GET /api/records/verify?limit=200` — reassembles each record and
/// recomputes its three digests, naming any row that differs.
pub async fn verify_record_integrity(
    State(state): State<AppState>,
    Query(params): Query<AuditQuery>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

    let limit = params
        .limit
        .unwrap_or(VERIFY_DEFAULT_LIMIT)
        .clamp(1, VERIFY_MAX_LIMIT);
    let rows = match crate::db::models::things::Entity::find()
        .order_by_desc(crate::db::models::things::Column::UpdatedAt)
        .limit(limit)
        .all(&state.db)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DATABASE_ERROR",
                    format!("Failed to read records for verification: {e}"),
                )),
            )
                .into_response();
        }
    };

    // Assemble each record so the digest covers the child tables too —
    // which is the whole point, since an identifier edit lives there.
    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        match state.thing_repository.get_by_id(&row.id).await {
            Ok(Some(thing)) => records.push(crate::compliance::record_integrity::StoredRecord {
                thing,
                sha256: row.content_hash,
                sha3: row.content_hash_sha3,
                mac: row.content_mac,
                is_deleted: row.is_deleted,
            }),
            // A row that vanished between the two queries, or is
            // soft-deleted (the getter hides those), is skipped rather
            // than reported: neither is a finding.
            Ok(None) => {}
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<serde_json::Value>::error(
                        "DATABASE_ERROR",
                        format!("Failed to assemble a record for verification: {e}"),
                    )),
                )
                    .into_response();
            }
        }
    }
    (
        StatusCode::OK,
        Json(crate::compliance::record_integrity::verify(&records)),
    )
        .into_response()
}

/// Verify audit-row integrity.
///
/// `GET /api/audit/verify?limit=200` — recomputes each audit row's
/// SHA-256, SHA-3, and MAC, naming any row whose content was altered.
///
/// The unkeyed digests are checked too, not just the MAC: they are
/// written even when no key is configured, so on a default deployment
/// they are the only integrity these rows have.
pub async fn verify_audit_integrity(
    State(state): State<AppState>,
    Query(params): Query<AuditQuery>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

    let limit = params
        .limit
        .unwrap_or(VERIFY_DEFAULT_LIMIT)
        .clamp(1, VERIFY_MAX_LIMIT);
    match crate::db::models::audit_log::Entity::find()
        .order_by_desc(crate::db::models::audit_log::Column::Id)
        .limit(limit)
        .all(&state.db)
        .await
    {
        Ok(rows) => (
            StatusCode::OK,
            Json(crate::compliance::audit_integrity::verify(&rows)),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to read audit rows for verification: {e}"),
            )),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod review_report_tests {
    use super::*;

    /// The review-status wire tokens are the family's lowercase form
    /// (matching person/worker), and a report serializes the full
    /// person-shaped item including `detection_method`.
    #[test]
    fn review_status_wire_tokens_are_lowercase() {
        let value = serde_json::to_value(ReviewStatus::Pending).unwrap();
        assert_eq!(value, serde_json::json!("pending"));
        let value = serde_json::to_value(ReviewStatus::AutoMerged).unwrap();
        assert_eq!(value, serde_json::json!("automerged"));
        let item = ReviewQueueItem {
            id: Uuid::new_v4(),
            thing_id_a: Uuid::new_v4(),
            thing_id_b: Uuid::new_v4(),
            match_score: 0.91,
            match_quality: "probable".to_string(),
            detection_method: "batch_deduplication".to_string(),
            status: ReviewStatus::Pending,
            reviewed_by: None,
            created_at: chrono::Utc::now(),
            reviewed_at: None,
        };
        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(value["detection_method"], "batch_deduplication");
        assert_eq!(value["status"], "pending");
    }
    /// Only the two operator verdicts parse as decisions.
    #[test]
    fn decision_wire_tokens() {
        let ok: ReviewDecisionRequest =
            serde_json::from_value(serde_json::json!({"status": "confirmed"})).unwrap();
        assert_eq!(ok.status, ReviewDecision::Confirmed);
        assert!(
            serde_json::from_value::<ReviewDecisionRequest>(
                serde_json::json!({"status": "pending"})
            )
            .is_err()
        );
    }
}
