//! Placeholder for typed FHIR `Bundle` support.
//!
//! The search handler currently emits a `searchset` `Bundle` ad hoc with
//! `serde_json::json!` (see [`super::handlers::search_fhir_workers`]). This
//! module is reserved for promoting that to typed `Bundle` / `BundleEntry`
//! structs (and transaction/batch bundles) if the FHIR surface grows; it is
//! intentionally empty for now.
