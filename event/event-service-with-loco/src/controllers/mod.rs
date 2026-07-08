//! Loco-idiomatic controllers.
//!
//! The event service's primary REST surface is the hand-written Axum
//! router in [`crate::api::rest`], merged onto loco in
//! [`crate::app::App::after_routes`]. This `controllers` module holds the
//! newer, loco-native controller style (a `routes()` returning a
//! [`loco_rs::controller::Routes`], added in
//! [`crate::app::App::routes`]) — currently the FHIR R5 surface.

/// HL7 FHIR R5 endpoints for the `Appointment` resource (`/fhir/*`).
pub mod fhir;
