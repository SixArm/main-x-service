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
    routing::{delete, get, post, put},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Bearer-token authentication extractor, `whoami` endpoint, and the
/// blanket `/api/*` enforcement middleware (`PERSON_REQUIRE_AUTH`).
pub mod auth;
/// REST endpoint handler functions.
pub mod handlers;
/// Route grouping helpers.
pub mod routes;
/// Shared [`AppState`] definition.
pub mod state;
/// Header-based API versioning (`Accepts-version`) middleware + helper.
pub mod version;

pub use state::AppState;

/// The `OpenAPI` 3 document: endpoint paths, schemas, and tags.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Person Service API",
        version = "0.1.0",
        description = "RESTful API for person identification, matching, deduplication, and privacy",
        contact(
            name = "Person Service Development Team",
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
/// `/api`, exposes `/metrics.prom` and the Swagger UI, and applies the
/// blanket-auth-enforcement middleware (default-off, gated by
/// `PERSON_REQUIRE_AUTH` — snapshotted here, so changing the env var
/// requires a restart) and a permissive CORS layer. The given
/// [`AppState`] is moved into the router as shared state.
pub fn create_router(state: AppState) -> Router {
    let enforcement = auth::Enforcement::from_env(state.verifier.clone());
    let api_routes = Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        // Auth — echo verified bearer-token claims
        .route("/whoami", get(auth::whoami))
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
        .route(
            "/persons/check-duplicates",
            post(handlers::check_duplicates),
        )
        .route("/persons/merge", post(handlers::merge_persons))
        .route("/persons/deduplicate", post(handlers::batch_deduplicate))
        // Privacy
        .route("/persons/{id}/export", get(handlers::export_person_data))
        .route("/persons/{id}/masked", get(handlers::get_person_masked))
        // Audit
        .route("/persons/{id}/audit", get(handlers::get_person_audit_logs))
        .route("/audit/recent", get(handlers::get_recent_audit_logs))
        .route("/audit/user", get(handlers::get_user_audit_logs))
        .with_state(state.clone());

    // FHIR R5 surface (`/fhir/Patient{,/{id}}`, `/fhir/Person{,/{id}}`
    // alias, `/fhir/metadata`). Merged onto the outer router below,
    // *before* the auth layer, so `/fhir/*` inherits the same blanket
    // guard as `/api/*` (it is not on the public allow-list).
    let fhir_routes = crate::api::fhir::handlers::fhir_router(state);

    // Mount under `/api`. Documented in AGENTS/restful.md and
    // consumed by `../person-front-end-with-svelte` at `/api/persons`.
    // No service uses a `/api/v1` URL segment anymore; API
    // versioning is negotiated via the `Accepts-version` header.
    Router::new()
        .nest("/api", api_routes)
        .merge(fhir_routes)
        .route("/metrics.prom", get(handlers::metrics_prom))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(axum::middleware::from_fn_with_state(
            enforcement,
            auth::require_auth_middleware,
        ))
        // Header-based API versioning (`Accepts-version`): negotiates the
        // version for `/api/*` and stamps it on the response
        // (`agents/share/api-versioning.md`).
        .layer(axum::middleware::from_fn(version::require_version_mw))
        .layer(CorsLayer::permissive())
}

/// Native loco controller routes (idiomatic path): the `/api` surface as
/// a loco `Routes`; handlers extract `AppState` from the `AppContext`
/// shared store via `FromRef`. `create_router` is retained for the
/// integration tests. The root `/metrics.prom` route is [`metrics_routes`].
#[must_use]
pub fn persons_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get, post};
    Routes::new()
        .prefix("/api")
        .add("/health", get(handlers::health_check))
        .add("/whoami", get(auth::whoami))
        .add("/persons", post(handlers::create_person))
        .add(
            "/persons/{id}",
            get(handlers::get_person)
                .put(handlers::update_person)
                .delete(handlers::delete_person),
        )
        .add("/persons/search", get(handlers::search_persons))
        .add("/persons/match", post(handlers::match_person))
        .add(
            "/persons/check-duplicates",
            post(handlers::check_duplicates),
        )
        .add("/persons/merge", post(handlers::merge_persons))
        .add("/persons/deduplicate", post(handlers::batch_deduplicate))
        .add("/persons/{id}/export", get(handlers::export_person_data))
        .add("/persons/{id}/masked", get(handlers::get_person_masked))
        .add("/persons/{id}/audit", get(handlers::get_person_audit_logs))
        .add("/audit/recent", get(handlers::get_recent_audit_logs))
        .add("/audit/user", get(handlers::get_user_audit_logs))
}

/// Root-level Prometheus scrape route (`GET /metrics.prom`).
#[must_use]
pub fn metrics_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get};
    Routes::new().add("/metrics.prom", get(handlers::metrics_prom))
}
