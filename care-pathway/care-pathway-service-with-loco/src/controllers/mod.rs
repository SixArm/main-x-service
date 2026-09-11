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
/// Time-based analysis: segment + clock recording, and the derived
/// per-instance, cohort, constraint and flow views.
pub mod tba;

/// Map a model-layer error to its HTTP shape: a missing record is
/// `404 Not Found`; anything else stays a model error (500-class).
/// loco 0.16 stopped mapping `ModelError::EntityNotFound` itself (its
/// `IntoResponse` catch-all turns it into a 500), so every controller
/// lookup routes through this instead of a bare `?`.
#[must_use]
pub fn model_not_found(err: loco_rs::model::ModelError) -> loco_rs::Error {
    match err {
        loco_rs::model::ModelError::EntityNotFound => loco_rs::Error::NotFound,
        other => loco_rs::Error::Model(other),
    }
}
