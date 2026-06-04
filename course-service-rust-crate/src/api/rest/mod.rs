//! REST API surface — Axum router + state.
//!
//! Routes mount under `/api` (matches `spec.md §9` and the front-end's
//! `CourseRepository`). FR-1..FR-5 + FR-7 are wired against the real
//! repository / search engine / matcher. The rest still return
//! `501 Not Implemented` until their per-task work in `spec.md §13`
//! lands.

use axum::{
    Router,
    routing::{get, post},
};
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
        // Course list + create.
        .route(
            "/courses",
            get(handlers::not_implemented).post(handlers::create_course),
        )
        // `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`
        // MUST be declared before the `/:id` catch-all so Axum's path-
        // segment router doesn't shadow them.
        .route("/courses/search", get(handlers::search_courses))
        .route(
            "/courses/match",
            get(handlers::not_implemented).post(handlers::not_implemented),
        )
        .route(
            "/courses/check-duplicates",
            post(handlers::check_duplicates),
        )
        .route(
            "/courses/merge",
            get(handlers::not_implemented).post(handlers::not_implemented),
        )
        .route(
            "/courses/deduplicate",
            get(handlers::not_implemented).post(handlers::not_implemented),
        )
        // Course CRUD by id.
        .route(
            "/courses/:id",
            get(handlers::get_course)
                .put(handlers::update_course)
                .delete(handlers::delete_course),
        )
        // CourseInstance sub-resource — pending T-8.
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
        // Privacy / GDPR — pending T-10.
        .route("/courses/:id/export", get(handlers::not_implemented))
        .route("/courses/:id/masked", get(handlers::not_implemented))
        // Audit — pending T-9.
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

/// Builder hook for the binary entry point.
pub fn build_state(
    db: DatabaseConnection,
    search_engine: crate::search::SearchEngine,
    matcher: crate::matching::CourseMatcher,
    config: crate::config::Config,
) -> AppState {
    AppState::new(db, search_engine, matcher, config)
}
