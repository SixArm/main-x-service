//! REST API surface — Axum router + state + OpenAPI doc.
//!
//! Routes mount under `/api`. Swagger UI is served at `/swagger-ui` with
//! the raw OpenAPI 3 JSON at `/api-docs/openapi.json`.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod handlers;
pub mod state;

pub use state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Place Service API",
        version = env!("CARGO_PKG_VERSION"),
        description = "schema.org/Place-aligned identity registry — CRUD, search, matching, merging, audit, privacy."
    ),
    paths(
        handlers::health,
        handlers::create_place,
        handlers::get_place,
        handlers::update_place,
        handlers::delete_place,
        handlers::search_places,
        handlers::match_place,
        handlers::check_duplicates,
        handlers::merge_places,
        handlers::deduplicate,
        handlers::export_place_data,
        handlers::masked_place,
        handlers::audit_for_place,
        handlers::audit_recent,
    ),
    components(schemas(
        crate::api::ApiError,
        crate::models::place::Place,
        crate::models::address::PostalAddress,
        crate::models::geo::GeoCoordinates,
        crate::models::place_type::PlaceType,
        crate::models::identifier::PlaceIdentifier,
        crate::models::identifier::IdentifierType,
        crate::models::amenity::AmenityFeature,
        crate::models::opening_hours::OpeningHoursSpecification,
        crate::models::opening_hours::DayOfWeek,
        crate::models::merge::MergeRequest,
        crate::models::merge::MergeResponse,
        crate::models::merge::MergeRecord,
        crate::validation::ValidationError,
        crate::db::audit::AuditEntry,
        handlers::HealthResponse,
        handlers::SearchResponse,
        handlers::ScoredCandidate,
        handlers::DuplicateCheckResponse,
        handlers::BatchDeduplicationRequest,
        handlers::BatchDeduplicationResponse,
    )),
    tags(
        (name = "health",   description = "Liveness probe"),
        (name = "places",   description = "Place CRUD"),
        (name = "search",   description = "Full-text + fuzzy search"),
        (name = "matching", description = "Match / dedup / merge"),
        (name = "privacy",  description = "Masking + GDPR export"),
        (name = "audit",    description = "Audit log queries"),
    ),
)]
/// utoipa OpenAPI document aggregating every path, schema, and tag.
pub struct ApiDoc;

/// Build the REST router with the given application state.
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/places", post(handlers::create_place))
        .route("/places/search", get(handlers::search_places))
        .route("/places/match", post(handlers::match_place))
        .route("/places/check-duplicates", post(handlers::check_duplicates))
        .route("/places/merge", post(handlers::merge_places))
        .route("/places/deduplicate", post(handlers::deduplicate))
        .route(
            "/places/{id}",
            get(handlers::get_place)
                .put(handlers::update_place)
                .delete(handlers::delete_place),
        )
        .route("/places/{id}/export", get(handlers::export_place_data))
        .route("/places/{id}/masked", get(handlers::masked_place))
        .route("/places/{id}/audit", get(handlers::audit_for_place))
        .route("/audit/recent", get(handlers::audit_recent))
        .with_state(state);

    Router::new()
        .nest("/api", api_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}
