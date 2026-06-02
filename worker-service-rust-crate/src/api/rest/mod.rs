//! RESTful API implementation with Axum

use axum::{
    Router,
    routing::{get, post, put, delete},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod handlers;
pub mod routes;
pub mod state;

pub use state::AppState;

use crate::Result;

/// API documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Worker Service API",
        version = "0.1.0",
        description = "RESTful API for worker identification, matching, deduplication, and privacy",
        contact(
            name = "MPI Development Team",
            email = "support@example.com"
        )
    ),
    paths(
        handlers::health_check,
        handlers::metrics_prom,
        handlers::create_worker,
        handlers::get_worker,
        handlers::update_worker,
        handlers::delete_worker,
        handlers::search_workers,
        handlers::match_worker,
        handlers::check_duplicates,
        handlers::merge_workers,
        handlers::batch_deduplicate,
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
            crate::models::Consent,
            crate::models::ConsentType,
            crate::models::ConsentStatus,
            crate::api::ApiResponse::<crate::models::Worker>,
            crate::api::ApiError,
            handlers::HealthResponse,
            handlers::CreateWorkerRequest,
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
        (name = "workers", description = "Worker management endpoints"),
        (name = "search", description = "Worker search endpoints"),
        (name = "matching", description = "Worker matcher endpoints"),
        (name = "deduplication", description = "Duplicate detection, review, and merge endpoints"),
        (name = "privacy", description = "Data masking, export, and consent endpoints"),
        (name = "audit", description = "Audit log query endpoints"),
    )
)]
pub struct ApiDoc;

/// Create the REST API router with application state
pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        // Worker CRUD
        .route("/workers", post(handlers::create_worker))
        .route("/workers/:id", get(handlers::get_worker))
        .route("/workers/:id", put(handlers::update_worker))
        .route("/workers/:id", delete(handlers::delete_worker))
        // Search
        .route("/workers/search", get(handlers::search_workers))
        // Matching
        .route("/workers/match", post(handlers::match_worker))
        // Duplicate detection & deduplication
        .route("/workers/check-duplicates", post(handlers::check_duplicates))
        .route("/workers/merge", post(handlers::merge_workers))
        .route("/workers/deduplicate", post(handlers::batch_deduplicate))
        // Privacy
        .route("/workers/:id/export", get(handlers::export_worker_data))
        .route("/workers/:id/masked", get(handlers::get_worker_masked))
        // Audit
        .route("/workers/:id/audit", get(handlers::get_worker_audit_logs))
        .route("/audit/recent", get(handlers::get_recent_audit_logs))
        .route("/audit/user", get(handlers::get_user_audit_logs))
        .with_state(state);

    Router::new()
        .nest("/api/v1", api_routes)
        .route("/metrics.prom", get(handlers::metrics_prom))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
}

/// Start the REST API server
pub async fn serve(state: AppState) -> Result<()> {
    let app = create_router(state.clone());
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| crate::Error::Api(e.to_string()))?;

    tracing::info!("REST API server listening on {}", addr);
    tracing::info!("Swagger UI available at http://{}/swagger-ui", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::Error::Api(e.to_string()))?;

    Ok(())
}
