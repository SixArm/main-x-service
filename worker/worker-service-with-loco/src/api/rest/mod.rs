//! REST API wiring: Axum router, OpenAPI document, and server bootstrap.
//!
//! [`create_router`] mounts every handler under `/api`, exposes
//! `/metrics.prom` and the Swagger UI, and applies a permissive CORS layer;
//! [`serve`] binds the configured host/port and runs it. [`ApiDoc`] is the
//! utoipa-generated OpenAPI spec. Handlers live in [`handlers`], shared state
//! in [`state`].

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Workforce-assessment endpoints (aptitude / personality / psychometric
/// / selection tests) and the derived assessment profile.
pub mod assessments;
/// Bearer-token authentication extractor + `whoami` endpoint + blanket
/// auth-enforcement middleware (default-off, `WORKER_REQUIRE_AUTH`).
pub mod auth;
/// REST endpoint handler implementations.
pub mod handlers;
/// Cross-service entity-link write-side endpoints (`same_identity`).
pub mod links;
/// Loco route-group registration (`workers_routes`, `fhir_routes`, `metrics_routes`).
pub mod routes;
/// Shared [`AppState`] carried by every handler.
pub mod state;
/// Header-based API versioning (`Accepts-version`) middleware + helper.
pub mod version;

pub use state::AppState;

/// utoipa `OpenAPI` document: aggregates every handler path and schema into the
/// spec served at `/api-docs/openapi.json` and rendered by the Swagger UI.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Worker Service API",
        version = "0.1.0",
        description = "RESTful API for worker identification, matching, deduplication, and privacy",
        contact(
            name = "Worker Service Development Team",
            email = "support@example.com"
        )
    ),
    paths(
        handlers::health_check,
        handlers::metrics_prom,
        auth::whoami,
        handlers::create_worker,
        handlers::list_workers,
        handlers::get_worker,
        handlers::update_worker,
        handlers::delete_worker,
        handlers::search_workers,
        handlers::match_worker,
        handlers::check_duplicates,
        handlers::merge_workers,
        handlers::batch_deduplicate,
        handlers::get_review_queue,
        handlers::review_decision,
        assessments::create_assessment,
        assessments::list_assessments,
        assessments::get_assessment,
        assessments::update_assessment,
        assessments::delete_assessment,
        assessments::assessment_profile,
        handlers::export_worker_data,
        handlers::get_worker_masked,
        handlers::get_worker_audit_logs,
        handlers::get_recent_audit_logs,
        handlers::get_user_audit_logs,
    ),
    components(
        schemas(
            crate::models::Worker,
            crate::models::worker::HumanName,
            crate::models::worker::NameUse,
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
            crate::models::ReviewDecision,
            crate::models::ReviewDecisionRequest,
            crate::models::ReviewQueueListResponse,
            crate::models::Consent,
            crate::models::ConsentType,
            crate::models::ConsentStatus,
            crate::models::Assessment,
            crate::models::AssessmentCategory,
            crate::models::AssessmentScale,
            crate::models::AssessmentStatus,
            crate::models::AssessmentResult,
            crate::models::ScoreBand,
            crate::api::ApiResponse::<crate::models::Worker>,
            crate::api::ApiError,
            handlers::HealthResponse,
            handlers::CreateWorkerRequest,
            handlers::ListQuery,
            handlers::ListResponse,
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
        (name = "auth", description = "Bearer-token verification endpoints"),
        (name = "workers", description = "Worker management endpoints"),
        (name = "search", description = "Worker search endpoints"),
        (name = "matching", description = "Worker matcher endpoints"),
        (name = "deduplication", description = "Duplicate detection, review, and merge endpoints"),
        (name = "assessments", description = "Aptitude, personality, psychometric, and selection tests"),
        (name = "privacy", description = "Data masking, export, and consent endpoints"),
        (name = "audit", description = "Audit log query endpoints"),
    )
)]
pub struct ApiDoc;

/// Builds the full Axum [`Router`]: API routes nested under `/api`, the
/// `/metrics.prom` scrape endpoint, the Swagger UI, and a permissive CORS
/// layer. Shared [`AppState`] is injected into the API routes. The blanket
/// auth-enforcement middleware ([`auth::apply_enforcement`]) is layered
/// unconditionally; the `WORKER_REQUIRE_AUTH` flag (read here, at
/// construction — restart to change; default off) is the only switch.
pub fn create_router(state: AppState) -> Router {
    // FHIR R5 `/fhir/Practitioner` surface + `/fhir/metadata`, mirroring the
    // loco mount in `crate::app::App::routes` (built from [`fhir_routes`]) so
    // the integration-test router matches production.
    let fhir_routes = {
        use crate::api::fhir::handlers as fhir;
        Router::new()
            .route("/metadata", get(fhir::metadata))
            .route(
                "/Practitioner",
                get(fhir::search_fhir_workers).post(fhir::create_fhir_worker),
            )
            .route(
                "/Practitioner/{id}",
                get(fhir::get_fhir_worker)
                    .put(fhir::update_fhir_worker)
                    .delete(fhir::delete_fhir_worker),
            )
            .with_state(state.clone())
    };

    let api_routes = Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        // Auth — echo verified bearer-token claims
        .route("/whoami", get(auth::whoami))
        // Worker CRUD (+ the plain database-backed collection list, GET
        // /workers — deliberately not Tantivy-backed; see
        // `handlers::list_workers`'s doc comment)
        .route(
            "/workers",
            post(handlers::create_worker).get(handlers::list_workers),
        )
        .route("/workers/{id}", get(handlers::get_worker))
        .route("/workers/{id}", put(handlers::update_worker))
        .route("/workers/{id}", delete(handlers::delete_worker))
        // Search
        .route("/workers/search", get(handlers::search_workers))
        // Matching
        .route("/workers/match", post(handlers::match_worker))
        // Duplicate detection & deduplication
        .route(
            "/workers/check-duplicates",
            post(handlers::check_duplicates),
        )
        .route("/workers/merge", post(handlers::merge_workers))
        .route("/workers/deduplicate", post(handlers::batch_deduplicate))
        .route("/workers/review-queue", get(handlers::get_review_queue))
        .route(
            "/workers/review-queue/{id}/decision",
            post(handlers::review_decision),
        )
        // Cross-service entity links (§4.1): the aggregator's bulk
        // reconciliation pull (static — must precede the `{id}` routes),
        // then a worker's outbound-edge create/list/withdraw.
        .route("/workers/links", get(links::bulk_links))
        .route(
            "/workers/{id}/links",
            post(links::create_link).get(links::list_links),
        )
        .route("/workers/{id}/links/{link_id}", delete(links::delete_link))
        // Workforce assessments (aptitude / personality / psychometric /
        // selection) + the derived profile.
        .route(
            "/workers/{id}/assessments",
            post(assessments::create_assessment).get(assessments::list_assessments),
        )
        .route(
            "/workers/{id}/assessments/{assessment_id}",
            get(assessments::get_assessment)
                .put(assessments::update_assessment)
                .delete(assessments::delete_assessment),
        )
        .route(
            "/workers/{id}/assessment-profile",
            get(assessments::assessment_profile),
        )
        // Privacy
        .route("/workers/{id}/export", get(handlers::export_worker_data))
        .route("/workers/{id}/masked", get(handlers::get_worker_masked))
        // Audit
        .route("/workers/{id}/audit", get(handlers::get_worker_audit_logs))
        .route("/workers/{id}/erase", post(handlers::erase_worker))
        .route(
            "/workers/{id}/audit/disclosures",
            get(handlers::get_worker_disclosures),
        )
        .route("/audit/recent", get(handlers::get_recent_audit_logs))
        .route("/audit/user", get(handlers::get_user_audit_logs))
        .route("/audit/verify", get(handlers::verify_audit_chain))
        .route("/audit/checkpoint", get(handlers::take_audit_checkpoint))
        .route(
            "/audit/checkpoint/verify",
            post(handlers::verify_audit_checkpoint),
        )
        .route("/records/verify", get(handlers::verify_record_integrity))
        // Compliance evidence. Guarded like everything else under `/api`:
        // the SBOM names every dependency version in the running binary,
        // which is what an attacker needs to match a deployment against
        // published advisories.
        .route("/compliance", get(handlers::compliance_identification))
        .route("/compliance/sbom", get(handlers::compliance_sbom))
        .with_state(state);

    let router = Router::new()
        // Mount the JSON API under `/api` and the FHIR surface under `/fhir`
        // (both already carry the shared `AppState`).
        .nest("/api", api_routes)
        .nest("/fhir", fhir_routes)
        // Root-level Prometheus scrape endpoint — outside `/api` so a default
        // scrape config (`metrics_path: /metrics.prom`) finds it.
        .route("/metrics.prom", get(handlers::metrics_prom))
        // Swagger UI + the served OpenAPI JSON built from `ApiDoc`.
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    // Blanket auth enforcement (spec §13 T-1b): default-off, gated by
    // `WORKER_REQUIRE_AUTH` read at construction, with the ABAC policy
    // (`WORKER_ABAC_POLICY`/`_FILE`, else the built-in default) captured
    // alongside it. Layered beneath CORS so preflight `OPTIONS` requests
    // are answered before enforcement runs.
    auth::apply_enforcement(router, auth::require_auth_from_env())
        // Header-based API versioning (`Accepts-version`): negotiates the
        // version for `/api/*` and stamps it on the response
        // (`agents/share/api-versioning.md`).
        .layer(axum::middleware::from_fn(version::require_version_mw))
        // Permissive CORS so browser-based operator UIs on other origins can
        // call the API; tighten for production deployments.
        .layer(CorsLayer::permissive())
}

/// Native loco controller routes (idiomatic path): the `/api` surface
/// as a loco `Routes`; handlers extract `AppState` from the `AppContext`
/// shared store via `FromRef`. `create_router` is retained for the
/// integration tests. The root `/metrics.prom` route is [`metrics_routes`].
#[must_use]
pub fn workers_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, delete, get, post};
    Routes::new()
        .prefix("/api")
        .add("/health", get(handlers::health_check))
        .add("/whoami", get(auth::whoami))
        // Cross-service entity links (§4.1): bulk reconciliation pull
        // (static, before `{id}`), then per-worker create/list/withdraw.
        .add("/workers/links", get(links::bulk_links))
        .add(
            "/workers/{id}/links",
            post(links::create_link).get(links::list_links),
        )
        .add("/workers/{id}/links/{link_id}", delete(links::delete_link))
        // Workforce assessments + the derived profile.
        .add(
            "/workers/{id}/assessments",
            post(assessments::create_assessment).get(assessments::list_assessments),
        )
        .add(
            "/workers/{id}/assessments/{assessment_id}",
            get(assessments::get_assessment)
                .put(assessments::update_assessment)
                .delete(assessments::delete_assessment),
        )
        .add(
            "/workers/{id}/assessment-profile",
            get(assessments::assessment_profile),
        )
        .add(
            "/workers",
            post(handlers::create_worker).get(handlers::list_workers),
        )
        .add(
            "/workers/{id}",
            get(handlers::get_worker)
                .put(handlers::update_worker)
                .delete(handlers::delete_worker),
        )
        .add("/workers/search", get(handlers::search_workers))
        .add("/workers/match", post(handlers::match_worker))
        .add(
            "/workers/check-duplicates",
            post(handlers::check_duplicates),
        )
        .add("/workers/merge", post(handlers::merge_workers))
        .add("/workers/deduplicate", post(handlers::batch_deduplicate))
        .add("/workers/review-queue", get(handlers::get_review_queue))
        .add(
            "/workers/review-queue/{id}/decision",
            post(handlers::review_decision),
        )
        .add("/workers/{id}/export", get(handlers::export_worker_data))
        .add("/workers/{id}/masked", get(handlers::get_worker_masked))
        .add("/workers/{id}/audit", get(handlers::get_worker_audit_logs))
        .add("/workers/{id}/erase", post(handlers::erase_worker))
        .add(
            "/workers/{id}/audit/disclosures",
            get(handlers::get_worker_disclosures),
        )
        .add("/audit/recent", get(handlers::get_recent_audit_logs))
        .add("/audit/user", get(handlers::get_user_audit_logs))
        .add("/audit/verify", get(handlers::verify_audit_chain))
        .add("/audit/checkpoint", get(handlers::take_audit_checkpoint))
        .add(
            "/audit/checkpoint/verify",
            post(handlers::verify_audit_checkpoint),
        )
        .add("/records/verify", get(handlers::verify_record_integrity))
        .add("/compliance", get(handlers::compliance_identification))
        .add("/compliance/sbom", get(handlers::compliance_sbom))
}

/// Root-level Prometheus scrape route (`GET /metrics.prom`).
#[must_use]
pub fn metrics_routes() -> loco_rs::controller::Routes {
    use loco_rs::prelude::{Routes, get};
    Routes::new().add("/metrics.prom", get(handlers::metrics_prom))
}

/// HL7 FHIR R5 `/fhir/Practitioner` controller routes + `/fhir/metadata`.
///
/// Mounts the handlers in [`crate::api::fhir::handlers`] as a loco `Routes`
/// group. The handlers extract [`AppState`] from the `AppContext` shared
/// store via `FromRef`, exactly like the REST surface, so they reuse the same
/// repository, search engine, and matcher. Wired into the router by
/// [`crate::app::App::routes`]. The literal `/metadata` is added before the
/// `/Practitioner/{id}` capture. `resourceType` is the standard FHIR R5
/// `Practitioner` (fhir.md §3), replacing the prototype's `Worker`.
#[must_use]
pub fn fhir_routes() -> loco_rs::controller::Routes {
    use crate::api::fhir::handlers;
    use loco_rs::prelude::{Routes, get};
    Routes::new()
        .prefix("/fhir")
        .add("/metadata", get(handlers::metadata))
        .add(
            "/Practitioner",
            get(handlers::search_fhir_workers).post(handlers::create_fhir_worker),
        )
        .add(
            "/Practitioner/{id}",
            get(handlers::get_fhir_worker)
                .put(handlers::update_fhir_worker)
                .delete(handlers::delete_fhir_worker),
        )
}
