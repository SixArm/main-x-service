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

pub mod handlers;
pub mod state;

pub use state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Course Service API",
        // Sourced from Cargo.toml at compile time so the OpenAPI
        // info block can't drift away from the crate version.
        version = env!("CARGO_PKG_VERSION"),
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
        handlers::get_instance,
        handlers::update_instance_handler,
        handlers::delete_instance,
        handlers::masked_course,
        handlers::export_course_data,
        handlers::audit_for_course,
        handlers::audit_recent,
        handlers::metrics_prom,
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
        (name = "metrics",    description = "Prometheus metrics"),
    ),
)]
/// utoipa OpenAPI document aggregating every path, schema, and tag for
/// the Course Service REST API. Rendered at `/swagger-ui` and served as
/// JSON at `/api-docs/openapi.json`.
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
        // Literal segments declared before the `/{id}` catch-all.
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
            "/courses/{id}",
            get(handlers::get_course)
                .put(handlers::update_course)
                .delete(handlers::delete_course),
        )
        // CourseInstance sub-resource (T-8, FR-10..FR-13).
        .route(
            "/courses/{id}/instances",
            get(handlers::list_instances).post(handlers::create_instance),
        )
        .route(
            "/courses/{id}/instances/{instance_id}",
            get(handlers::get_instance)
                .put(handlers::update_instance_handler)
                .delete(handlers::delete_instance),
        )
        // Privacy / GDPR (T-10, FR-15 + FR-16).
        .route("/courses/{id}/export", get(handlers::export_course_data))
        .route("/courses/{id}/masked", get(handlers::masked_course))
        // Audit (T-9, FR-14 + FR-17).
        .route("/courses/{id}/audit", get(handlers::audit_for_course))
        .route("/audit/recent", get(handlers::audit_recent))
        .with_state(state);

    Router::new()
        .nest("/api", api_routes)
        // Prometheus metrics at the application root (not under `/api`),
        // alongside the docs. Public — no bearer token needed to scrape.
        .route("/metrics.prom", get(handlers::metrics_prom))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}

/// Native loco controller routes (idiomatic path). Mirrors
/// [`create_router`]'s `/api` surface, but as a loco `Routes` whose
/// handlers extract `AppState` from the `AppContext` shared store via
/// `FromRef`. Registered in `App::routes`; `create_router` is retained
/// for the `tower::oneshot`-based integration tests.
#[must_use]
pub fn courses_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get, post};
    Routes::new()
        .prefix("/api")
        .add("/health", get(handlers::health))
        .add(
            "/courses",
            get(handlers::not_implemented).post(handlers::create_course),
        )
        .add("/courses/search", get(handlers::search_courses))
        .add("/courses/match", post(handlers::match_course))
        .add(
            "/courses/check-duplicates",
            post(handlers::check_duplicates),
        )
        .add("/courses/merge", post(handlers::merge_courses))
        .add("/courses/deduplicate", post(handlers::deduplicate))
        .add(
            "/courses/{id}",
            get(handlers::get_course)
                .put(handlers::update_course)
                .delete(handlers::delete_course),
        )
        .add(
            "/courses/{id}/instances",
            get(handlers::list_instances).post(handlers::create_instance),
        )
        .add(
            "/courses/{id}/instances/{instance_id}",
            get(handlers::get_instance)
                .put(handlers::update_instance_handler)
                .delete(handlers::delete_instance),
        )
        .add("/courses/{id}/export", get(handlers::export_course_data))
        .add("/courses/{id}/masked", get(handlers::masked_course))
        .add("/courses/{id}/audit", get(handlers::audit_for_course))
        .add("/audit/recent", get(handlers::audit_recent))
}

/// Native loco controller route for the Prometheus metrics endpoint,
/// mounted at the application **root** (`/metrics.prom`, not under
/// `/api`) — alongside the Swagger UI. Registered in `App::routes`
/// separately from [`courses_routes`] because that set is prefixed
/// `/api`. Public: scraping needs no bearer token.
#[must_use]
pub fn metrics_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get};
    Routes::new().add("/metrics.prom", get(handlers::metrics_prom))
}

/// DB-free pins for the metrics route registration.
#[cfg(test)]
mod tests {
    use super::*;

    /// The Prometheus metrics endpoint is mounted at the application
    /// **root** (`/metrics.prom`), not under the `/api` prefix that
    /// [`courses_routes`] carries. Asserting against the typed loco
    /// `Routes` (rather than building the full `ApiDoc` document) keeps
    /// the test DB-free and cheap.
    #[test]
    fn metrics_route_is_mounted_at_root() {
        let routes = metrics_routes();
        assert!(
            routes.prefix.is_none(),
            "metrics route must be at root, not prefixed; got {:?}",
            routes.prefix
        );
        assert!(
            routes.handlers.iter().any(|h| h.uri == "/metrics.prom"),
            "missing /metrics.prom handler in metrics_routes()"
        );
    }
}
