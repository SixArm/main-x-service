//! REST controllers for the organization service.
//!
//! The `organizations` module holds the CRUD, matching, merge, audit and
//! event endpoints (mounted under `/api/organizations`); the `docs`
//! module serves the `OpenAPI` document and the Swagger UI page.

/// OpenAPI JSON + Swagger UI endpoints.
pub mod docs;
/// Organization CRUD, matching, merge, audit, and event endpoints.
pub mod organizations;
