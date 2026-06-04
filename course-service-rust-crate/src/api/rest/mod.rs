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
        .route("/courses/match", post(handlers::match_course))
        .route(
            "/courses/check-duplicates",
            post(handlers::check_duplicates),
        )
        .route("/courses/merge", post(handlers::merge_courses))
        .route("/courses/deduplicate", post(handlers::deduplicate))
        // Course CRUD by id.
        .route(
            "/courses/:id",
            get(handlers::get_course)
                .put(handlers::update_course)
                .delete(handlers::delete_course),
        )
        // CourseInstance sub-resource (T-8, FR-10..FR-13).
        .route(
            "/courses/:id/instances",
            get(handlers::list_instances).post(handlers::create_instance),
        )
        .route(
            "/courses/:id/instances/:instance_id",
            get(handlers::not_implemented)
                .put(handlers::update_instance_handler)
                .delete(handlers::delete_instance),
        )
        // Privacy / GDPR (T-10, FR-15 + FR-16).
        .route("/courses/:id/export", get(handlers::export_course_data))
        .route("/courses/:id/masked", get(handlers::masked_course))
        // Audit (T-9, FR-14 + FR-17).
        .route("/courses/:id/audit", get(handlers::audit_for_course))
        .route("/audit/recent", get(handlers::audit_recent))
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
