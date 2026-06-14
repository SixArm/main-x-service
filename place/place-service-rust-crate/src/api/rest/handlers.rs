//! REST handlers for the Place Service.
//!
//! Each handler returns the standard [`ApiResponse`] envelope wrapped in an
//! HTTP status. Errors map: [`crate::Error::NotFound`] → 404,
//! `Validation` → 422, `Conflict` → 409, everything else → 500.

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
use crate::models::place::Place;
use crate::privacy::{gdpr_export, mask_place};
use crate::streaming::{EventKind, PlaceEvent};
use crate::validation::{normalize_place, validate_place};

/// Map a crate error to the HTTP status the REST layer returns for it.
fn status_for(err: &crate::Error) -> StatusCode {
    match err {
        crate::Error::NotFound => StatusCode::NOT_FOUND,
        crate::Error::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
        crate::Error::Conflict(_) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Build a failure response `(status, Json(envelope))` from a crate error.
fn fail(err: &crate::Error) -> (StatusCode, Json<ApiResponse<Place>>) {
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
        service: "place-service".into(),
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

/// Create a place (with real-time duplicate detection).
#[utoipa::path(post, path = "/api/places", tag = "places",
    request_body = Place,
    responses(
        (status = 201, description = "Created", body = Place),
        (status = 409, description = "Duplicate detected", body = crate::api::ApiError),
        (status = 422, description = "Validation error", body = crate::api::ApiError),
    ))]
pub async fn create_place(
    State(state): State<AppState>,
    Json(mut place): Json<Place>,
) -> impl IntoResponse {
    normalize_place(&mut place);
    let errors = validate_place(&place);
    if !errors.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error_with_details(
                "validation_error",
                "place failed validation",
                errors,
            )),
        );
    }

    // Real-time duplicate detection.
    let candidates = find_candidates(&state, &place).await;
    let dups: Vec<ScoredCandidate> = candidates
        .into_iter()
        .filter(|c| c.score >= state.matcher.threshold())
        .collect();
    if !dups.is_empty() {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error_with_details(
                "duplicate_detected",
                "potential duplicates found",
                dups,
            )),
        );
    }

    match state.place_repository.create(&place).await {
        Ok(stored) => {
            let _ = state.search_engine.index_place(&stored);
            let _ = state
                .event_publisher
                .publish(PlaceEvent::new(
                    EventKind::PlaceCreated,
                    stored.id,
                    serde_json::json!({ "name": stored.name }),
                ))
                .await;
            if let Ok(v) = serde_json::to_value(&stored) {
                let _ = state
                    .audit_log
                    .log_create("place", stored.id, v, &AuditContext::default())
                    .await;
            }
            (StatusCode::CREATED, Json(ApiResponse::success(stored)))
        }
        Err(e) => (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        ),
    }
}

/// Get a place by id.
#[utoipa::path(get, path = "/api/places/{id}", tag = "places",
    params(("id" = Uuid, Path, description = "Place id")),
    responses((status = 200, body = Place), (status = 404, description = "Not found")))]
pub async fn get_place(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match state.place_repository.get_by_id(&id).await {
        Ok(Some(p)) => (StatusCode::OK, Json(ApiResponse::success(p))),
        Ok(None) => fail(&crate::Error::NotFound),
        Err(e) => fail(&e),
    }
}

/// Update a place.
#[utoipa::path(put, path = "/api/places/{id}", tag = "places",
    params(("id" = Uuid, Path, description = "Place id")),
    request_body = Place,
    responses((status = 200, body = Place), (status = 404, description = "Not found"),
        (status = 422, description = "Validation error", body = crate::api::ApiError)))]
pub async fn update_place(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut place): Json<Place>,
) -> impl IntoResponse {
    place.id = id;
    normalize_place(&mut place);
    let errors = validate_place(&place);
    if !errors.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error_with_details(
                "validation_error",
                "place failed validation",
                errors,
            )),
        );
    }
    let old = state.place_repository.get_by_id(&id).await.ok().flatten();
    match state.place_repository.update(&place).await {
        Ok(stored) => {
            let _ = state.search_engine.delete_place(&id.to_string());
            let _ = state.search_engine.index_place(&stored);
            let _ = state
                .event_publisher
                .publish(PlaceEvent::new(
                    EventKind::PlaceUpdated,
                    stored.id,
                    serde_json::json!({ "name": stored.name }),
                ))
                .await;
            if let (Some(old), Ok(new_v)) = (old, serde_json::to_value(&stored))
                && let Ok(old_v) = serde_json::to_value(&old)
            {
                let _ = state
                    .audit_log
                    .log_update("place", stored.id, old_v, new_v, &AuditContext::default())
                    .await;
            }
            (StatusCode::OK, Json(ApiResponse::success(stored)))
        }
        Err(e) => fail(&e),
    }
}

/// Soft-delete a place.
#[utoipa::path(delete, path = "/api/places/{id}", tag = "places",
    params(("id" = Uuid, Path, description = "Place id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_place(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let old = state.place_repository.get_by_id(&id).await.ok().flatten();
    match state.place_repository.soft_delete(&id).await {
        Ok(()) => {
            let _ = state.search_engine.delete_place(&id.to_string());
            let _ = state
                .event_publisher
                .publish(PlaceEvent::new(
                    EventKind::PlaceDeleted,
                    id,
                    serde_json::json!({}),
                ))
                .await;
            if let Some(old) = old
                && let Ok(v) = serde_json::to_value(&old)
            {
                let _ = state
                    .audit_log
                    .log_delete("place", id, v, &AuditContext::default())
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
    /// The matched places (hydrated from the DB).
    pub results: Vec<Place>,
    /// Number of results returned.
    pub total: usize,
}

/// Full-text / fuzzy place search.
#[utoipa::path(get, path = "/api/places/search", tag = "search",
    params(SearchQuery),
    responses((status = 200, body = SearchResponse)))]
pub async fn search_places(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(10).min(100);
    let query = q.q.unwrap_or_default();
    let ids = if q.fuzzy.unwrap_or(false) {
        state.search_engine.fuzzy_search(&query, limit)
    } else {
        state.search_engine.search(&query, limit)
    }
    .unwrap_or_default();

    let mut results = Vec::new();
    for id in ids {
        if let Ok(uuid) = Uuid::parse_str(&id)
            && let Ok(Some(p)) = state.place_repository.get_by_id(&uuid).await
        {
            results.push(if q.mask_sensitive.unwrap_or(false) {
                mask_place(&p)
            } else {
                p
            });
        }
    }
    let total = results.len();
    (
        StatusCode::OK,
        Json(ApiResponse::success(SearchResponse { results, total })),
    )
}

/// A scored candidate place returned by match / duplicate endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScoredCandidate {
    /// The candidate place.
    pub place: Place,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Confidence band (`certain`/`probable`/`possible`/`unlikely`).
    pub confidence: String,
}

/// Score a request place against existing records, returning candidates
/// sorted by descending score.
async fn find_candidates(state: &AppState, place: &Place) -> Vec<ScoredCandidate> {
    let ids = state
        .search_engine
        .search_by_name(&place.name, 50)
        .unwrap_or_default();
    let mut out = Vec::new();
    for id in ids {
        let Ok(uuid) = Uuid::parse_str(&id) else {
            continue;
        };
        if uuid == place.id {
            continue;
        }
        if let Ok(Some(existing)) = state.place_repository.get_by_id(&uuid).await {
            let r = state.matcher.score(place, &existing);
            out.push(ScoredCandidate {
                place: existing,
                score: r.score,
                confidence: confidence_label(&r.confidence).to_string(),
            });
        }
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Match a candidate place against existing records.
#[utoipa::path(post, path = "/api/places/match", tag = "matching",
    request_body = Place,
    responses((status = 200, body = [ScoredCandidate])))]
pub async fn match_place(
    State(state): State<AppState>,
    Json(place): Json<Place>,
) -> impl IntoResponse {
    let candidates = find_candidates(&state, &place).await;
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
#[utoipa::path(post, path = "/api/places/check-duplicates", tag = "matching",
    request_body = Place,
    responses((status = 200, body = DuplicateCheckResponse)))]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(place): Json<Place>,
) -> impl IntoResponse {
    let candidates = find_candidates(&state, &place).await;
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

/// Merge a duplicate place into a surviving main place.
#[utoipa::path(post, path = "/api/places/merge", tag = "matching",
    request_body = MergeRequest,
    responses((status = 200, body = MergeResponse), (status = 404, description = "Not found")))]
pub async fn merge_places(
    State(state): State<AppState>,
    Json(req): Json<MergeRequest>,
) -> impl IntoResponse {
    let main = match state.place_repository.get_by_id(&req.main_place_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("not_found", "main place not found")),
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
        .place_repository
        .get_by_id(&req.duplicate_place_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("not_found", "duplicate place not found")),
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
    if let Err(e) = state.place_repository.soft_delete(&dup.id).await {
        return (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        );
    }
    let _ = state.search_engine.delete_place(&dup.id.to_string());

    let record = MergeRecord {
        id: Uuid::new_v4(),
        main_place_id: main.id,
        duplicate_place_id: dup.id,
        merge_reason: req.merge_reason.clone(),
        transferred_data: transferred,
        merged_at: chrono::Utc::now(),
    };
    let record = match state.place_repository.record_merge(&record).await {
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
        .publish(PlaceEvent::new(
            EventKind::PlaceMerged,
            main.id,
            serde_json::json!({ "duplicate": dup.id }),
        ))
        .await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(MergeResponse {
            merge_record: record,
            main_place: main,
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

/// Batch-deduplication response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    /// Number of places scanned.
    pub places_scanned: usize,
    /// Number of duplicate pairs found.
    pub duplicates_found: usize,
}

/// Batch deduplication scan over all active places.
#[utoipa::path(post, path = "/api/places/deduplicate", tag = "matching",
    request_body = BatchDeduplicationRequest,
    responses((status = 200, body = BatchDeduplicationResponse)))]
pub async fn deduplicate(
    State(state): State<AppState>,
    Json(req): Json<BatchDeduplicationRequest>,
) -> impl IntoResponse {
    let limit = req.max_candidates.unwrap_or(100);
    let threshold = req.threshold.unwrap_or_else(|| state.matcher.threshold());
    let places = state
        .place_repository
        .list(limit, 0)
        .await
        .unwrap_or_default();
    let mut duplicates_found = 0usize;
    for i in 0..places.len() {
        for j in (i + 1)..places.len() {
            if state.matcher.score(&places[i], &places[j]).score >= threshold {
                duplicates_found += 1;
            }
        }
    }
    (
        StatusCode::OK,
        Json(ApiResponse::success(BatchDeduplicationResponse {
            places_scanned: places.len(),
            duplicates_found,
        })),
    )
}

/// GDPR data export for one place.
#[utoipa::path(get, path = "/api/places/{id}/export", tag = "privacy",
    params(("id" = Uuid, Path, description = "Place id")),
    responses((status = 200, description = "Full place export"), (status = 404, description = "Not found")))]
pub async fn export_place_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.place_repository.get_by_id(&id).await {
        Ok(Some(p)) => (StatusCode::OK, Json(ApiResponse::success(gdpr_export(&p)))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("not_found", "place not found")),
        ),
        Err(e) => (
            status_for(&e),
            Json(ApiResponse::error("error", e.to_string())),
        ),
    }
}

/// Masked place view.
#[utoipa::path(get, path = "/api/places/{id}/masked", tag = "privacy",
    params(("id" = Uuid, Path, description = "Place id")),
    responses((status = 200, body = Place), (status = 404, description = "Not found")))]
pub async fn masked_place(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.place_repository.get_by_id(&id).await {
        Ok(Some(p)) => (StatusCode::OK, Json(ApiResponse::success(mask_place(&p)))),
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

/// Audit log for one place.
#[utoipa::path(get, path = "/api/places/{id}/audit", tag = "audit",
    params(("id" = Uuid, Path, description = "Place id"), AuditQuery),
    responses((status = 200, body = [crate::db::audit::AuditEntry])))]
pub async fn audit_for_place(
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
