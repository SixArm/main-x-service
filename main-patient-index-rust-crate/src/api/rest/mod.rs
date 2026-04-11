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
        title = "Main Patient Index API",
        version = "0.1.0",
        description = "RESTful API for patient identification, matching, deduplication, and privacy",
        contact(
            name = "MPI Development Team",
            email = "support@example.com"
        )
    ),
    paths(
        handlers::health_check,
        handlers::create_patient,
        handlers::get_patient,
        handlers::update_patient,
        handlers::delete_patient,
        handlers::search_patients,
        handlers::match_patient,
        handlers::check_duplicates,
        handlers::merge_patients,
        handlers::batch_deduplicate,
        handlers::export_patient_data,
        handlers::get_patient_masked,
        handlers::get_patient_audit_logs,
        handlers::get_recent_audit_logs,
        handlers::get_user_audit_logs,
    ),
    components(
        schemas(
            crate::models::Patient,
            crate::models::patient::HumanName,
            crate::models::patient::NameUse,
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
            crate::api::ApiResponse::<crate::models::Patient>,
            crate::api::ApiError,
            handlers::HealthResponse,
            handlers::CreatePatientRequest,
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
        (name = "patients", description = "Patient management endpoints"),
        (name = "search", description = "Patient search endpoints"),
        (name = "matching", description = "Patient matching endpoints"),
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
        // Patient CRUD
        .route("/patients", post(handlers::create_patient))
        .route("/patients/:id", get(handlers::get_patient))
        .route("/patients/:id", put(handlers::update_patient))
        .route("/patients/:id", delete(handlers::delete_patient))
        // Search
        .route("/patients/search", get(handlers::search_patients))
        // Matching
        .route("/patients/match", post(handlers::match_patient))
        // Duplicate detection & deduplication
        .route("/patients/check-duplicates", post(handlers::check_duplicates))
        .route("/patients/merge", post(handlers::merge_patients))
        .route("/patients/deduplicate", post(handlers::batch_deduplicate))
        // Privacy
        .route("/patients/:id/export", get(handlers::export_patient_data))
        .route("/patients/:id/masked", get(handlers::get_patient_masked))
        // Audit
        .route("/patients/:id/audit", get(handlers::get_patient_audit_logs))
        .route("/audit/recent", get(handlers::get_recent_audit_logs))
        .route("/audit/user", get(handlers::get_user_audit_logs))
        .with_state(state);

    Router::new()
        .nest("/api/v1", api_routes)
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
