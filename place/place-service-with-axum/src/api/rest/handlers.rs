//! REST handlers for the Place Service.
//!
//! Each handler returns bare JSON ([`ok`]) or an RFC 9457 problem
//! ([`ApiProblem`]) with an HTTP status. Errors map: [`crate::Error::NotFound`] → 404,
//! `Validation` → 422, `Conflict` → 409, everything else → 500.

use axa::cursor_pagination::{CursorLimits, CursorPage, PageError};
use axa::extract::{Json, Path, Query};
use axum::{
    extract::{OriginalUri, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::state::AppState;
use crate::api::{ApiProblem, ok};
use crate::db::audit::AuditContext;
use crate::matching::geo::search_box;
use crate::matching::{Evidence, confidence_label};
use crate::models::geo::GeoCoordinates;
use crate::models::merge::{MergeRecord, MergeRequest, MergeResponse};
use crate::models::place::Place;
use crate::privacy::{gdpr_export, mask_place};
use crate::streaming::{EventKind, PlaceEvent};
use crate::validation::{normalize_place, validate_place};

/// `400 Bad Request` problem for a rejected query parameter.
fn bad_request(code: &str, message: impl Into<String>) -> Response {
    ApiProblem::new(code, message.into()).respond(StatusCode::BAD_REQUEST)
}

/// A refused page, a `limit` out of range or a cursor this service did
/// not sign for this query, as this crate's own `422` problem with the
/// `invalid_page` code.
fn invalid_page(error: &PageError) -> Response {
    ApiProblem::new("invalid_page", error.to_string()).respond(error.status())
}

/// The page sizes of a collection read (`agents/share/restful.md`): a
/// page of `default_limit` when none is asked for, and a `limit` above
/// `max_limit` refused with a `422`, never clamped. There is no offset
/// bound: a position comes only from a cursor this service signed, so
/// reaching a deep page means fetching every page before it.
const fn limits(default_limit: u64, max_limit: u64) -> CursorLimits {
    CursorLimits {
        default_limit,
        max_limit,
    }
}

/// The page `?limit=` and `?cursor=` in `uri` name, under `limits`.
fn page_of(uri: &axum::http::Uri, limits: CursorLimits) -> Result<CursorPage, PageError> {
    CursorPage::new(crate::cursor_key::key(), limits, uri)
}

/// `response` with the page's `Link: rel="next"` when `more` says rows
/// follow.
fn with_next(page: &CursorPage, more: bool, mut response: Response) -> Response {
    if more && let Some(link) = page.next_link() {
        response
            .headers_mut()
            .insert(axum::http::header::LINK, link);
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

/// Build a failure response, a problem with the error's status, from a
/// crate error.
fn fail(err: &crate::Error) -> Response {
    ApiProblem::new(code_for(err), err.to_string()).respond(status_for(err))
}

/// The problem `code` for a crate error, matching [`status_for`].
fn code_for(err: &crate::Error) -> &'static str {
    match err {
        crate::Error::NotFound => "not_found",
        crate::Error::Validation(_) => "validation",
        crate::Error::Conflict(_) => "conflict",
        _ => "internal_error",
    }
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
    Json(HealthResponse {
        status: "healthy".into(),
        service: "place-service".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
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

/// Query parameters for `POST /api/places` (T-15): whether to mask
/// sensitive fields on the `409` duplicate-candidate body, same flag
/// name and default (`false`) as [`SearchQuery::mask_sensitive`].
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct CreatePlaceQuery {
    /// Mask sensitive fields on the `409` candidate list.
    pub mask_sensitive: Option<bool>,
}

/// Create a place (with real-time duplicate detection).
#[utoipa::path(post, path = "/api/places", tag = "places",
    params(CreatePlaceQuery),
    request_body = Place,
    responses(
        (status = 201, description = "Created", body = Place),
        (status = 409, description = "Duplicate detected"),
        (status = 422, description = "Validation error"),
    ))]
pub async fn create_place(
    State(state): State<AppState>,
    Query(q): Query<CreatePlaceQuery>,
    Json(mut place): Json<Place>,
) -> impl IntoResponse {
    normalize_place(&mut place);
    let errors = validate_place(&place);
    if !errors.is_empty() {
        return ApiProblem::new("validation", "place failed validation")
            .with_errors(serde_json::to_value(errors).ok())
            .respond(StatusCode::UNPROCESSABLE_ENTITY);
    }

    // `id` is server-managed (see `Place::id`'s docs): a client that omits
    // it — now that the field is `#[serde(default)]` — arrives here as
    // the nil UUID. Mint a fresh one, the same pattern the event service
    // uses, so a hand-written create body never has to invent an id.
    if place.id == Uuid::nil() {
        place.id = Uuid::new_v4();
    }

    // Real-time duplicate detection.
    let candidates = find_candidates(&state, &place, q.mask_sensitive.unwrap_or(false)).await;
    let dups: Vec<ScoredCandidate> = candidates
        .into_iter()
        .filter(|c| c.score >= state.matcher.threshold())
        .collect();
    if !dups.is_empty() {
        return ApiProblem::new("duplicate_detected", "potential duplicates found")
            .with_errors(serde_json::to_value(dups).ok())
            .respond(StatusCode::CONFLICT);
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
            ok(StatusCode::CREATED, stored)
        }
        Err(e) => fail(&e),
    }
}

/// Get a place by id.
#[utoipa::path(get, path = "/api/places/{id}", tag = "places",
    params(("id" = Uuid, Path, description = "Place id")),
    responses((status = 200, body = Place), (status = 404, description = "Not found")))]
pub async fn get_place(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match state.place_repository.get_by_id(&id).await {
        Ok(Some(p)) => ok(StatusCode::OK, p),
        Ok(None) => fail(&crate::Error::NotFound),
        Err(e) => fail(&e),
    }
}

/// Update a place.
#[utoipa::path(put, path = "/api/places/{id}", tag = "places",
    params(("id" = Uuid, Path, description = "Place id")),
    request_body = Place,
    responses((status = 200, body = Place), (status = 404, description = "Not found"),
        (status = 422, description = "Validation error")))]
pub async fn update_place(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut place): Json<Place>,
) -> impl IntoResponse {
    place.id = id;
    normalize_place(&mut place);
    let errors = validate_place(&place);
    if !errors.is_empty() {
        return ApiProblem::new("validation", "place failed validation")
            .with_errors(serde_json::to_value(errors).ok())
            .respond(StatusCode::UNPROCESSABLE_ENTITY);
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
            ok(StatusCode::OK, stored)
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
    /// Page size (default 10, at most 100; anything else is a `422`).
    /// Read from the URI by [`CursorPage::new`], declared here for the
    /// `OpenAPI` document.
    #[param(value_type = Option<u64>)]
    pub limit: Option<String>,
    /// The signed position from a previous page's `Link: rel="next"`.
    /// Read from the URI by [`CursorPage::new`].
    pub cursor: Option<String>,
    /// Use fuzzy matching.
    pub fuzzy: Option<bool>,
    /// Mask sensitive fields in the results.
    pub mask_sensitive: Option<bool>,
}

/// Default page size for `GET /api/places/search` — the cap this
/// endpoint applied before it was paginated, so omitting `limit`
/// returns exactly what it always did.
pub const SEARCH_DEFAULT_LIMIT: usize = 10;

/// Largest page the search endpoint will serve in one call.
pub const SEARCH_MAX_LIMIT: usize = 100;

/// Search response payload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchResponse {
    /// The matched places (hydrated from the DB). No total: a count
    /// would include rows the caller may not see, so whether more follow
    /// is the `Link: rel="next"` header's answer instead.
    pub results: Vec<Place>,
}

/// Full-text / fuzzy place search.
///
/// `GET /api/places/search?q=&limit=&cursor=&fuzzy=&mask_sensitive=`.
/// Returns `200` with a [`SearchResponse`] and a `Link: rel="next"`
/// header while more hits follow (`agents/share/restful.md`). A `limit`
/// above [`SEARCH_MAX_LIMIT`], or a cursor this service did not sign for
/// this query, is a `422`.
#[utoipa::path(get, path = "/api/places/search", tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, body = SearchResponse),
        (status = 422, description = "invalid limit or cursor")
    ))]
pub async fn search_places(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    let page = match page_of(
        &uri,
        limits(SEARCH_DEFAULT_LIMIT as u64, SEARCH_MAX_LIMIT as u64),
    ) {
        Ok(page) => page,
        Err(refused) => return invalid_page(&refused),
    };
    let query = q.q.unwrap_or_default();
    let (ids, _total) = state
        .search_engine
        .search_page(
            &query,
            q.fuzzy.unwrap_or(false),
            usize::try_from(page.limit() + 1).unwrap_or(usize::MAX),
            usize::try_from(page.offset()).unwrap_or(usize::MAX),
        )
        .unwrap_or_default();
    // Whether more follow is the index's answer, decided before the hits
    // resolve: a hit whose row has since been deleted is dropped from the
    // page, and a short page must not end the walk.
    let (ids, more_follow) = page.split(ids);

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
    with_next(
        &page,
        more_follow,
        ok(StatusCode::OK, SearchResponse { results }),
    )
}

/// Query parameters for `GET /api/places` (the plain collection list).
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
pub struct ListQuery {
    /// Page size (default 10, at most 100; anything else is a `422`).
    /// Read from the URI by [`CursorPage::new`], declared here for the
    /// `OpenAPI` document.
    #[param(value_type = Option<u64>)]
    pub limit: Option<String>,
    /// The signed position from a previous page's `Link: rel="next"`.
    /// Read from the URI by [`CursorPage::new`].
    pub cursor: Option<String>,
    /// Mask sensitive fields in the results.
    pub mask_sensitive: Option<bool>,
}

/// Default page size for `GET /api/places` — matches
/// [`SEARCH_DEFAULT_LIMIT`] so switching between the two surfaces
/// doesn't surprise a caller.
pub const LIST_DEFAULT_LIMIT: usize = SEARCH_DEFAULT_LIMIT;

/// Largest page this endpoint will ever serve in one response —
/// matches [`SEARCH_MAX_LIMIT`].
pub const LIST_MAX_LIMIT: usize = SEARCH_MAX_LIMIT;

/// Collection-list response payload — the same shape as
/// [`SearchResponse`], so the two surfaces are interchangeable for a
/// caller that already handles one.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListResponse {
    /// The places on this page. No total: whether more follow is the
    /// `Link: rel="next"` header's answer (`agents/share/restful.md`).
    pub results: Vec<Place>,
}

/// Enumerate every active place, database-backed and paginated.
///
/// `GET /api/places[?limit=&cursor=&mask_sensitive=]` — the genuine
/// "list the collection" endpoint, distinct from `/places/search`:
/// there was previously no way to enumerate places at all (mirroring
/// thing-service's own T-15) — `q="*"` tokenises to nothing
/// (`src/search/mod.rs::tokenise`) and `/places/search` returns zero
/// hits regardless of how many records exist, so an operator (or the
/// front-end's own empty-query default) had to already know a search
/// term. This endpoint pages
/// [`crate::db::PlaceRepository::list`] directly, so its answer is
/// only ever as stale as the database itself, never as stale as a
/// second, independently lifecycled search index.
///
/// Returns `200` with a `Link: rel="next"` header while more rows
/// follow (`agents/share/restful.md`); a `limit` above
/// [`LIST_MAX_LIMIT`], or a cursor this service did not sign for this
/// query, is a `422`.
#[utoipa::path(get, path = "/api/places", tag = "places",
    params(ListQuery),
    responses(
        (status = 200, body = ListResponse),
        (status = 422, description = "invalid limit or cursor")
    ))]
pub async fn list_places(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let page = match page_of(
        &uri,
        limits(LIST_DEFAULT_LIMIT as u64, LIST_MAX_LIMIT as u64),
    ) {
        Ok(page) => page,
        Err(refused) => return invalid_page(&refused),
    };
    let mask_sensitive = q.mask_sensitive.unwrap_or(false);
    // A database failure is a 500, not an empty page: an empty list would
    // tell the caller there are no places.
    let places = match state
        .place_repository
        .list(page.limit() + 1, page.offset())
        .await
    {
        Ok(rows) => rows,
        Err(e) => return fail(&e),
    };
    let (places, more_follow) = page.split(places);
    let results: Vec<Place> = places
        .into_iter()
        .map(|p| if mask_sensitive { mask_place(&p) } else { p })
        .collect();
    with_next(
        &page,
        more_follow,
        ok(StatusCode::OK, ListResponse { results }),
    )
}

/// Every place below one place in the `contained_in_place` hierarchy, at
/// any depth (spec §13 T-2).
///
/// `GET /api/places/{id}/descendants?limit=&cursor=&mask_sensitive=`.
/// One recursive query walks down through active places; the place itself
/// is not in the answer, and a cycle already in the data ends the walk.
/// Places come in `id` order and page by signed cursor like
/// `GET /api/places`, with the same limits. An unknown `{id}` is a `404`.
#[utoipa::path(get, path = "/api/places/{id}/descendants", tag = "places",
    params(("id" = Uuid, Path, description = "Place id"), ListQuery),
    responses(
        (status = 200, body = ListResponse),
        (status = 404, description = "Not found"),
        (status = 422, description = "invalid limit or cursor")
    ))]
pub async fn place_descendants(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Path(id): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let page = match page_of(
        &uri,
        limits(LIST_DEFAULT_LIMIT as u64, LIST_MAX_LIMIT as u64),
    ) {
        Ok(page) => page,
        Err(refused) => return invalid_page(&refused),
    };
    match state.place_repository.get_by_id(&id).await {
        Ok(Some(_)) => {}
        Ok(None) => return fail(&crate::Error::NotFound),
        Err(e) => return fail(&e),
    }
    let rows = match state
        .place_repository
        .page_descendants(id, page.offset(), page.limit() + 1)
        .await
    {
        Ok(rows) => rows,
        Err(e) => return fail(&e),
    };
    let (places, more_follow) = page.split(rows);
    let mask_sensitive = q.mask_sensitive.unwrap_or(false);
    let results: Vec<Place> = places
        .into_iter()
        .map(|p| if mask_sensitive { mask_place(&p) } else { p })
        .collect();
    with_next(
        &page,
        more_follow,
        ok(StatusCode::OK, ListResponse { results }),
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
    /// Page size (default 10, at most 100; anything else is a `422`).
    /// Read from the URI by [`CursorPage::new`], declared here for the
    /// `OpenAPI` document.
    #[param(value_type = Option<u64>)]
    pub limit: Option<String>,
    /// The signed position from a previous page's `Link: rel="next"`.
    /// Read from the URI by [`CursorPage::new`].
    pub cursor: Option<String>,
}

/// Default page size for `GET /api/places/nearby`.
pub const NEARBY_DEFAULT_LIMIT: u64 = 10;

/// Largest page the `nearby` endpoint will serve in one call.
pub const NEARBY_MAX_LIMIT: u64 = 100;

/// Geo-radius search.
///
/// `GET /api/places/nearby?lat=&lon=&radius_km=&limit=&cursor=`. Filters
/// places within `radius_km` of `(lat, lon)`. A bounding box
/// ([`search_box`], over the `idx_places_geo` index, wrapping across the
/// antimeridian) narrows the rows, and the database computes the Haversine
/// distance, keeps those inside the circle, orders them nearest-first and
/// then by `id`, and pages them, so every place in the radius is reached
/// by following `next` however many share the box (spec §13 T-21). Returns `200` with a [`SearchResponse`] and a
/// `Link: rel="next"` header while more in-radius matches follow
/// (`agents/share/restful.md`). `lat`/`lon`/`radius_km` out of range is
/// a `400`; a `limit` above [`NEARBY_MAX_LIMIT`], or a cursor this
/// service did not sign for this query, is a `422`.
#[utoipa::path(get, path = "/api/places/nearby", tag = "search",
    params(NearbyQuery),
    responses(
        (status = 200, body = SearchResponse),
        (status = 400, description = "invalid lat/lon/radius_km"),
        (status = 422, description = "invalid limit or cursor")
    ))]
pub async fn nearby_places(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
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
    let page = match page_of(&uri, limits(NEARBY_DEFAULT_LIMIT, NEARBY_MAX_LIMIT)) {
        Ok(page) => page,
        Err(refused) => return invalid_page(&refused),
    };

    let center = GeoCoordinates::new(q.lat, q.lon);
    let window = match state
        .place_repository
        .page_within_radius(
            q.lat,
            q.lon,
            q.radius_km * 1000.0,
            search_box(&center, q.radius_km),
            page.offset(),
            page.limit() + 1,
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => return fail(&e).into_response(),
    };
    let (results, more_follow) = page.split(window);

    with_next(
        &page,
        more_follow,
        ok(StatusCode::OK, SearchResponse { results }),
    )
}

/// Query for `GET /api/places/search.geojson`.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct BboxQuery {
    /// `west,south,east,north` in degrees (RFC 7946 §5). `west > east`
    /// crosses the antimeridian.
    pub bbox: String,
    /// Page size: default [`GEOJSON_DEFAULT_LIMIT`], at most
    /// [`GEOJSON_MAX_LIMIT`]. A string, so an unparseable value reaches
    /// [`CursorPage::new`] and is a `422`, not an extractor `400`.
    #[param(value_type = Option<u64>)]
    pub limit: Option<String>,
    /// The signed position from a previous page's `Link: rel="next"`.
    pub cursor: Option<String>,
}

/// Default page size for `GET /api/places/search.geojson`.
pub const GEOJSON_DEFAULT_LIMIT: u64 = 10;

/// Largest page `GET /api/places/search.geojson` serves in one call.
pub const GEOJSON_MAX_LIMIT: u64 = 100;

/// The active places inside a bounding box, as a `GeoJSON`
/// `FeatureCollection` (RFC 7946), served as `application/geo+json`.
///
/// `GET /api/places/search.geojson?bbox=west,south,east,north&limit=&cursor=`.
/// The box's edges are inclusive, and `west > east` crosses the
/// antimeridian. Places come in `id` order, paged in the database, so
/// every place in the box is reached by following `Link: rel="next"` and
/// none is skipped by a scan cap. A malformed or out-of-range `bbox` is a
/// `400`; a bad `limit` or cursor is a `422`. Each feature discloses what
/// `GET /api/places/{id}` does.
#[utoipa::path(get, path = "/api/places/search.geojson", tag = "search",
    params(BboxQuery),
    responses(
        (status = 200, description = "GeoJSON FeatureCollection", content_type = "application/geo+json"),
        (status = 400, description = "invalid bbox"),
        (status = 422, description = "invalid limit or cursor")
    ))]
pub async fn places_in_bbox_geojson(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Query(q): Query<BboxQuery>,
) -> impl IntoResponse {
    let bbox = match crate::geojson::BoundingBox::parse(&q.bbox) {
        Ok(bbox) => bbox,
        Err(why) => return bad_request("invalid_bbox", why),
    };
    let page = match page_of(&uri, limits(GEOJSON_DEFAULT_LIMIT, GEOJSON_MAX_LIMIT)) {
        Ok(page) => page,
        Err(refused) => return invalid_page(&refused),
    };
    let rows = match state
        .place_repository
        .page_in_bbox(bbox, page.offset(), page.limit() + 1)
        .await
    {
        Ok(rows) => rows,
        Err(e) => return fail(&e),
    };
    let (places, more_follow) = page.split(rows);
    match crate::geojson::feature_collection(&places) {
        Ok(collection) => {
            let mut response = ok(StatusCode::OK, collection);
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(crate::geojson::MEDIA_TYPE),
            );
            with_next(&page, more_follow, response)
        }
        Err(e) => fail(&crate::Error::Api(e.to_string())),
    }
}

// ─── Match evidence (OpenAPI shapes) ────────────────────────────────────────

/// `OpenAPI` shape of the matcher's [`MatchStrength`]
/// (`agents/share/match-evidence.md` §4). Only `proven` permits an
/// automatic merge.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrengthDoc {
    /// A contradiction rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears the matcher's `proof_bits`, with no contradiction.
    Proven,
    /// A match, with no contradiction, but short of proof.
    Consistent,
    /// Not a match.
    NonMatch,
}

/// `OpenAPI` shape of one hard conflict between two records.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ContradictionDoc {
    /// What conflicts, e.g. `identifier:osm_node`.
    pub field: String,
}

/// `OpenAPI` shape of one term of the evidence sum.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ComponentBitsDoc {
    /// The component, e.g. `name`, `identifier`, `contradiction:identifier:other:gln`.
    pub component: String,
    /// Bits this term added.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

/// `OpenAPI` shape of the place-matcher's [`Evidence`]: how much the compared
/// data **proves**, beside the similarity score
/// (`agents/share/match-evidence.md`). It carries component names and bits
/// only, never field values, so it is returned unchanged on masked paths.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EvidenceDoc {
    /// Sum of the components' log-likelihood weights, in bits.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `contradicted`.
    pub contradictions: Vec<ContradictionDoc>,
    /// The verdict.
    pub strength: MatchStrengthDoc,
    /// Every term of the sum; they add up to `weight_bits`.
    pub components: Vec<ComponentBitsDoc>,
}

/// A scored candidate place returned by match / duplicate endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScoredCandidate {
    /// The candidate place.
    pub place: Place,
    /// Overall match score in `[0.0, 1.0]` (the service's own scorer).
    pub score: f64,
    /// Confidence band (`certain`/`probable`/`possible`/`unlikely`).
    pub confidence: String,
    /// The matcher's evidence for the pair. Names and bits only, so it is
    /// the same whether or not `place` is masked.
    #[schema(value_type = EvidenceDoc)]
    pub evidence: Evidence,
}

/// Score a request place against existing records, returning candidates
/// sorted by descending score.
///
/// `mask_sensitive`: when `true`, each candidate's `place` is redacted
/// via [`mask_place`] before being returned — the same function and
/// default `search_places` uses. Per `agents/share/security.md`
/// invariant 5, a bulk/aggregate read (candidates found via a
/// duplicate probe) must never reveal more than the equivalent single
/// `GET` would; without this, a caller who cannot see a place's full
/// record via `GET` could still recover it by posting a near-duplicate
/// probe to `check-duplicates` or `create` (T-15).
async fn find_candidates(
    state: &AppState,
    place: &Place,
    mask_sensitive: bool,
) -> Vec<ScoredCandidate> {
    let ids = state
        .search_engine
        .search_by_name(&place.name, 50)
        .unwrap_or_default();
    // Cached shares; never waits on a rebuild (`matching::frequency`).
    let frequencies = state.frequencies.get(&state.db);
    let mut out = Vec::new();
    for id in ids {
        let Ok(uuid) = Uuid::parse_str(&id) else {
            continue;
        };
        if uuid == place.id {
            continue;
        }
        if let Ok(Some(existing)) = state.place_repository.get_by_id(&uuid).await {
            let pair = state
                .matcher
                .evaluate(place, &existing, frequencies.as_ref());
            out.push(ScoredCandidate {
                place: if mask_sensitive {
                    mask_place(&existing)
                } else {
                    existing
                },
                score: pair.result.score,
                confidence: confidence_label(&pair.result.confidence).to_string(),
                evidence: pair.evidence,
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

/// Match a candidate place against existing records. Out of T-15's
/// scope (never masked): a caller of `/match` supplies the probe
/// record explicitly to compare it, unlike `check-duplicates`/`create`,
/// where the probe is a record the caller is trying to *persist* and
/// masking closes the "recover a hidden record via a near-duplicate
/// probe" path those two endpoints are exposed to.
#[utoipa::path(post, path = "/api/places/match", tag = "matching",
    request_body = Place,
    responses((status = 200, body = [ScoredCandidate])))]
pub async fn match_place(
    State(state): State<AppState>,
    Json(place): Json<Place>,
) -> impl IntoResponse {
    let candidates = find_candidates(&state, &place, false).await;
    ok(StatusCode::OK, candidates)
}

/// Duplicate-check response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DuplicateCheckResponse {
    /// Whether any candidate scored at/above the match threshold.
    pub duplicates_found: bool,
    /// The scored candidates.
    pub candidates: Vec<ScoredCandidate>,
}

/// Query parameters for `POST /api/places/check-duplicates` (T-15).
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct CheckDuplicatesQuery {
    /// Mask sensitive fields on the returned candidates.
    pub mask_sensitive: Option<bool>,
}

/// Check for duplicates without creating a record.
#[utoipa::path(post, path = "/api/places/check-duplicates", tag = "matching",
    params(CheckDuplicatesQuery),
    request_body = Place,
    responses((status = 200, body = DuplicateCheckResponse)))]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Query(q): Query<CheckDuplicatesQuery>,
    Json(place): Json<Place>,
) -> impl IntoResponse {
    let candidates = find_candidates(&state, &place, q.mask_sensitive.unwrap_or(false)).await;
    let threshold = state.matcher.threshold();
    let duplicates_found = candidates.iter().any(|c| c.score >= threshold);
    ok(
        StatusCode::OK,
        DuplicateCheckResponse {
            duplicates_found,
            candidates,
        },
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
    // SEC-B5: a record cannot be merged into itself. With `main ==
    // duplicate` the merge applies the survivor and then soft-deletes the
    // same id, so the record is gone. Refused before any read, as person,
    // course and the controller-style services already do.
    if req.main_place_id == req.duplicate_place_id {
        return ApiProblem::new(
            "invalid_merge",
            "main_place_id and duplicate_place_id must differ".to_string(),
        )
        .respond(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let main = match state.place_repository.get_by_id(&req.main_place_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return ApiProblem::new("not_found", "main place not found")
                .respond(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            return fail(&e);
        }
    };
    let dup = match state
        .place_repository
        .get_by_id(&req.duplicate_place_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return ApiProblem::new("not_found", "duplicate place not found")
                .respond(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            return fail(&e);
        }
    };

    let transferred = serde_json::to_value(&dup).ok();
    // Soft-delete the duplicate and, under the outbox transport, atomically
    // enqueue the `Merged` (survivor, carrying the duplicate's pid) and
    // `Deleted` (duplicate) outbox rows in one transaction.
    if let Err(e) = state.place_repository.merge(&main, &dup.id).await {
        return fail(&e);
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
            return fail(&e);
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

    ok(
        StatusCode::OK,
        MergeResponse {
            merge_record: record,
            main_place: main,
        },
    )
}

/// Batch-deduplication request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchDeduplicationRequest {
    /// Match threshold (default 0.7).
    #[serde(default)]
    pub threshold: Option<f64>,
    /// Max candidates to scan: default 100, at most
    /// [`DEDUPLICATE_MAX_CANDIDATES`]. Every pair of them is scored, so a
    /// larger value is refused with `422` rather than clamped.
    #[serde(default)]
    pub max_candidates: Option<u64>,
    /// Score at or above which a pair is marked `automerged` (default
    /// [`DEFAULT_AUTO_MERGE_THRESHOLD`]) — and only when its evidence is
    /// also `proven` (`agents/share/match-evidence.md` §4). A `consistent`
    /// or `contradicted` pair stays `pending` at any score.
    #[serde(default)]
    pub auto_merge_threshold: Option<f64>,
}

/// Default [`BatchDeduplicationRequest::auto_merge_threshold`], the
/// family's value (event, person, worker, course).
pub const DEFAULT_AUTO_MERGE_THRESHOLD: f64 = 0.95;

/// The most active places one `POST /api/places/deduplicate` scans. The
/// scan scores every pair, about half a million at this cap, so an
/// unbounded value would let one request occupy the service for as long
/// as the table is large (SEC-M1, bound every input).
pub const DEDUPLICATE_MAX_CANDIDATES: u64 = 1_000;

/// Review disposition of one queued duplicate pair. Serialized with the
/// family's lowercase wire tokens (`pending`, `confirmed`, `rejected`,
/// `automerged`), matching the person/worker services.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Awaiting manual review.
    Pending,
    /// Confirmed as a duplicate — ready for merge.
    Confirmed,
    /// Rejected — not a duplicate.
    Rejected,
    /// Marked for automatic merge: the score clears the auto-merge
    /// threshold **and** the evidence is `proven`. A mark only; this
    /// service performs no merge without an operator.
    AutoMerged,
}

/// One candidate duplicate pair emitted by the batch scan, mirroring the
/// person/worker review-item shape (`detection_method` included) so the
/// family front-ends can render one review queue. Items persist in the
/// stored `review_queue` and are decided via the decision endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    /// Per-component score breakdown explaining [`match_score`](Self::match_score)
    /// (spec §13 T-14) — the matcher's own [`MatchBreakdown`](crate::matching::scoring::MatchBreakdown),
    /// serialized verbatim. `None` only for a row scanned before this field
    /// existed.
    pub score_breakdown: Option<serde_json::Value>,
    /// The matcher's evidence for the pair (`agents/share/match-evidence.md`),
    /// which decides whether it may be auto-merged. `None` for a row
    /// scanned before the field existed; absent in older JSON, which still
    /// deserialises.
    #[serde(default)]
    #[schema(value_type = Option<EvidenceDoc>)]
    pub evidence: Option<Evidence>,
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
    /// Number of pairs marked `automerged`: score at or above the
    /// auto-merge threshold **and** evidence `proven`. A mark only; no
    /// merge is executed.
    pub auto_merged: usize,
    /// Number of pairs left `pending` for human review.
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
    if limit > DEDUPLICATE_MAX_CANDIDATES {
        return ApiProblem::new(
            "validation",
            format!("max_candidates must be at most {DEDUPLICATE_MAX_CANDIDATES}"),
        )
        .respond(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let threshold = req.threshold.unwrap_or_else(|| state.matcher.threshold());
    let auto_merge_threshold = req
        .auto_merge_threshold
        .unwrap_or(DEFAULT_AUTO_MERGE_THRESHOLD);
    let places = match state.place_repository.list(limit, 0).await {
        Ok(rows) => rows,
        Err(e) => return fail(&e),
    };
    // Value frequencies from the records this scan loaded; below
    // MIN_FREQUENCY_POPULATION this is NoFrequencies.
    let frequencies = crate::matching::frequency::Frequencies::from_places(&places);
    let mut review_items = Vec::new();
    // Upper-triangular pair iteration: j starts at i+1 so each pair is
    // scored once and no record is compared with itself. Every pair at
    // or above the threshold becomes a review item; it is marked
    // `automerged` only when the score clears the auto-merge threshold
    // AND the evidence is `proven` (match-evidence.md §4). Everything
    // else, `consistent` and `contradicted` included, stays `pending`.
    for i in 0..places.len() {
        for j in (i + 1)..places.len() {
            let pair = state.matcher.evaluate(&places[i], &places[j], &frequencies);
            let result = &pair.result;
            if result.score >= threshold {
                let status = if pair.may_auto_merge(auto_merge_threshold) {
                    ReviewStatus::AutoMerged
                } else {
                    ReviewStatus::Pending
                };
                review_items.push(ReviewQueueItem {
                    id: Uuid::new_v4(),
                    place_id_a: places[i].id,
                    place_id_b: places[j].id,
                    match_score: result.score,
                    match_quality: confidence_label(&result.confidence).to_string(),
                    detection_method: "batch_deduplication".to_string(),
                    score_breakdown: serde_json::to_value(&result.breakdown).ok(),
                    evidence: Some(pair.evidence.clone()),
                    status,
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
            score_breakdown: r.score_breakdown.clone(),
            evidence: r
                .evidence
                .as_ref()
                .and_then(|e| serde_json::to_value(e).ok()),
            status: review_status_token(&r.status).to_string(),
        })
        .collect();
    let rows = match crate::db::review_queue::upsert(&state.db, &new_items).await {
        Ok(rows) => rows,
        Err(e) => {
            return ApiProblem::new(
                "database_error",
                format!("Failed to persist review queue: {e}"),
            )
            .respond(StatusCode::INTERNAL_SERVER_ERROR);
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
    ok(
        StatusCode::OK,
        BatchDeduplicationResponse {
            places_scanned: places.len(),
            duplicates_found,
            auto_merged,
            queued_for_review,
            review_items,
        },
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
#[derive(Debug, Deserialize, IntoParams)]
pub struct ReviewQueueListQuery {
    /// Optional status-token filter (`pending` / `confirmed` /
    /// `rejected` / `automerged`).
    pub status: Option<String>,
    /// Page size (default [`REVIEW_QUEUE_DEFAULT_LIMIT`], at most
    /// [`REVIEW_QUEUE_MAX_LIMIT`]; anything else is a `422`). Read from the
    /// URI by [`CursorPage::new`], declared here for the `OpenAPI` document.
    #[param(value_type = Option<u64>)]
    pub limit: Option<String>,
    /// The signed position from a previous page's `Link: rel="next"`.
    /// Read from the URI by [`CursorPage::new`].
    pub cursor: Option<String>,
}

/// Default page size for `GET /api/places/review-queue`.
pub const REVIEW_QUEUE_DEFAULT_LIMIT: u64 = 100;

/// Largest review-queue page; a `limit` above it is a `422`, not clamped.
pub const REVIEW_QUEUE_MAX_LIMIT: u64 = 500;

/// Response body for `GET /api/places/review-queue`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReviewQueueListResponse {
    /// The stored review-queue items on this page (newest first).
    pub items: Vec<ReviewQueueItem>,
    /// Number of items on this page, not in the queue: whether more
    /// follow is the `Link: rel="next"` header's answer.
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
        score_breakdown: row.score_breakdown.clone(),
        evidence: row
            .evidence
            .clone()
            .and_then(|v| serde_json::from_value(v).ok()),
        status: parse_review_status(&row.status),
        reviewed_by: row.reviewed_by.clone(),
        created_at: row.created_at,
        reviewed_at: row.reviewed_at,
    }
}

/// List the stored deduplication review queue (newest first), one page
/// at a time: `?limit=&cursor=`, with a `Link: rel="next"` header while
/// more rows follow (`agents/share/restful.md` §Pagination, spec §13 T-20).
#[utoipa::path(get, path = "/api/places/review-queue", tag = "matching",
    params(ReviewQueueListQuery),
    responses(
        (status = 200, description = "Stored review-queue items", body = ReviewQueueListResponse),
        (status = 422, description = "Unknown status token, or an invalid limit or cursor"),
        (status = 500, description = "Internal server error")))]
pub async fn get_review_queue(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    Query(query): Query<ReviewQueueListQuery>,
) -> axum::response::Response {
    if let Some(status) = query.status.as_deref()
        && !matches!(status, "pending" | "confirmed" | "rejected" | "automerged")
    {
        return ApiProblem::new(
            "invalid_status",
            format!("unknown review status `{status}`"),
        )
        .respond(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let page = match page_of(
        &uri,
        limits(REVIEW_QUEUE_DEFAULT_LIMIT, REVIEW_QUEUE_MAX_LIMIT),
    ) {
        Ok(page) => page,
        Err(refused) => return invalid_page(&refused),
    };
    match crate::db::review_queue::list(
        &state.db,
        query.status.as_deref(),
        page.offset(),
        page.limit() + 1,
    )
    .await
    {
        Ok(rows) => {
            let (rows, more_follow) = page.split(rows);
            let items: Vec<ReviewQueueItem> = rows.iter().map(review_row_to_item).collect();
            let total = items.len();
            with_next(
                &page,
                more_follow,
                ok(StatusCode::OK, ReviewQueueListResponse { items, total }),
            )
        }
        Err(e) => ApiProblem::new(
            "database_error",
            format!("Failed to list review queue: {e}"),
        )
        .respond(StatusCode::INTERNAL_SERVER_ERROR),
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
        Ok(crate::db::review_queue::DecideOutcome::Decided(row)) => {
            ok(StatusCode::OK, review_row_to_item(&row))
        }
        Ok(crate::db::review_queue::DecideOutcome::NotFound) => {
            ApiProblem::new("not_found", format!("no review item {id}"))
                .respond(StatusCode::NOT_FOUND)
        }
        Ok(crate::db::review_queue::DecideOutcome::AlreadyDecided(current)) => ApiProblem::new(
            "invalid_review_transition",
            format!("item is `{current}`; only `pending` items can be decided"),
        )
        .respond(StatusCode::UNPROCESSABLE_ENTITY),
        Err(e) => ApiProblem::new(
            "database_error",
            format!("Failed to decide review item: {e}"),
        )
        .respond(StatusCode::INTERNAL_SERVER_ERROR),
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
        Ok(Some(p)) => ok(StatusCode::OK, gdpr_export(&p)),
        Ok(None) => ApiProblem::new("not_found", "place not found").respond(StatusCode::NOT_FOUND),
        Err(e) => fail(&e),
    }
}

/// One place as a `GeoJSON` `Feature` (RFC 7946), served as
/// `application/geo+json`.
///
/// It discloses what `GET /api/places/{id}` does, in another shape: the
/// coordinates become the `Point` geometry and the other fields become
/// properties. A place with no coordinates has a `null` geometry. The
/// path is `/geojson` rather than the `.geojson` suffix first written in
/// spec §13 T-7, because the router matches a parameter only as a whole
/// path segment.
#[utoipa::path(get, path = "/api/places/{id}/geojson", tag = "places",
    params(("id" = Uuid, Path, description = "Place id")),
    responses((status = 200, description = "GeoJSON Feature", content_type = "application/geo+json"),
        (status = 404, description = "Not found")))]
pub async fn place_geojson(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.place_repository.get_by_id(&id).await {
        Ok(Some(p)) => match crate::geojson::place_feature(&p) {
            Ok(feature) => {
                let mut response = ok(StatusCode::OK, feature);
                response.headers_mut().insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static(crate::geojson::MEDIA_TYPE),
                );
                response
            }
            Err(e) => fail(&crate::Error::Api(e.to_string())),
        },
        Ok(None) => fail(&crate::Error::NotFound),
        Err(e) => fail(&e),
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
        Ok(Some(p)) => ok(StatusCode::OK, mask_place(&p)),
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
        Ok(entries) => ok(StatusCode::OK, entries),
        Err(e) => fail(&e),
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
        Ok(entries) => ok(StatusCode::OK, entries),
        Err(e) => fail(&e),
    }
}

/// Default rows examined by the integrity endpoints.
pub const VERIFY_DEFAULT_LIMIT: u64 = 200;

/// Hard cap on rows examined in one call.
///
/// Record verification assembles each row through the repository — one
/// query per row — so an unbounded limit is a denial-of-service on a
/// large table (the SEC-M1 bound-every-input invariant). The cap is
/// lower than the sibling axa services' because their rows are a single
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
    let limit = params
        .limit
        .unwrap_or(VERIFY_DEFAULT_LIMIT)
        .clamp(1, VERIFY_MAX_LIMIT);
    let rows = match crate::db::rows_for_verification(&state.db, limit).await {
        Ok(rows) => rows,
        Err(e) => {
            return ApiProblem::new(
                "database_error",
                format!("Failed to read records for verification: {e}"),
            )
            .respond(StatusCode::INTERNAL_SERVER_ERROR);
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
                return ApiProblem::new(
                    "database_error",
                    format!("Failed to assemble a record for verification: {e}"),
                )
                .respond(StatusCode::INTERNAL_SERVER_ERROR);
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
    let limit = params
        .limit
        .unwrap_or(VERIFY_DEFAULT_LIMIT)
        .clamp(1, VERIFY_MAX_LIMIT);
    match state.audit_log.rows_for_verification(limit).await {
        Ok(rows) => (
            StatusCode::OK,
            Json(crate::compliance::audit_integrity::verify(&rows)),
        )
            .into_response(),
        Err(e) => ApiProblem::new(
            "database_error",
            format!("Failed to read audit rows for verification: {e}"),
        )
        .respond(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;

    fn page(query: &str, limits: CursorLimits) -> Result<CursorPage, PageError> {
        let uri = format!("/api/places{query}").parse().unwrap();
        page_of(&uri, limits)
    }

    /// This crate's bounds: no `limit` is the default, a `limit` up to
    /// the max is taken, and anything else is refused, never clamped, as
    /// a `422` `invalid_page`.
    #[test]
    fn a_page_takes_its_limit_or_refuses_it() {
        let list = limits(LIST_DEFAULT_LIMIT as u64, LIST_MAX_LIMIT as u64);
        assert_eq!(page("", list).unwrap().limit(), LIST_DEFAULT_LIMIT as u64);
        assert_eq!(
            page(&format!("?limit={LIST_MAX_LIMIT}"), list)
                .unwrap()
                .limit(),
            LIST_MAX_LIMIT as u64
        );
        let nearby = limits(NEARBY_DEFAULT_LIMIT, NEARBY_MAX_LIMIT);
        assert_eq!(page("", nearby).unwrap().limit(), NEARBY_DEFAULT_LIMIT);
        for refused in [
            "?limit=0",
            "?limit=lots",
            &format!("?limit={}", LIST_MAX_LIMIT + 1),
        ] {
            let error = page(refused, list).expect_err(refused);
            assert_eq!(
                invalid_page(&error).status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{refused}"
            );
        }
    }

    /// A cursor this service did not sign, such as a caller's own offset
    /// in disguise, is refused rather than read.
    #[test]
    fn an_unsigned_cursor_is_refused() {
        let list = limits(LIST_DEFAULT_LIMIT as u64, LIST_MAX_LIMIT as u64);
        assert!(page("?cursor=AQAAAAAAAAAK", list).is_err());
    }

    /// The next link is only sent while more rows follow.
    #[test]
    fn the_next_link_follows_more() {
        let uri = "/api/places/search?q=park&limit=2".parse().unwrap();
        let page = page_of(
            &uri,
            limits(SEARCH_DEFAULT_LIMIT as u64, SEARCH_MAX_LIMIT as u64),
        )
        .unwrap();
        let body = || ok(StatusCode::OK, SearchResponse { results: vec![] });
        let more = with_next(&page, true, body());
        let link = more.headers()["link"].to_str().unwrap();
        assert!(
            link.starts_with("</api/places/search?q=park&limit=2&cursor="),
            "{link}"
        );
        assert!(
            with_next(&page, false, body())
                .headers()
                .get("link")
                .is_none()
        );
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
            score_breakdown: None,
            evidence: None,
            status: ReviewStatus::Pending,
            reviewed_by: None,
            created_at: chrono::Utc::now(),
            reviewed_at: None,
        };
        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(value["detection_method"], "batch_deduplication");
        assert_eq!(value["status"], "pending");
    }

    /// A review item serialized before `evidence` existed still
    /// deserialises, with `evidence: None` (the migration leaves old rows
    /// NULL, and old JSON simply lacks the key).
    #[test]
    fn old_review_item_json_without_evidence_deserialises() {
        let old = serde_json::json!({
            "id": Uuid::new_v4(),
            "place_id_a": Uuid::new_v4(),
            "place_id_b": Uuid::new_v4(),
            "match_score": 0.9,
            "match_quality": "probable",
            "detection_method": "batch_deduplication",
            "score_breakdown": null,
            "status": "pending",
            "reviewed_by": null,
            "created_at": "2026-07-19T00:00:00Z",
            "reviewed_at": null
        });
        let item: ReviewQueueItem = serde_json::from_value(old).unwrap();
        assert!(item.evidence.is_none());
        assert_eq!(item.status, ReviewStatus::Pending);
    }

    /// Evidence round-trips through the stored JSONB form, and a stored
    /// row's evidence that is not valid evidence reads as `None` rather
    /// than failing the list.
    #[test]
    fn stored_evidence_maps_onto_the_item() {
        let engine = crate::matching::matcher_lib::MatchingEngine::default_config();
        let a = crate::matching::matcher_lib::Place::builder()
            .name("X")
            .build();
        let evidence = engine.match_places(&a, &a).evidence;
        let row = crate::db::review_queue::ReviewQueueRow {
            id: Uuid::new_v4(),
            record_id_a: Uuid::new_v4(),
            record_id_b: Uuid::new_v4(),
            match_score: 1.0,
            match_quality: "certain".into(),
            detection_method: "batch_deduplication".into(),
            score_breakdown: None,
            evidence: Some(serde_json::to_value(&evidence).unwrap()),
            status: "pending".into(),
            reviewed_by: None,
            created_at: chrono::Utc::now(),
            reviewed_at: None,
        };
        assert_eq!(review_row_to_item(&row).evidence, Some(evidence));
        let garbled = crate::db::review_queue::ReviewQueueRow {
            evidence: Some(serde_json::json!("nonsense")),
            ..row
        };
        assert!(review_row_to_item(&garbled).evidence.is_none());
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
