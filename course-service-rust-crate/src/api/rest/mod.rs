//! REST API surface — Axum router + state + OpenAPI doc.
//!
//! Routes mount under `/api` (matches `spec.md §9` and the front-end's
//! `CourseRepository`). Swagger UI is served at `/swagger-ui` with the
//! raw OpenAPI 3 JSON at `/api-docs/openapi.json`.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use sea_orm::DatabaseConnection;

pub mod handlers;
pub mod state;

pub use state::AppState;

use crate::Result;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Course Service API",
        version = "0.1.0",
        description = "schema.org/Course-aligned identity registry — CRUD, search, matching, merging, audit, privacy."
    ),
    paths(
        handlers::health,
        handlers::create_course,
        handlers::get_course,
        handlers::update_course,
        handlers::delete_course,
        handlers::search_courses,
        handlers::check_duplicates,
        handlers::match_course,
        handlers::merge_courses,
        handlers::deduplicate,
        handlers::list_instances,
        handlers::create_instance,
        handlers::update_instance_handler,
        handlers::delete_instance,
        handlers::masked_course,
        handlers::export_course_data,
        handlers::audit_for_course,
        handlers::audit_recent,
    ),
    components(schemas(
        crate::api::ApiError,
        crate::models::Course,
        crate::models::CourseStatus,
        crate::models::EducationalLevel,
        crate::models::LearningResourceType,
        crate::models::InteractivityType,
        crate::models::CourseLink,
        crate::models::LinkType,
        crate::models::CourseIdentifier,
        crate::models::IdentifierType,
        crate::models::CourseInstance,
        crate::models::CourseMode,
        crate::models::CourseInstanceStatus,
        crate::models::Schedule,
        crate::models::course_instance::Session,
        crate::models::EducationalCredential,
        crate::models::CredentialCategory,
        crate::models::Syllabus,
        crate::models::Provider,
        crate::models::ProviderKind,
        crate::models::MergeRequest,
        crate::models::MergeResponse,
        crate::models::MergeRecord,
        crate::models::MergeStatus,
        crate::models::BatchDeduplicationRequest,
        crate::models::BatchDeduplicationResponse,
        crate::models::ReviewQueueItem,
        crate::models::ReviewStatus,
        crate::matching::MatchBreakdown,
        crate::validation::ValidationError,
        crate::db::audit::AuditEntry,
        handlers::HealthResponse,
        handlers::SearchQuery,
        handlers::SearchResponse,
        handlers::ScoredCandidate,
        handlers::AuditQuery,
    )),
    tags(
        (name = "health",     description = "Liveness probe"),
        (name = "courses",    description = "Course CRUD"),
        (name = "instances",  description = "CourseInstance sub-resource"),
        (name = "search",     description = "Full-text + fuzzy search"),
        (name = "matching",   description = "Match / dedup / merge"),
        (name = "privacy",    description = "Masking + GDPR export"),
        (name = "audit",      description = "Audit log queries"),
    ),
)]
pub struct ApiDoc;

/// Build the REST router with the given application state.
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/health", get(handlers::health))
        // Course list + create.
        .route(
            "/courses",
            get(handlers::not_implemented).post(handlers::create_course),
        )
        // Literal segments declared before the `/:id` catch-all.
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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
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
