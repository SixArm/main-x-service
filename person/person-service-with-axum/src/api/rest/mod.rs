//! Axum REST API: OpenAPI doc and route tables.
//!
//! [`ApiDoc`](crate::api::rest::ApiDoc) is the utoipa-generated OpenAPI 3 document.
//! [`persons_routes`](crate::api::rest::persons_routes) maps every endpoint onto a handler in [`handlers`](crate::api::rest::handlers)
//! under `/api`, and [`metrics_routes`](crate::api::rest::metrics_routes) adds the Prometheus
//! `/metrics.prom` scrape path. `App::routes` mounts both. [`AppState`](crate::api::rest::AppState) (re-exported
//! from [`state`](crate::api::rest::state)) is the shared state injected into each handler.

use utoipa::OpenApi;

/// Bearer-token authentication extractor, `whoami` endpoint, and the
/// blanket `/api/*` enforcement middleware (`PERSON_REQUIRE_AUTH`).
pub mod auth;
/// REST endpoint handler functions.
pub mod handlers;
/// Cross-service entity-link endpoints (`same_identity` write side).
pub mod links;
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
        handlers::list_persons,
        handlers::get_person,
        handlers::update_person,
        handlers::delete_person,
        handlers::search_persons,
        handlers::match_person,
        handlers::check_duplicates,
        handlers::merge_persons,
        handlers::batch_deduplicate,
        handlers::get_review_queue,
        handlers::review_decision,
        handlers::export_person_data,
        handlers::get_person_masked,
        handlers::get_person_audit_logs,
        handlers::get_recent_audit_logs,
        handlers::verify_audit_chain,
        handlers::get_user_audit_logs,
        crate::bulk::handlers::import_person,
        crate::bulk::handlers::export_person,
        crate::bulk::handlers::get_import_job,
        crate::bulk::handlers::get_export_job,
        crate::bulk::handlers::list_bulk_jobs,
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
            crate::models::ReviewDecision,
            crate::models::ReviewDecisionRequest,
            crate::models::ReviewQueueListResponse,
            crate::models::Consent,
            crate::models::ConsentType,
            crate::models::ConsentStatus,
            handlers::HealthResponse,
            handlers::CreatePersonRequest,
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
            crate::bulk::handlers::JobAccepted,
            crate::bulk::handlers::ExportRequest,
            crate::bulk::handlers::BulkJobView,
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
        (name = "bulk", description = "Bulk import/export endpoints"),
    )
)]
pub struct ApiDoc;

/// Native axa controller routes (idiomatic path): the `/api` surface as
/// an axa `Routes`; handlers extract `AppState` from the `AppContext`
/// shared store via `FromRef`. The root `/metrics.prom` route is [`metrics_routes`].
#[must_use]
pub fn persons_routes() -> axa::routes::Routes {
    use axa::prelude::{Routes, delete, get, post};
    Routes::new()
        .prefix("/api")
        .add("/health", get(handlers::health_check))
        .add("/whoami", get(auth::whoami))
        .add("/compliance", get(handlers::compliance_identification))
        .add("/compliance/sbom", get(handlers::compliance_sbom))
        // Cross-service entity links (§4.1): bulk reconciliation pull
        // (static, before `{id}`), then per-person create/list/withdraw.
        .add("/persons/links", get(links::bulk_links))
        .add(
            "/persons/{id}/links",
            post(links::create_link).get(links::list_links),
        )
        .add("/persons/{id}/links/{link_id}", delete(links::delete_link))
        .add(
            "/persons",
            post(handlers::create_person).get(handlers::list_persons),
        )
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
        .add("/persons/review-queue", get(handlers::get_review_queue))
        .add(
            "/persons/review-queue/{id}/decision",
            post(handlers::review_decision),
        )
        // Bulk import/export (agents/share/bulk-import-export.md §4). The
        // static bulk paths precede the `{id}` routes so they are not
        // shadowed. `/persons/import` is a declared destructive POST.
        .add(
            "/persons/import",
            post(crate::bulk::handlers::import_person)
                .layer(axa::middleware::body_limit(crate::bulk::IMPORT_BODY_LIMIT)),
        )
        .add(
            "/persons/export",
            post(crate::bulk::handlers::export_person),
        )
        .add(
            "/persons/import/{id}",
            get(crate::bulk::handlers::get_import_job),
        )
        .add(
            "/persons/export/{id}",
            get(crate::bulk::handlers::get_export_job),
        )
        .add(
            "/persons/bulk-jobs",
            get(crate::bulk::handlers::list_bulk_jobs),
        )
        .add("/persons/{id}/export", get(handlers::export_person_data))
        .add("/persons/{id}/masked", get(handlers::get_person_masked))
        .add("/persons/{id}/audit", get(handlers::get_person_audit_logs))
        .add("/persons/{id}/erase", post(handlers::erase_person))
        .add(
            "/persons/{id}/audit/disclosures",
            get(handlers::get_person_disclosures),
        )
        .add("/audit/recent", get(handlers::get_recent_audit_logs))
        .add("/audit/verify", get(handlers::verify_audit_chain))
        .add("/audit/checkpoint", get(handlers::take_audit_checkpoint))
        .add(
            "/audit/checkpoint/verify",
            post(handlers::verify_audit_checkpoint),
        )
        .add("/records/verify", get(handlers::verify_record_integrity))
        .add("/audit/user", get(handlers::get_user_audit_logs))
}

/// Root-level Prometheus scrape route (`GET /metrics.prom`).
#[must_use]
pub fn metrics_routes() -> axa::routes::Routes {
    use axa::prelude::{Routes, get};
    Routes::new().add("/metrics.prom", get(handlers::metrics_prom))
}
