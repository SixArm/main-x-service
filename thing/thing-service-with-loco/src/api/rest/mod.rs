//! REST API surface — Axum router + state + `OpenAPI` doc.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Bearer-token authentication extractor + `whoami` endpoint.
pub mod auth;
pub mod handlers;
pub mod state;
/// Header-based API versioning (`Accepts-version`) middleware + helper.
pub mod version;

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
        handlers::metrics_prom,
        auth::whoami,
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
        (name = "observability", description = "Prometheus metrics endpoint"),
        (name = "auth",     description = "Bearer-token verification (PASETO v4.public)"),
        (name = "things",   description = "Thing CRUD"),
        (name = "search",   description = "Full-text + fuzzy search"),
        (name = "matching", description = "Match / dedup / merge"),
        (name = "privacy",  description = "Masking + GDPR export"),
        (name = "audit",    description = "Audit log queries"),
    ),
)]
/// utoipa `OpenAPI` document aggregating every path, schema, and tag.
pub struct ApiDoc;

/// Build the REST router with the given application state.
///
/// The blanket-enforcement middleware ([`auth::require_auth_mw`]) is
/// layered unconditionally (inside CORS, so preflight requests still
/// pass); it is a near-noop unless `THING_REQUIRE_AUTH` was truthy at
/// [`AppState`] construction.
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/whoami", get(auth::whoami))
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
        .with_state(state.clone());

    Router::new()
        .nest("/api", api_routes)
        .route("/metrics.prom", get(handlers::metrics_prom))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(axum::middleware::from_fn_with_state(
            state,
            auth::require_auth_mw,
        ))
        .layer(axum::middleware::from_fn(version::require_version_mw))
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
        .add("/whoami", get(auth::whoami))
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

/// Root-level Prometheus scrape route (`GET /metrics.prom`).
///
/// Mounted at the root rather than under `/api` so a default Prometheus
/// scrape config (`metrics_path: /metrics.prom`) finds it.
#[must_use]
pub fn metrics_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get};
    Routes::new().add("/metrics.prom", get(handlers::metrics_prom))
}

#[cfg(test)]
mod tests {
    //! DB-free tests for `OpenAPI` wiring of the Prometheus scrape route.
    use super::*;

    /// The `/metrics.prom` path is registered in the `OpenAPI` document at
    /// the root (not under `/api`), so Swagger/scrapers can discover it.
    #[test]
    fn openapi_exposes_metrics_prom_path() {
        let doc = ApiDoc::openapi();
        assert!(
            doc.paths.paths.contains_key("/metrics.prom"),
            "OpenAPI paths missing /metrics.prom: {:?}",
            doc.paths.paths.keys().collect::<Vec<_>>()
        );
    }

    /// The root-level loco route group binds exactly the `/metrics.prom`
    /// scrape path.
    #[test]
    fn metrics_routes_binds_metrics_prom() {
        let routes = metrics_routes();
        assert!(
            routes.handlers.iter().any(|h| h.uri == "/metrics.prom"),
            "metrics_routes missing /metrics.prom binding"
        );
    }
}
