//! HTTP controllers for the case service.

pub mod cases;
/// Compliance evidence: service identification and the CycloneDX SBOM.
pub mod compliance;
pub mod docs;
pub mod fhir;
pub mod links;
pub mod metrics;

/// Map a model-layer error to its HTTP shape: a missing record is
/// `404 Not Found`; a database error keeps axa's own mapping, and
/// anything else is a `500`. This is `From<ModelError> for axa::Error`
/// ([`crate::models::ModelError`]), kept as a named function because
/// every controller lookup already routes through it.
#[must_use]
pub fn model_not_found(err: crate::models::ModelError) -> axa::Error {
    err.into()
}
