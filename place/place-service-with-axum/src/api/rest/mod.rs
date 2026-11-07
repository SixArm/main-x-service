//! REST API surface — Axum router + state + `OpenAPI` doc.
//!
//! Routes mount under `/api`. Swagger UI is served at `/swagger-ui` with
//! the raw `OpenAPI` 3 JSON at `/api-docs/openapi.json`.

use utoipa::OpenApi;

/// Bearer-token authentication extractor + `whoami` endpoint + blanket
/// `/api/*` enforcement middleware (default-off, `PLACE_REQUIRE_AUTH`).
pub mod auth;
pub mod handlers;
pub mod state;
/// Header-based API versioning (`Accepts-version`) middleware + helper.
pub mod version;

pub use state::AppState;

/// Registers the `bearer` HTTP security scheme (PASETO `v4.public`
/// bearer tokens) that [`auth::whoami`]'s `security(("bearer" = []))`
/// requirement references.
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("PASETO")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Place Service API",
        version = env!("CARGO_PKG_VERSION"),
        description = "schema.org/Place-aligned identity registry — CRUD, search, matching, merging, audit, privacy."
    ),
    paths(
        handlers::health,
        handlers::metrics_prom,
        auth::whoami,
        handlers::create_place,
        handlers::list_places,
        handlers::get_place,
        handlers::update_place,
        handlers::delete_place,
        handlers::search_places,
        handlers::nearby_places,
        handlers::match_place,
        handlers::check_duplicates,
        handlers::merge_places,
        handlers::deduplicate,
        handlers::get_review_queue,
        handlers::review_decision,
        handlers::export_place_data,
        handlers::masked_place,
        handlers::audit_for_place,
        handlers::audit_recent,
    ),
    components(schemas(
        crate::models::place::Place,
        crate::models::address::PostalAddress,
        crate::models::geo::GeoCoordinates,
        crate::models::place_type::PlaceType,
        crate::models::identifier::PlaceIdentifier,
        crate::models::identifier::IdentifierType,
        crate::models::amenity::AmenityFeature,
        crate::models::opening_hours::OpeningHoursSpecification,
        crate::models::opening_hours::DayOfWeek,
        crate::models::merge::MergeRequest,
        crate::models::merge::MergeResponse,
        crate::models::merge::MergeRecord,
        crate::validation::ValidationError,
        crate::db::audit::AuditEntry,
        handlers::HealthResponse,
        handlers::ListResponse,
        handlers::SearchResponse,
        handlers::ScoredCandidate,
        handlers::DuplicateCheckResponse,
        handlers::BatchDeduplicationRequest,
        handlers::BatchDeduplicationResponse,
        handlers::ReviewQueueItem,
        handlers::ReviewStatus,
        handlers::ReviewDecision,
        handlers::ReviewDecisionRequest,
        handlers::ReviewQueueListResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "health",   description = "Liveness probe"),
        (name = "observability", description = "Prometheus metrics endpoint"),
        (name = "auth",     description = "Bearer-token verification (PASETO v4.public)"),
        (name = "places",   description = "Place CRUD"),
        (name = "search",   description = "Full-text + fuzzy search"),
        (name = "matching", description = "Match / dedup / merge"),
        (name = "privacy",  description = "Masking + GDPR export"),
        (name = "audit",    description = "Audit log queries"),
    ),
)]
/// `utoipa` `OpenAPI` document aggregating every path, schema, and tag.
pub struct ApiDoc;

/// Native axa controller routes (idiomatic path): the `/api` surface as an
/// axa `Routes`; handlers extract `AppState` from the `AppContext` shared
/// store via `FromRef`.
#[must_use]
pub fn places_routes() -> axa::routes::Routes {
    use axa::prelude::{Routes, get, post};
    Routes::new()
        .prefix("/api")
        .add("/health", get(handlers::health))
        .add("/whoami", get(auth::whoami))
        .add(
            "/places",
            post(handlers::create_place).get(handlers::list_places),
        )
        .add("/places/search", get(handlers::search_places))
        .add("/places/nearby", get(handlers::nearby_places))
        .add("/places/match", post(handlers::match_place))
        .add("/places/check-duplicates", post(handlers::check_duplicates))
        .add("/places/merge", post(handlers::merge_places))
        .add("/places/deduplicate", post(handlers::deduplicate))
        .add("/places/review-queue", get(handlers::get_review_queue))
        .add(
            "/places/review-queue/{id}/decision",
            post(handlers::review_decision),
        )
        .add(
            "/places/{id}",
            get(handlers::get_place)
                .put(handlers::update_place)
                .delete(handlers::delete_place),
        )
        .add("/places/{id}/export", get(handlers::export_place_data))
        .add("/places/{id}/masked", get(handlers::masked_place))
        .add("/places/{id}/audit", get(handlers::audit_for_place))
        .add("/audit/recent", get(handlers::audit_recent))
        // Both registration surfaces carry these: this crate is
        // mid-conversion and a handler added to only one compiles
        // cleanly while serving 404 from the other.
        .add("/records/verify", get(handlers::verify_record_integrity))
        .add("/audit/verify", get(handlers::verify_audit_integrity))
}

/// Root-level Prometheus scrape route (`GET /metrics.prom`).
#[must_use]
pub fn metrics_routes() -> axa::routes::Routes {
    use axa::prelude::{Routes, get};
    Routes::new().add("/metrics.prom", get(handlers::metrics_prom))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generated `OpenAPI` document advertises the root-level
    /// `/metrics.prom` scrape path (DB-free — built from `ApiDoc`).
    #[test]
    fn openapi_includes_metrics_prom_path() {
        let doc = ApiDoc::openapi();
        assert!(
            doc.paths.paths.contains_key("/metrics.prom"),
            "OpenAPI paths missing /metrics.prom: {:?}",
            doc.paths.paths.keys().collect::<Vec<_>>()
        );
    }

    /// The generated `OpenAPI` document advertises the T-9 geo-radius
    /// `GET /api/places/nearby` and the paginated
    /// `GET /api/places/search` routes (DB-free — built from `ApiDoc`).
    #[test]
    fn openapi_includes_nearby_and_search_paths() {
        let doc = ApiDoc::openapi();
        for path in ["/api/places/nearby", "/api/places/search"] {
            assert!(
                doc.paths.paths.contains_key(path),
                "OpenAPI paths missing {path}: {:?}",
                doc.paths.paths.keys().collect::<Vec<_>>()
            );
        }
    }

    /// The generated `OpenAPI` document advertises `GET /api/whoami` and
    /// defines the `bearer` security scheme its `security` requirement
    /// references (DB-free — built from `ApiDoc`).
    #[test]
    fn openapi_includes_whoami_path_and_bearer_scheme() {
        let doc = ApiDoc::openapi();
        assert!(
            doc.paths.paths.contains_key("/api/whoami"),
            "OpenAPI paths missing /api/whoami: {:?}",
            doc.paths.paths.keys().collect::<Vec<_>>()
        );
        let components = doc.components.expect("components present");
        assert!(
            components.security_schemes.contains_key("bearer"),
            "OpenAPI security schemes missing 'bearer': {:?}",
            components.security_schemes.keys().collect::<Vec<_>>()
        );
    }
}
