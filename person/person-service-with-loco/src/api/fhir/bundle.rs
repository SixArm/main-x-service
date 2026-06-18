//! FHIR `Bundle` resource support (reserved).
//!
//! Placeholder for a typed `Bundle` resource. Today the search handler
//! [`super::handlers::search_fhir_persons`](crate::api::fhir::handlers::search_fhir_persons) assembles a `searchset`
//! Bundle ad hoc via `serde_json::json!` (`resourceType`/`type`/`total`/
//! `entry`). This module exists for when that wire shape is promoted to
//! a strongly-typed struct (entries, links, paging) mirroring the other
//! DTOs in [`super::resources`](crate::api::fhir::resources).

// FHIR Bundle resource implementation
