//! REST handlers for the Place Service.
//!
//! Each handler returns the standard [`ApiResponse`] envelope wrapped in an
//! HTTP status. Errors map: [`crate::Error::NotFound`] → 404,
//! `Validation` → 422, `Conflict` → 409, everything else → 500.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::state::AppState;
use crate::api::ApiResponse;
use crate::db::audit::AuditContext;
use crate::matching::confidence_label;
use crate::matching::geo::{bounding_box, within_radius};
use crate::models::geo::GeoCoordinates;
use crate::models::merge::{MergeRecord, MergeRequest, MergeResponse};
use crate::models::place::Place;
use crate::privacy::{gdpr_export, mask_place};
use crate::streaming::{EventKind, PlaceEvent};
use crate::validation::{normalize_place, validate_place};

/// Largest accepted `offset` on a paginated collection read
/// (`agents/share/restful.md`, SEC-G7). Past this a request is a `400`:
/// the database (or, for `nearby`, the in-process filter) would
/// otherwise have to materialise and discard arbitrarily many rows,
/// which is a cheap denial of service. Deep paging past this bound
/// wants a cursor, not a bigger number.
pub const MAX_OFFSET: u64 = 10_000;

/// `400 Bad Request` envelope for a rejected query parameter.
fn bad_request(code: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse::<serde_json::Value>::error(
            code,
            message.into(),
        )),
    )
        .into_response()
}

/// `400` for an `offset` beyond [`MAX_OFFSET`].
fn offset_too_large() -> Response {
    bad_request(
        "offset_too_large",
        format!("offset must not exceed {MAX_OFFSET}; narrow the query instead"),
    )
}

/// Stamp the pagination headers onto a response
/// (`agents/share/restful.md`): `X-Total-Count` is the total ignoring
/// the page window, `X-Limit`/`X-Offset` are the limit/offset actually
/// applied — so a caller that sent neither still learns the defaults.
fn with_page_headers(mut response: Response, total: u64, limit: u64, offset: u64) -> Response {
    let headers = response.headers_mut();
    for (name, value) in [
        ("x-total-count", total),
        ("x-limit", limit),
        ("x-offset", offset),
    ] {
        if let Ok(v) = axum::http::HeaderValue::from_str(&value.to_string()) {
            headers.insert(name, v);
        }
    }
    response
}

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

    // `id` is server-managed (see `Place::id`'s docs): a client that omits
    // it — now that the field is `#[serde(default)]` — arrives here as
    // the nil UUID. Mint a fresh one, the same pattern the event service
    // uses, so a hand-written create body never has to invent an id.
    if place.id == Uuid::nil() {
        place.id = Uuid::new_v4();
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
    /// Rows to skip (default 0). Bounded by [`MAX_OFFSET`]; an `offset`
    /// beyond that is a `400`.
    pub offset: Option<u64>,
    /// Use fuzzy matching.
    pub fuzzy: Option<bool>,
    /// Mask sensitive fields in the results.
    pub mask_sensitive: Option<bool>,
}

/// Default page size for `GET /api/places/search` — the cap this
/// endpoint applied before `offset` existed, so omitting `limit`
/// returns exactly what it always did.
pub const SEARCH_DEFAULT_LIMIT: usize = 10;

/// Largest page the search endpoint will serve in one call.
pub const SEARCH_MAX_LIMIT: usize = 100;

/// Search response payload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchResponse {
    /// The matched places (hydrated from the DB).
    pub results: Vec<Place>,
    /// Number of results returned.
    pub total: usize,
}

/// Full-text / fuzzy place search.
///
/// `GET /api/places/search?q=&limit=&offset=&fuzzy=&mask_sensitive=`.
/// Returns `200` with a [`SearchResponse`] plus the `X-Total-Count` /
/// `X-Limit` / `X-Offset` pagination headers
/// (`agents/share/restful.md`) — the total is the true index match
/// count, not the number of rows this page returned. An `offset`
/// beyond [`MAX_OFFSET`] is a `400`.
#[utoipa::path(get, path = "/api/places/search", tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, body = SearchResponse),
        (status = 400, description = "offset too large", body = crate::api::ApiError)
    ))]
pub async fn search_places(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    let offset = q.offset.unwrap_or(0);
    if offset > MAX_OFFSET {
        return offset_too_large();
    }
    let limit = q
        .limit
        .filter(|l| *l > 0)
        .unwrap_or(SEARCH_DEFAULT_LIMIT)
        .min(SEARCH_MAX_LIMIT);
    let query = q.q.unwrap_or_default();
    let offset_usize = usize::try_from(offset).unwrap_or(usize::MAX);
    let (ids, total) = state
        .search_engine
        .search_page(&query, q.fuzzy.unwrap_or(false), limit, offset_usize)
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
    let count = results.len();
    let response = (
        StatusCode::OK,
        Json(ApiResponse::success(SearchResponse {
            results,
            total: count,
        })),
    )
        .into_response();
    with_page_headers(
        response,
        u64::try_from(total).unwrap_or(u64::MAX),
        u64::try_from(limit).unwrap_or(u64::MAX),
        offset,
    )
}

/// Query parameters for `GET /api/places/nearby`.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct NearbyQuery {
    /// Center latitude, decimal degrees (`-90..=90`).
    pub lat: f64,
    /// Center longitude, decimal degrees (`-180..=180`).
    pub lon: f64,
    /// Search radius in kilometers (non-negative, finite).
    pub radius_km: f64,
    /// Max results (default 10, capped at 100).
    pub limit: Option<u64>,
    /// Rows to skip (default 0). Bounded by [`MAX_OFFSET`]; an `offset`
    /// beyond that is a `400`.
    pub offset: Option<u64>,
}

/// Default page size for `GET /api/places/nearby`.
pub const NEARBY_DEFAULT_LIMIT: u64 = 10;

/// Largest page the `nearby` endpoint will serve in one call.
pub const NEARBY_MAX_LIMIT: u64 = 100;

/// Safety cap on how many bounding-box candidates are read from the
/// database before the exact Haversine filter runs. Bounds the request
/// to a fixed amount of work regardless of how large a box a caller's
/// `radius_km` produces (SEC-M1: bound every input).
pub const NEARBY_BBOX_SCAN_CAP: u64 = 5_000;

/// Geo-radius search.
///
/// `GET /api/places/nearby?lat=&lon=&radius_km=&limit=&offset=`. Filters
/// places within `radius_km` of `(lat, lon)`: a coarse SQL bounding-box
/// pre-filter (`matching::geo::bounding_box`, over the `idx_places_geo`
/// index) narrows the candidates, then the exact
/// [`within_radius`](crate::matching::geo::within_radius) Haversine
/// check keeps only those truly inside the circle. Results are ordered
/// nearest-first. Returns `200` with a [`SearchResponse`] plus the
/// `X-Total-Count` / `X-Limit` / `X-Offset` pagination headers
/// (`agents/share/restful.md`) — the total is every in-radius match,
/// ignoring the page window. `lat`/`lon`/`radius_km` out of range, or an
/// `offset` beyond [`MAX_OFFSET`], is a `400`.
#[utoipa::path(get, path = "/api/places/nearby", tag = "search",
    params(NearbyQuery),
    responses(
        (status = 200, body = SearchResponse),
        (status = 400, description = "invalid lat/lon/radius_km, or offset too large", body = crate::api::ApiError)
    ))]
pub async fn nearby_places(
    State(state): State<AppState>,
    Query(q): Query<NearbyQuery>,
) -> impl IntoResponse {
    if !(-90.0..=90.0).contains(&q.lat) {
        return bad_request("invalid_latitude", "lat must be in -90..=90");
    }
    if !(-180.0..=180.0).contains(&q.lon) {
        return bad_request("invalid_longitude", "lon must be in -180..=180");
    }
    if !q.radius_km.is_finite() || q.radius_km < 0.0 {
        return bad_request(
            "invalid_radius",
            "radius_km must be a non-negative finite number",
        );
    }
    let offset = q.offset.unwrap_or(0);
    if offset > MAX_OFFSET {
        return offset_too_large();
    }
    let limit = q
        .limit
        .filter(|l| *l > 0)
        .unwrap_or(NEARBY_DEFAULT_LIMIT)
        .min(NEARBY_MAX_LIMIT);

    let center = GeoCoordinates::new(q.lat, q.lon);
    let (lat_min, lat_max, lon_min, lon_max) = bounding_box(&center, q.radius_km);
    let candidates = match state
        .place_repository
        .list_in_bbox(lat_min, lat_max, lon_min, lon_max, NEARBY_BBOX_SCAN_CAP)
        .await
    {
        Ok(c) => c,
        Err(e) => return fail(&e).into_response(),
    };

    let radius_m = q.radius_km * 1000.0;
    let mut within: Vec<(f64, Place)> = candidates
        .into_iter()
        .filter_map(|p| {
            let geo = p.geo.as_ref()?;
            let dist = geo.distance_to(&center);
            within_radius(geo, &center, radius_m).then_some((dist, p))
        })
        .collect();
    // Nearest-first, so pagination is a meaningful "closer" ordering
    // rather than an arbitrary one.
    within.sort_by(|(da, _), (db, _)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal));

    let total = within.len();
    let offset_usize = usize::try_from(offset).unwrap_or(usize::MAX);
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let results: Vec<Place> = within
        .into_iter()
        .skip(offset_usize)
        .take(limit_usize)
        .map(|(_dist, p)| p)
        .collect();
    let count = results.len();

    let response = (
        StatusCode::OK,
        Json(ApiResponse::success(SearchResponse {
            results,
            total: count,
        })),
    )
        .into_response();
    with_page_headers(
        response,
        u64::try_from(total).unwrap_or(u64::MAX),
        limit,
        offset,
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
    // Soft-delete the duplicate and, under the outbox transport, atomically
    // enqueue the `Merged` (survivor, carrying the duplicate's pid) and
    // `Deleted` (duplicate) outbox rows in one transaction.
    if let Err(e) = state.place_repository.merge(&main, &dup.id).await {
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
    /// First place in the candidate pair.
    pub place_id_a: Uuid,
    /// Second place in the candidate pair.
    pub place_id_b: Uuid,
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
    /// Number of places scanned.
    pub places_scanned: usize,
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
    let mut review_items = Vec::new();
    // Upper-triangular pair iteration: j starts at i+1 so each pair is
    // scored once and no record is compared with itself. Every pair at
    // or above the threshold becomes a pending review item (the
    // person/worker report shape).
    for i in 0..places.len() {
        for j in (i + 1)..places.len() {
            let result = state.matcher.score(&places[i], &places[j]);
            if result.score >= threshold {
                review_items.push(ReviewQueueItem {
                    id: Uuid::new_v4(),
                    place_id_a: places[i].id,
                    place_id_b: places[j].id,
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
            record_id_a: r.place_id_a,
            record_id_b: r.place_id_b,
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
            places_scanned: places.len(),
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

/// Request body for `POST /api/places/review-queue/{id}/decision`.
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

/// Response envelope for `GET /api/places/review-queue`.
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
        place_id_a: row.record_id_a,
        place_id_b: row.record_id_b,
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
#[utoipa::path(get, path = "/api/places/review-queue", tag = "matching",
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
#[utoipa::path(post, path = "/api/places/review-queue/{id}/decision", tag = "matching",
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
    let rows = match crate::db::models::places::Entity::find()
        .order_by_desc(crate::db::models::places::Column::UpdatedAt)
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
        match state.place_repository.get_by_id(&row.id).await {
            Ok(Some(place)) => records.push(crate::compliance::record_integrity::StoredRecord {
                place,
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
            place_id_a: Uuid::new_v4(),
            place_id_b: Uuid::new_v4(),
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
