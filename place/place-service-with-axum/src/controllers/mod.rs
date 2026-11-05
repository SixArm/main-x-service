//! axa-idiomatic controllers mounted on the axa router.
//!
//! The native `/api` surface lives in [`crate::api::rest`]; this module
//! holds the additional axa `Routes` groups wired in
//! [`crate::app::App::routes`]. Today that is the FHIR R5 surface.

/// HL7 FHIR R5 endpoints for the `Location` resource (`/fhir/*`).
pub mod fhir;
