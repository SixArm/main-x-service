//! Axum REST API: OpenAPI doc, router wiring, and server bootstrap.
//!
//! [`ApiDoc`](crate::api::rest::ApiDoc) is the utoipa-generated OpenAPI 3 document (served at
//! `/api-docs/openapi.json` and rendered by Swagger UI at `/swagger-ui`).
//! [`create_router`](crate::api::rest::create_router) maps every endpoint onto a handler in [`handlers`](crate::api::rest::handlers)
//! and nests them under `/api`, plus the Prometheus `/metrics.prom`
//! scrape path and a permissive CORS layer. [`serve`](crate::api::rest::serve) binds the
//! configured host/port and runs the server. [`AppState`](crate::api::rest::AppState) (re-exported
//! from [`state`](crate::api::rest::state)) is the shared state injected into each handler.

use axum::{
    Router,
    routing::{get, post, put, delete},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// REST endpoint handler functions.
pub mod handlers;
/// Route grouping helpers.
pub mod routes;
/// Shared [`AppState`] definition.
pub mod state;

pub use state::AppState;

/// The OpenAPI 3 document: endpoint paths, schemas, and tags.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Person Service API",
        version = "0.1.0",
        description = "RESTful API for person identification, matching, deduplication, and privacy",
        contact(
            name = "MPI Development Team",
            email = "support@example.com"
        )
    ),
    paths(
        handlers::health_check,
        handlers::metrics_prom,
        handlers::create_person,
        handlers::get_person,
        handlers::update_person,
        handlers::delete_person,
        handlers::search_persons,
        handlers::match_person,
        handlers::check_duplicates,
        handlers::merge_persons,
        handlers::batch_deduplicate,
        handlers::export_person_data,
        handlers::get_person_masked,
        handlers::get_person_audit_logs,
        handlers::get_recent_audit_logs,
        handlers::get_user_audit_logs,
    ),
    components(
        schemas(
            crate::models::Person,
            crate::models::person::HumanName,
            crate::models::person::NameUse,
            crate::models::Organization,
            crate::models::Identifier,
            crate::models::identifier::IdentifierType,
            crate::models::identifier::IdentifierUse,
            crate::models::IdentityDocument,
            crate::models::DocumentType,
            crate::models::EmergencyContact,
            crate::models::MergeRequest,
            crate::models::MergeResponse,
            crate::models::MergeRecord,
            crate::models::MergeStatus,
            crate::models::BatchDeduplicationRequest,
            crate::models::BatchDeduplicationResponse,
            crate::models::ReviewQueueItem,
            crate::models::ReviewStatus,
            crate::models::Consent,
            crate::models::ConsentType,
            crate::models::ConsentStatus,
            crate::api::ApiResponse::<crate::models::Person>,
            crate::api::ApiError,
            handlers::HealthResponse,
            handlers::CreatePersonRequest,
            handlers::SearchQuery,
            handlers::SearchResponse,
            handlers::MatchRequest,
            handlers::MatchResponse,
            handlers::MatchResultsResponse,
            handlers::DuplicateCheckResponse,
            handlers::AuditLogQuery,
            handlers::UserAuditLogQuery,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoint"),
        (name = "observability", description = "Prometheus metrics endpoint"),
        (name = "persons", description = "Person management endpoints"),
        (name = "search", description = "Person search endpoints"),
        (name = "matching", description = "Person matcher endpoints"),
        (name = "deduplication", description = "Duplicate detection, review, and merge endpoints"),
        (name = "privacy", description = "Data masking, export, and consent endpoints"),
        (name = "audit", description = "Audit log query endpoints"),
    )
)]
pub struct ApiDoc;

/// Build the fully-wired Axum [`Router`] for the service.
///
/// Mounts the entity/search/match/merge/privacy/audit routes under
/// `/api`, exposes `/metrics.prom` and the Swagger UI, and applies a
/// permissive CORS layer. The given [`AppState`] is moved into the
/// router as shared state.
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        // Person CRUD
        .route("/persons", post(handlers::create_person))
        .route("/persons/{id}", get(handlers::get_person))
        .route("/persons/{id}", put(handlers::update_person))
        .route("/persons/{id}", delete(handlers::delete_person))
        // Search
        .route("/persons/search", get(handlers::search_persons))
        // Matching
        .route("/persons/match", post(handlers::match_person))
        // Duplicate detection & deduplication
        .route("/persons/check-duplicates", post(handlers::check_duplicates))
        .route("/persons/merge", post(handlers::merge_persons))
        .route("/persons/deduplicate", post(handlers::batch_deduplicate))
        // Privacy
        .route("/persons/{id}/export", get(handlers::export_person_data))
        .route("/persons/{id}/masked", get(handlers::get_person_masked))
        // Audit
        .route("/persons/{id}/audit", get(handlers::get_person_audit_logs))
        .route("/audit/recent", get(handlers::get_recent_audit_logs))
        .route("/audit/user", get(handlers::get_user_audit_logs))
        .with_state(state);

    // Mount under `/api`. Documented in AGENTS/restful.md and
    // consumed by `../person-front-end-with-svelte` at `/api/persons`.
    // The Event service uses `/api/v1`; Person does not.
    Router::new()
        .nest("/api", api_routes)
        .route("/metrics.prom", get(handlers::metrics_prom))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}
