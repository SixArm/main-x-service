//! REST API surface — Axum router + state.
//!
//! Routes are mounted under `/api` (matches the spec.md §9 surface
//! used by the front-end's `CourseRepository`). The handler set is a
//! STUB: every endpoint returns `501 Not Implemented` until the
//! per-route work in `spec.md §13` lands.

use axum::{Router, routing::get};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use sea_orm::DatabaseConnection;

pub mod handlers;
pub mod state;

pub use state::AppState;

use crate::Result;

/// Build the REST router with the given application state.
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        // Course CRUD — stubs return 501 Not Implemented.
        .route("/courses", get(handlers::not_implemented).post(handlers::not_implemented))
        .route(
            "/courses/:id",
            get(handlers::not_implemented)
                .put(handlers::not_implemented)
                .delete(handlers::not_implemented),
        )
        .route("/courses/search", get(handlers::not_implemented))
        .route("/courses/match", get(handlers::not_implemented).post(handlers::not_implemented))
        .route("/courses/check-duplicates", get(handlers::not_implemented).post(handlers::not_implemented))
        .route("/courses/merge", get(handlers::not_implemented).post(handlers::not_implemented))
        .route("/courses/deduplicate", get(handlers::not_implemented).post(handlers::not_implemented))
        // CourseInstance sub-resource.
        .route(
            "/courses/:id/instances",
            get(handlers::not_implemented).post(handlers::not_implemented),
        )
        .route(
            "/courses/:id/instances/:instance_id",
            get(handlers::not_implemented)
                .put(handlers::not_implemented)
                .delete(handlers::not_implemented),
        )
        // Privacy / GDPR.
        .route("/courses/:id/export", get(handlers::not_implemented))
        .route("/courses/:id/masked", get(handlers::not_implemented))
        // Audit.
        .route("/courses/:id/audit", get(handlers::not_implemented))
        .route("/audit/recent", get(handlers::not_implemented))
        .with_state(state);

    Router::new()
        .nest("/api", api_routes)
        .layer(CorsLayer::permissive())
}

/// Start the REST API server.
pub async fn serve(state: AppState) -> Result<()> {
    let app = create_router(state.clone());
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| crate::Error::Api(e.to_string()))?;

    tracing::info!("REST API server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::Error::Api(e.to_string()))?;
    Ok(())
}

/// Builder hook reserved for the future `AppState::new(db, search,
/// matcher, config)` signature. Kept here so the binary can call it
/// without knowing the field-order details.
pub fn build_state(
    db: DatabaseConnection,
    search_engine: crate::search::SearchEngine,
    matcher: crate::matching::CourseMatcher,
    config: crate::config::Config,
) -> AppState {
    AppState::new(db, search_engine, matcher, config)
}

#[allow(unused_imports)]
use Arc as _;
