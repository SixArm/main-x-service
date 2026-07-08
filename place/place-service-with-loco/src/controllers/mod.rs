//! Loco-idiomatic controllers mounted on the loco router.
//!
//! The native `/api` surface lives in [`crate::api::rest`]; this module
//! holds the additional loco `Routes` groups wired in
//! [`crate::app::App::routes`]. Today that is the FHIR R5 surface.

/// HL7 FHIR R5 endpoints for the `Location` resource (`/fhir/*`).
pub mod fhir;
