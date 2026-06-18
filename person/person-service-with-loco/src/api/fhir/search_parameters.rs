//! FHIR search-parameter parsing (reserved).
//!
//! Placeholder for richer FHIR search-parameter handling (modifiers,
//! prefixes, chained/composite params). Today the supported subset
//! (`name`, `family`, `given`, `identifier`, `birthdate`, `gender`,
//! `_count`) is parsed directly into
//! [`super::handlers::FhirSearchParams`](crate::api::fhir::handlers::FhirSearchParams) by Axum's `Query` extractor.
//! This module exists for when that parsing grows beyond simple
//! field-to-field mapping.

// FHIR search parameter parsing and handling
