//! Loco-idiomatic controllers.
//!
//! Route groups mounted via `App::routes` (`add_route`) that extract
//! [`crate::api::rest::AppState`] from the loco `AppContext` shared store.
//! Today this hosts the HL7 FHIR R5 surface; the native REST API lives in
//! [`crate::api::rest`].

/// HL7 FHIR R5 endpoints for the `Device` resource (`/fhir/*`).
pub mod fhir;
