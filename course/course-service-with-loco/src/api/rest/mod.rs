//! REST API surface — Axum router + state + `OpenAPI` doc.
//!
//! Routes mount under `/api` (matches `spec.md §9` and the front-end's
//! `CourseRepository`). Swagger UI is served at `/swagger-ui` with the
//! raw `OpenAPI` 3 JSON at `/api-docs/openapi.json`.

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
/// utoipa `OpenAPI` document aggregating every path, schema, and tag for
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

/// DB-free pins for the metrics route registration and the `OpenAPI` doc.
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

    /// A live `GET /metrics.prom` over the actual Axum router returns
    /// `200` with the Prometheus text-exposition content type, and the
    /// rendered body reflects counter increments performed on the live
    /// request path. This exercises the route end-to-end (handler +
    /// content-type header + registry render) without a database: the
    /// metrics handler is stateless, so a router carrying just the
    /// root-mounted `/metrics.prom` route is sufficient.
    #[tokio::test]
    async fn metrics_endpoint_serves_text_exposition_and_reflects_increments() {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode, header::CONTENT_TYPE as CT_HEADER},
        };
        use tower::ServiceExt;

        // The metrics handler reaches the process-wide registry directly,
        // so no AppState is needed — mirror the root mount in `create_router`.
        let app = Router::new().route("/metrics.prom", get(handlers::metrics_prom));

        let scrape = |app: Router| async move {
            let req = Request::builder()
                .method(Method::GET)
                .uri("/metrics.prom")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let status = resp.status();
            let content_type = resp
                .headers()
                .get(CT_HEADER)
                .map(|v| v.to_str().unwrap().to_owned());
            let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap();
            (
                status,
                content_type,
                String::from_utf8(bytes.to_vec()).unwrap(),
            )
        };

        // Record the pre-increment value of `course_merged_total`, then
        // increment it on the live registry the handler reads from.
        let before = crate::metrics::Metrics::global().course_merged_total.get();
        crate::metrics::Metrics::global().course_merged_total.inc();

        let (status, content_type, body) = scrape(app).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            content_type.as_deref(),
            Some(crate::metrics::CONTENT_TYPE),
            "metrics endpoint must advertise the 0.0.4 text-exposition content type",
        );
        assert!(
            content_type.unwrap().contains("text/plain"),
            "content type must be text/plain",
        );
        assert!(
            body.contains("course_created_total"),
            "scrape must include the declared plain counters; got: {body}",
        );

        // The increment performed on the live registry is reflected in the
        // scraped body: the sample value is at least `before + 1`.
        let line = body
            .lines()
            .find(|l| l.starts_with("course_merged_total "))
            .expect("course_merged_total sample line present in scrape");
        let value: f64 = line
            .rsplit(' ')
            .next()
            .and_then(|v| v.parse().ok())
            .expect("counter sample has a numeric value");
        assert!(
            value >= before + 1.0,
            "expected course_merged_total >= {} after inc(), got {value}",
            before + 1.0,
        );
    }

    /// `ApiDoc::openapi()` must return without panicking. This is a
    /// regression guard for the self-referential `Syllabus` schema
    /// (`sub_sections: Vec<Syllabus>`), which previously drove utoipa
    /// into unbounded recursion and overflowed the stack while building
    /// the document. The fix is `#[schema(no_recursion)]` on that field;
    /// if it is removed, this test aborts with a stack overflow.
    ///
    /// Building the full document is DB-free, so it also lets us assert
    /// the path surface directly — including the root-mounted
    /// `/metrics.prom` endpoint — rather than only inspecting the typed
    /// loco `Routes`.
    #[test]
    fn openapi_doc_builds_with_expected_paths() {
        let doc = ApiDoc::openapi();
        let paths = &doc.paths.paths;

        // Prometheus metrics endpoint, mounted at the application root.
        assert!(
            paths.contains_key("/metrics.prom"),
            "OpenAPI doc is missing the /metrics.prom path; got {:?}",
            paths.keys().collect::<Vec<_>>()
        );

        // A representative sample of the `/api` surface, to prove the
        // document is fully populated (not merely non-empty).
        for expected in [
            "/api/health",
            "/api/courses",
            "/api/courses/{id}",
            "/api/courses/merge",
            "/api/courses/{id}/instances",
        ] {
            assert!(
                paths.contains_key(expected),
                "OpenAPI doc is missing the {expected} path; got {:?}",
                paths.keys().collect::<Vec<_>>()
            );
        }
    }
}
