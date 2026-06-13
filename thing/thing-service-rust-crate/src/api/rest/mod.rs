//! REST API surface — Axum router + state + OpenAPI doc.

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
        title = "Thing Service API",
        version = env!("CARGO_PKG_VERSION"),
        description = "schema.org/Thing-aligned identity registry — CRUD, search, matching, merging, audit, privacy."
    ),
    paths(
        handlers::health,
        handlers::create_thing,
        handlers::get_thing,
        handlers::update_thing,
        handlers::delete_thing,
        handlers::search_things,
        handlers::match_thing,
        handlers::check_duplicates,
        handlers::merge_things,
        handlers::deduplicate,
        handlers::export_thing_data,
        handlers::masked_thing,
        handlers::audit_for_thing,
        handlers::audit_recent,
    ),
    components(schemas(
        crate::api::ApiError,
        crate::models::thing::Thing,
        crate::models::identifier::ThingIdentifier,
        crate::models::identifier::IdentifierType,
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
        (name = "things",   description = "Thing CRUD"),
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
        .route("/things", post(handlers::create_thing))
        .route("/things/search", get(handlers::search_things))
        .route("/things/match", post(handlers::match_thing))
        .route("/things/check-duplicates", post(handlers::check_duplicates))
        .route("/things/merge", post(handlers::merge_things))
        .route("/things/deduplicate", post(handlers::deduplicate))
        .route(
            "/things/{id}",
            get(handlers::get_thing)
                .put(handlers::update_thing)
                .delete(handlers::delete_thing),
        )
        .route("/things/{id}/export", get(handlers::export_thing_data))
        .route("/things/{id}/masked", get(handlers::masked_thing))
        .route("/things/{id}/audit", get(handlers::audit_for_thing))
        .route("/audit/recent", get(handlers::audit_recent))
        .with_state(state);

    Router::new()
        .nest("/api", api_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}

/// Native loco controller routes (idiomatic path). Mirrors
/// [`create_router`]'s `/api` surface as a loco `Routes`; handlers
/// extract `AppState` from the `AppContext` shared store via `FromRef`.
/// `create_router` is retained for the integration tests.
#[must_use]
pub fn things_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get, post};
    Routes::new()
        .prefix("/api")
        .add("/health", get(handlers::health))
        .add("/things", post(handlers::create_thing))
        .add("/things/search", get(handlers::search_things))
        .add("/things/match", post(handlers::match_thing))
        .add("/things/check-duplicates", post(handlers::check_duplicates))
        .add("/things/merge", post(handlers::merge_things))
        .add("/things/deduplicate", post(handlers::deduplicate))
        .add(
            "/things/{id}",
            get(handlers::get_thing)
                .put(handlers::update_thing)
                .delete(handlers::delete_thing),
        )
        .add("/things/{id}/export", get(handlers::export_thing_data))
        .add("/things/{id}/masked", get(handlers::masked_thing))
        .add("/things/{id}/audit", get(handlers::audit_for_thing))
        .add("/audit/recent", get(handlers::audit_recent))
}
