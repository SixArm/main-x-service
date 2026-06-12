//! REST handlers for events.
//!
//! One async function per route (CRUD, search, match, dedup, merge,
//! privacy export/mask, audit) plus their request/response DTOs and a
//! few private blocking/conversion helpers. Each public handler carries
//! a `#[utoipa::path]` attribute that feeds
//! [`ApiDoc`](crate::api::rest::ApiDoc). Handlers take
//! [`AppState`](crate::api::rest::AppState) via Axum's `State` extractor
//! and return `impl IntoResponse`; errors are returned as
//! [`ApiResponse`](crate::api::ApiResponse) error envelopes with an
//! appropriate [`StatusCode`](axum::http::StatusCode).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::ApiResponse;
use crate::models::{Event, EventLink, EventStatus, EventType, LinkType};

use super::state::AppState;

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/// Body returned by the health-check endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Liveness status string (`"healthy"`).
    pub status: String,
    /// Service name (`"event-service"`).
    pub service: String,
    /// Crate version from `CARGO_PKG_VERSION`.
    pub version: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "health",
    responses((status = 200, description = "Service is healthy", body = HealthResponse))
)]
/// Liveness probe: always returns a 200 [`HealthResponse`] naming the
/// service and version. Used by orchestrators and load balancers.
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".into(),
        service: "event-service".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

/// Prometheus metrics endpoint (text-exposition format).
///
/// Renders [`crate::metrics::METRICS`] for scraping. Mounted at the
/// root (`/metrics.prom`) — not under `/api/v1` — so a default
/// Prometheus scrape config (`metrics_path: /metrics.prom`) finds it.
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

// ---------------------------------------------------------------------------
// Event CRUD
// ---------------------------------------------------------------------------

/// Create-event request body: a flattened [`Event`] (the create
/// endpoint deserializes the event JSON directly).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEventRequest {
    /// The event to create, deserialized inline.
    #[serde(flatten)]
    pub event: Event,
}

#[utoipa::path(
    post,
    path = "/api/v1/events",
    tag = "events",
    request_body = Event,
    responses(
        (status = 201, description = "Event created"),
        (status = 409, description = "Potential duplicates detected"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
/// `POST /events` — validate, mint an id if absent, run duplicate
/// detection (returning `409` with candidates when matches are found),
/// persist via the repository, index for search, and return `201`.
pub async fn create_event(
    State(state): State<AppState>,
    Json(mut payload): Json<Event>,
) -> impl IntoResponse {
    let validation_errors = crate::validation::validate_event(&payload);
    if !validation_errors.is_empty() {
        let msg = validation_errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        let err =
            ApiResponse::<Event>::error("VALIDATION_ERROR", format!("Validation failed: {msg}"));
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(err));
    }

    if payload.id == Uuid::nil() {
        payload.id = Uuid::new_v4();
    }

    let duplicates = check_duplicates_internal(&state, &payload).await;
    if !duplicates.is_empty() {
        let dup_response = DuplicateCheckResponse {
            has_duplicates: true,
            potential_matches: duplicates,
        };
        let details = serde_json::to_value(&dup_response).ok();
        let mut err = ApiResponse::<Event>::error(
            "DUPLICATE_DETECTED",
            "Potential duplicate events found. Review matches before proceeding.",
        );
        if let Some(ref mut e) = err.error {
            e.details = details;
        }
        return (StatusCode::CONFLICT, Json(err));
    }

    match state.event_repository.create(&payload).await {
        Ok(event) => {
            if let Err(e) = state.search_engine.index_event(&event) {
                tracing::warn!("Failed to index event: {}", e);
            }
            (StatusCode::CREATED, Json(ApiResponse::success(event)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Event>::error(
                "DATABASE_ERROR",
                format!("Failed to create event: {e}"),
            )),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/events/{id}",
    tag = "events",
    params(("id" = Uuid, Path, description = "Event UUID")),
    responses(
        (status = 200, description = "Event found"),
        (status = 404, description = "Event not found")
    )
)]
/// `GET /events/{id}` — fetch one non-deleted event, or `404`.
pub async fn get_event(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match state.event_repository.get_by_id(&id).await {
        Ok(Some(event)) => (StatusCode::OK, Json(ApiResponse::success(event))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<Event>::error(
                "NOT_FOUND",
                format!("Event with id '{id}' not found"),
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Event>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve event: {e}"),
            )),
        ),
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/events/{id}",
    tag = "events",
    params(("id" = Uuid, Path, description = "Event UUID")),
    request_body = Event,
    responses(
        (status = 200, description = "Event updated"),
        (status = 422, description = "Validation error")
    )
)]
/// `PUT /events/{id}` — validate, force the body's id to the path id,
/// replace the event (and its child rows), re-index, return `200`.
pub async fn update_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut payload): Json<Event>,
) -> impl IntoResponse {
    let validation_errors = crate::validation::validate_event(&payload);
    if !validation_errors.is_empty() {
        let msg = validation_errors
            .iter()
            .map(|e| format!("{}: {}", e.field, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        let err =
            ApiResponse::<Event>::error("VALIDATION_ERROR", format!("Validation failed: {msg}"));
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(err));
    }
    payload.id = id;
    match state.event_repository.update(&payload).await {
        Ok(event) => {
            if let Err(e) = state.search_engine.index_event(&event) {
                tracing::warn!("Failed to update event in search index: {}", e);
            }
            (StatusCode::OK, Json(ApiResponse::success(event)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Event>::error(
                "DATABASE_ERROR",
                format!("Failed to update event: {e}"),
            )),
        ),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/events/{id}",
    tag = "events",
    params(("id" = Uuid, Path, description = "Event UUID")),
    responses(
        (status = 204, description = "Event deleted"),
        (status = 500, description = "Internal server error")
    )
)]
/// `DELETE /events/{id}` — soft-delete the event and remove it from the
/// search index; returns `204`.
pub async fn delete_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.event_repository.delete(&id).await {
        Ok(()) => {
            if let Err(e) = state.search_engine.delete_event(&id.to_string()) {
                tracing::warn!("Failed to delete event from search index: {}", e);
            }
            (StatusCode::NO_CONTENT, Json(ApiResponse::<()>::success(())))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "DATABASE_ERROR",
                format!("Failed to delete event: {e}"),
            )),
        ),
    }
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Query-string parameters for `GET /events/search`.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct SearchQuery {
    /// Free-text query (name / description / keywords / parties / identifiers).
    pub q: String,
    /// Max results to return (defaults to 10, capped at 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Number of leading results to skip (pagination).
    #[serde(default)]
    pub offset: usize,
    /// Use fuzzy title search instead of exact full-text search.
    #[serde(default)]
    pub fuzzy: bool,
    /// Mask sensitive PII (party emails, etc.) before returning.
    #[serde(default)]
    pub mask_sensitive: bool,
    /// Optional ISO date (yyyy-mm-dd) start-date filter range — start.
    pub date_from: Option<String>,
    /// Optional ISO date (yyyy-mm-dd) start-date filter range — end (inclusive).
    pub date_to: Option<String>,
    /// Optional event status filter.
    pub event_status: Option<EventStatus>,
    /// Optional event type filter.
    pub event_type: Option<EventType>,
}

/// Default `limit` for search when unspecified.
fn default_limit() -> usize {
    10
}

/// Paginated search results envelope.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    /// The matching events on this page (post-filter, post-mask).
    pub events: Vec<Event>,
    /// Number of events on this page.
    pub total: usize,
    /// Echo of the original query string.
    pub query: String,
    /// Echo of the requested offset.
    pub offset: usize,
    /// Echo of the effective (capped) limit.
    pub limit: usize,
}

#[utoipa::path(
    get,
    path = "/api/v1/events/search",
    tag = "search",
    params(SearchQuery),
    responses((status = 200, description = "Search results", body = SearchResponse))
)]
/// `GET /events/search` — run a full-text or fuzzy search, hydrate the
/// hit ids from the repository, apply optional status / type / date
/// filters and optional masking, and return a paginated
/// [`SearchResponse`].
pub async fn search_events(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(100);
    let total_needed = params.offset + limit;

    let event_ids = if params.fuzzy {
        state.search_engine.fuzzy_search(&params.q, total_needed)
    } else {
        state.search_engine.search(&params.q, total_needed)
    };

    let ids = match event_ids {
        Ok(ids) => ids,
        Err(e) => {
            let err =
                ApiResponse::<SearchResponse>::error("SEARCH_ERROR", format!("Search failed: {e}"));
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(err));
        }
    };

    let paginated: Vec<_> = ids.into_iter().skip(params.offset).take(limit).collect();
    let mut events = Vec::new();
    for id_str in paginated {
        let id = match Uuid::parse_str(&id_str) {
            Ok(id) => id,
            Err(_) => continue,
        };
        if let Ok(Some(event)) = state.event_repository.get_by_id(&id).await {
            if let Some(ref s) = params.event_status {
                if event.event_status != *s {
                    continue;
                }
            }
            if let Some(ref t) = params.event_type {
                if event.event_type != *t {
                    continue;
                }
            }
            if let Some(from) = params.date_from.as_deref() {
                if event.start_date.strftime("%Y-%m-%d").to_string().as_str() < from {
                    continue;
                }
            }
            if let Some(to) = params.date_to.as_deref() {
                if event.start_date.strftime("%Y-%m-%d").to_string().as_str() > to {
                    continue;
                }
            }
            if params.mask_sensitive {
                events.push(crate::privacy::mask_event(&event));
            } else {
                events.push(event);
            }
        }
    }

    let response = SearchResponse {
        total: events.len(),
        events,
        query: params.q,
        offset: params.offset,
        limit,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// Request body for `POST /events/match`: the probe event (flattened)
/// plus optional score threshold and result limit.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MatchRequest {
    /// The event to find matches for, deserialized inline.
    #[serde(flatten)]
    pub event: Event,
    /// Minimum score to include in results (defaults to 0.5).
    #[serde(default)]
    pub threshold: Option<f64>,
    /// Max number of matches to return (defaults to 10).
    #[serde(default = "default_match_limit")]
    pub limit: usize,
}

/// Default `limit` for the match endpoint when unspecified.
fn default_match_limit() -> usize {
    10
}

/// One scored candidate returned by match / dedup endpoints.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MatchResponse {
    /// The candidate event.
    pub event: Event,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Human label for the score (`certain` / `probable` / `possible`).
    pub quality: String,
    /// Which strategy produced the score (`"probabilistic"`).
    pub detection_method: String,
    /// Optional per-component score breakdown as JSON.
    pub score_breakdown: Option<serde_json::Value>,
}

/// Collection wrapper for match results.
#[derive(Debug, Serialize, ToSchema)]
pub struct MatchResultsResponse {
    /// The scored matches, best first.
    pub matches: Vec<MatchResponse>,
    /// Number of matches returned.
    pub total: usize,
}

#[utoipa::path(
    post,
    path = "/api/v1/events/match",
    tag = "matching",
    request_body = MatchRequest,
    responses((status = 200, description = "Match results", body = MatchResultsResponse))
)]
/// `POST /events/match` — gather blocking candidates by name + date,
/// score them against the probe event, filter by threshold, and return
/// the top matches.
pub async fn match_event(
    State(state): State<AppState>,
    Json(payload): Json<MatchRequest>,
) -> impl IntoResponse {
    let candidates = blocking_candidates(&state, &payload.event).await;

    let match_results = match state.matcher.find_matches(&payload.event, &candidates) {
        Ok(r) => r,
        Err(e) => {
            let err = ApiResponse::<MatchResultsResponse>::error(
                "MATCH_ERROR",
                format!("Matching failed: {e}"),
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(err));
        }
    };

    let threshold = payload.threshold.unwrap_or(0.5);
    let matches: Vec<MatchResponse> = match_results
        .into_iter()
        .filter(|m| m.score >= threshold)
        .take(payload.limit)
        .map(to_match_response)
        .collect();

    let response = MatchResultsResponse {
        total: matches.len(),
        matches,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// Duplicate detection
// ---------------------------------------------------------------------------

/// Result of a duplicate check: whether any candidates crossed the
/// configured threshold, and the matches themselves.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateCheckResponse {
    /// `true` when at least one candidate scored at/above threshold.
    pub has_duplicates: bool,
    /// The scored potential duplicates.
    pub potential_matches: Vec<MatchResponse>,
}

/// Shared duplicate-detection core used by both `create_event` and
/// `check_duplicates`: block by name + date, score, keep candidates at
/// or above the configured `threshold_score`.
async fn check_duplicates_internal(state: &AppState, event: &Event) -> Vec<MatchResponse> {
    let candidates = blocking_candidates(state, event).await;
    state
        .matcher
        .find_matches(event, &candidates)
        .map(|results| {
            results
                .into_iter()
                .filter(|m| m.score >= state.config.matching.threshold_score)
                .map(to_match_response)
                .collect()
        })
        .unwrap_or_default()
}

/// Build the candidate set for matching: query the search index for
/// events with a similar name on the same `start_date` day, hydrate
/// them from the repository, and exclude the probe event's own id.
async fn blocking_candidates(state: &AppState, event: &Event) -> Vec<Event> {
    let date = event.start_date.strftime("%Y-%m-%d").to_string();
    let candidate_ids = state
        .search_engine
        .search_by_name_and_date(&event.name, Some(&date), 100)
        .unwrap_or_default();
    let mut candidates = Vec::new();
    for id_str in candidate_ids {
        let Ok(id) = Uuid::parse_str(&id_str) else {
            continue;
        };
        if id == event.id {
            continue;
        }
        if let Ok(Some(e)) = state.event_repository.get_by_id(&id).await {
            candidates.push(e);
        }
    }
    candidates
}

/// Convert an internal [`MatchResult`](crate::matching::MatchResult)
/// into the API [`MatchResponse`], deriving the `quality` label from
/// the score and serializing the breakdown to JSON.
fn to_match_response(m: crate::matching::MatchResult) -> MatchResponse {
    let quality = if m.score >= 0.95 {
        "certain"
    } else if m.score >= 0.7 {
        "probable"
    } else {
        "possible"
    };
    let breakdown_json = serde_json::to_value(&m.breakdown).ok();
    MatchResponse {
        event: m.event,
        score: m.score,
        quality: quality.into(),
        detection_method: "probabilistic".into(),
        score_breakdown: breakdown_json,
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/events/check-duplicates",
    tag = "deduplication",
    request_body = Event,
    responses((status = 200, description = "Duplicate check result", body = DuplicateCheckResponse))
)]
/// `POST /events/check-duplicates` — run duplicate detection for a
/// candidate event without persisting it; returns a
/// [`DuplicateCheckResponse`].
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(event): Json<Event>,
) -> impl IntoResponse {
    let matches = check_duplicates_internal(&state, &event).await;
    let response = DuplicateCheckResponse {
        has_duplicates: !matches.is_empty(),
        potential_matches: matches,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// Merge
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/events/merge",
    tag = "deduplication",
    request_body = crate::models::MergeRequest,
    responses(
        (status = 200, description = "Merge completed", body = crate::models::MergeResponse),
        (status = 404, description = "Event not found")
    )
)]
/// `POST /events/merge` — merge a duplicate into a surviving main
/// event: union identifiers / alternate names / keywords / locations /
/// parties / `same_as`, record the duplicate's name as an alternate,
/// link main → `Replaces` duplicate, soft-delete the duplicate, update
/// indices, publish a `Merged` event, and return the merge record.
pub async fn merge_events(
    State(state): State<AppState>,
    Json(req): Json<crate::models::MergeRequest>,
) -> impl IntoResponse {
    let main = match state.event_repository.get_by_id(&req.main_event_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "NOT_FOUND",
                    format!("Main event {} not found", req.main_event_id),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to fetch main event: {e}"),
                )),
            );
        }
    };

    let duplicate = match state
        .event_repository
        .get_by_id(&req.duplicate_event_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "NOT_FOUND",
                    format!("Duplicate event {} not found", req.duplicate_event_id),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to fetch duplicate event: {e}"),
                )),
            );
        }
    };

    let mut merged = main.clone();
    let mut transferred = serde_json::Map::new();

    // Identifiers: add ones not already present.
    for id in &duplicate.identifiers {
        let dup = merged
            .identifiers
            .iter()
            .any(|x| x.value == id.value && x.identifier_type == id.identifier_type);
        if !dup {
            merged.identifiers.push(id.clone());
            transferred
                .entry("identifiers".to_string())
                .or_insert_with(|| serde_json::Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
                .push(serde_json::to_value(id).unwrap_or_default());
        }
    }

    // The duplicate's name becomes an alternate name on the main.
    if !merged.alternate_names.contains(&duplicate.name) {
        merged.alternate_names.push(duplicate.name.clone());
    }

    // Union of alternate_names.
    for n in &duplicate.alternate_names {
        if !merged.alternate_names.contains(n) {
            merged.alternate_names.push(n.clone());
        }
    }

    // Union of keywords.
    for k in &duplicate.keywords {
        if !merged.keywords.contains(k) {
            merged.keywords.push(k.clone());
        }
    }

    // Append locations, parties, offers, same_as where not already present.
    for loc in &duplicate.location {
        if !merged.location.iter().any(|l| l == loc) {
            merged.location.push(loc.clone());
        }
    }
    for p in &duplicate.organizers {
        if !merged.organizers.iter().any(|x| x == p) {
            merged.organizers.push(p.clone());
        }
    }
    for p in &duplicate.performers {
        if !merged.performers.iter().any(|x| x == p) {
            merged.performers.push(p.clone());
        }
    }
    for p in &duplicate.attendees {
        if !merged.attendees.iter().any(|x| x == p) {
            merged.attendees.push(p.clone());
        }
    }
    for s in &duplicate.same_as {
        if !merged.same_as.contains(s) {
            merged.same_as.push(s.clone());
        }
    }

    // Link main → replaces duplicate.
    merged.links.push(EventLink {
        other_event_id: duplicate.id,
        link_type: LinkType::Replaces,
    });

    if let Err(e) = state.event_repository.update(&merged).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR",
                format!("Failed to update main event: {e}"),
            )),
        );
    }

    if let Err(e) = state.event_repository.delete(&duplicate.id).await {
        tracing::error!("Failed to soft-delete duplicate event: {}", e);
    }
    if let Err(e) = state.search_engine.delete_event(&duplicate.id.to_string()) {
        tracing::warn!("Failed to remove duplicate from search index: {}", e);
    }
    if let Err(e) = state.search_engine.index_event(&merged) {
        tracing::warn!("Failed to update search index for merged event: {}", e);
    }

    state
        .event_publisher
        .publish(crate::streaming::EventEvent::Merged {
            source_id: duplicate.id,
            target_id: merged.id,
            timestamp: jiff::Timestamp::now(),
        })
        .ok();

    let merge_record = crate::models::MergeRecord {
        id: Uuid::new_v4(),
        main_event_id: merged.id,
        duplicate_event_id: duplicate.id,
        status: crate::models::MergeStatus::Completed,
        merged_by: req.merged_by,
        merge_reason: req.merge_reason,
        match_score: None,
        transferred_data: Some(serde_json::Value::Object(transferred)),
        merged_at: jiff::Timestamp::now(),
    };

    let response = crate::models::MergeResponse {
        merge_record,
        main_event: merged,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// Batch deduplication
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/events/deduplicate",
    tag = "deduplication",
    request_body = crate::models::BatchDeduplicationRequest,
    responses((status = 200, description = "Deduplication results", body = crate::models::BatchDeduplicationResponse))
)]
/// `POST /events/deduplicate` — scan up to 1000 active events pairwise,
/// score each pair, de-dupe symmetric pairs, and emit
/// [`ReviewQueueItem`](crate::models::ReviewQueueItem)s — `AutoMerged`
/// above the auto-merge threshold, otherwise `Pending` for review.
pub async fn batch_deduplicate(
    State(state): State<AppState>,
    Json(req): Json<crate::models::BatchDeduplicationRequest>,
) -> impl IntoResponse {
    let events = match state.event_repository.list_active(1000, 0).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    ApiResponse::<crate::models::BatchDeduplicationResponse>::error(
                        "DATABASE_ERROR",
                        format!("Failed to list events: {e}"),
                    ),
                ),
            );
        }
    };

    let events_scanned = events.len();
    let mut review_items = Vec::new();
    let mut auto_merged = 0usize;
    let mut seen: std::collections::HashSet<(Uuid, Uuid)> = Default::default();

    for (i, event) in events.iter().enumerate() {
        let candidates: Vec<_> = events[i + 1..]
            .iter()
            .take(req.max_candidates)
            .cloned()
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let matches = match state.matcher.find_matches(event, &candidates) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for m in matches {
            if m.score < req.threshold {
                continue;
            }
            let pair = if event.id < m.event.id {
                (event.id, m.event.id)
            } else {
                (m.event.id, event.id)
            };
            if !seen.insert(pair) {
                continue;
            }
            let quality = if m.score >= 0.95 {
                "certain"
            } else if m.score >= 0.7 {
                "probable"
            } else {
                "possible"
            };
            let status = if m.score >= req.auto_merge_threshold {
                auto_merged += 1;
                crate::models::ReviewStatus::AutoMerged
            } else {
                crate::models::ReviewStatus::Pending
            };
            review_items.push(crate::models::ReviewQueueItem {
                id: Uuid::new_v4(),
                event_id_a: event.id,
                event_id_b: m.event.id,
                match_score: m.score,
                match_quality: quality.into(),
                detection_method: "batch_deduplication".into(),
                score_breakdown: serde_json::to_value(&m.breakdown).ok(),
                status,
                reviewed_by: None,
                created_at: jiff::Timestamp::now(),
                reviewed_at: None,
            });
        }
    }

    let queued = review_items
        .iter()
        .filter(|r| r.status == crate::models::ReviewStatus::Pending)
        .count();

    let response = crate::models::BatchDeduplicationResponse {
        events_scanned,
        duplicates_found: review_items.len(),
        auto_merged,
        queued_for_review: queued,
        review_items,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ---------------------------------------------------------------------------
// Privacy: export + masked view
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/events/{id}/export",
    tag = "privacy",
    params(("id" = Uuid, Path, description = "Event UUID")),
    responses(
        (status = 200, description = "Event data export"),
        (status = 404, description = "Event not found")
    )
)]
/// `GET /events/{id}/export` — GDPR right-of-access export of one
/// event's data as JSON, or `404`.
pub async fn export_event_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.event_repository.get_by_id(&id).await {
        Ok(Some(event)) => {
            let export = crate::privacy::export_event_data(&event);
            (StatusCode::OK, Json(ApiResponse::success(export)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(
                "NOT_FOUND",
                format!("Event with id '{id}' not found"),
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve event: {e}"),
            )),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/events/{id}/masked",
    tag = "privacy",
    params(("id" = Uuid, Path, description = "Event UUID")),
    responses(
        (status = 200, description = "Masked event data"),
        (status = 404, description = "Event not found")
    )
)]
/// `GET /events/{id}/masked` — return the event with sensitive fields
/// (identifier values, party emails) masked, or `404`.
pub async fn get_event_masked(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.event_repository.get_by_id(&id).await {
        Ok(Some(event)) => {
            let masked = crate::privacy::mask_event(&event);
            (StatusCode::OK, Json(ApiResponse::success(masked)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<Event>::error(
                "NOT_FOUND",
                format!("Event with id '{id}' not found"),
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Event>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve event: {e}"),
            )),
        ),
    }
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

/// Query parameters for the entity / recent audit-log endpoints.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AuditLogQuery {
    /// Max log rows to return (defaults to 50, capped at 500).
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
}

/// Default `limit` for audit-log queries when unspecified.
fn default_audit_limit() -> i64 {
    50
}

#[utoipa::path(
    get,
    path = "/api/v1/events/{id}/audit",
    tag = "audit",
    params(("id" = Uuid, Path, description = "Event UUID"), AuditLogQuery),
    responses((status = 200, description = "Audit logs retrieved"))
)]
/// `GET /events/{id}/audit` — return the audit trail for one event,
/// newest first.
pub async fn get_event_audit_logs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);
    match state
        .audit_log
        .get_logs_for_entity("Event", id, limit as u64)
        .await
    {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                    "DATABASE_ERROR",
                    format!("Failed to retrieve audit logs: {e}"),
                ),
            ),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/recent",
    tag = "audit",
    params(AuditLogQuery),
    responses((status = 200, description = "Recent audit logs retrieved"))
)]
/// `GET /audit/recent` — return recent system-wide audit activity,
/// newest first.
pub async fn get_recent_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);
    match state.audit_log.get_recent_logs(limit as u64).await {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                    "DATABASE_ERROR",
                    format!("Failed to retrieve audit logs: {e}"),
                ),
            ),
        ),
    }
}

/// Query parameters for the per-user audit-log endpoint.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct UserAuditLogQuery {
    /// The acting user id to filter the audit trail by.
    pub user_id: String,
    /// Max log rows to return (defaults to 50, capped at 500).
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/user",
    tag = "audit",
    params(UserAuditLogQuery),
    responses((status = 200, description = "User audit logs retrieved"))
)]
/// `GET /audit/user` — return the audit trail for one user, newest
/// first.
pub async fn get_user_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<UserAuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);
    match state
        .audit_log
        .get_logs_by_user(&params.user_id, limit as u64)
        .await
    {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                    "DATABASE_ERROR",
                    format!("Failed to retrieve audit logs: {e}"),
                ),
            ),
        ),
    }
}
