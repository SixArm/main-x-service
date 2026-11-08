//! HTTP controllers for the care-pathway service.

pub mod care_pathways;
/// Compliance-evidence endpoints (`/api/compliance*`): posture, SBOM, and
/// audit-chain verification.
pub mod compliance;
/// Journey data-quality and missingness report (spec `13-tasks.md`
/// T-14h). See [`crate::data_quality`] for the pure detectors this
/// module only loads data for and renders.
pub mod data_quality;
pub mod docs;
/// Bulk `event_log` / `journey_features` export codecs over the
/// instance layer (spec `13-tasks.md` T-14a). See [`crate::analytics`]
/// for the pure row-shaping this module only loads data for and
/// renders.
pub mod exports;
/// HL7 FHIR R5 endpoints for the `PlanDefinition` resource (`/fhir/*`).
pub mod fhir;
pub mod insights;
pub mod instances;
/// Cross-service journey links (`entity_links` write side).
pub mod links;
pub mod metrics;
/// The stored duplicate-review queue (§13 T-10): list + decide. Rows are
/// written by the native bulk-import pipeline (`crate::bulk::pipeline`,
/// `provenance = "import"`); this module is the operator-facing surface
/// over `crate::models::review_queue`.
pub mod review_queue;
/// Time-based analysis: segment + clock recording, and the derived
/// per-instance, cohort, constraint and flow views.
pub mod tba;

/// Map a model-layer error to its HTTP shape: a missing record is
/// `404 Not Found`; a database error keeps axa's own mapping, and
/// anything else is a `500`. This is `From<ModelError> for axa::Error`
/// ([`crate::models::ModelError`]), kept as a named function because
/// every controller lookup already routes through it.
#[must_use]
pub fn model_not_found(err: crate::models::ModelError) -> axa::Error {
    err.into()
}
