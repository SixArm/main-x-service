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
        title = "Main Event Service API",
        version = "0.1.0",
        description = "RESTful API for event identification, matching, deduplication, and privacy",
        contact(
            name = "MPI Development Team",
            email = "support@example.com"
        )
    ),
    paths(
        handlers::health_check,
        handlers::create_event,
        handlers::get_event,
        handlers::update_event,
        handlers::delete_event,
        handlers::search_events,
        handlers::match_event,
        handlers::check_duplicates,
        handlers::merge_events,
        handlers::batch_deduplicate,
        handlers::export_event_data,
        handlers::get_event_masked,
        handlers::get_event_audit_logs,
        handlers::get_recent_audit_logs,
        handlers::get_user_audit_logs,
    ),
    components(
        schemas(
            crate::models::Event,
            crate::models::event::HumanName,
            crate::models::event::NameUse,
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
            crate::api::ApiResponse::<crate::models::Event>,
            crate::api::ApiError,
            handlers::HealthResponse,
            handlers::CreateEventRequest,
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
        (name = "events", description = "Event management endpoints"),
        (name = "search", description = "Event search endpoints"),
        (name = "matching", description = "Event matching endpoints"),
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
        // Event CRUD
        .route("/events", post(handlers::create_event))
        .route("/events/:id", get(handlers::get_event))
        .route("/events/:id", put(handlers::update_event))
        .route("/events/:id", delete(handlers::delete_event))
        // Search
        .route("/events/search", get(handlers::search_events))
        // Matching
        .route("/events/match", post(handlers::match_event))
        // Duplicate detection & deduplication
        .route("/events/check-duplicates", post(handlers::check_duplicates))
        .route("/events/merge", post(handlers::merge_events))
        .route("/events/deduplicate", post(handlers::batch_deduplicate))
        // Privacy
        .route("/events/:id/export", get(handlers::export_event_data))
        .route("/events/:id/masked", get(handlers::get_event_masked))
        // Audit
        .route("/events/:id/audit", get(handlers::get_event_audit_logs))
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
