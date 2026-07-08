//! REST controllers for the organization service.
//!
//! The `organizations` module holds the CRUD, matching, merge, audit and
//! event endpoints (mounted under `/api/organizations`); the `docs`
//! module serves the `OpenAPI` document and the Swagger UI page; the
//! `metrics` module serves Prometheus metrics at the root `/metrics.prom`.

/// OpenAPI JSON + Swagger UI endpoints.
pub mod docs;
/// HL7 FHIR R5 endpoints for the `Organization` resource (`/fhir/*`).
pub mod fhir;
/// Prometheus `/metrics.prom` endpoint.
pub mod metrics;
/// Organization CRUD, matching, merge, audit, and event endpoints.
pub mod organizations;
