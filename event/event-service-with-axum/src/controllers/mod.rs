//! axa-idiomatic controllers.
//!
//! The event service's primary REST surface is the hand-written Axum
//! router in [`crate::api::rest`], merged onto axa in
//! [`crate::app::App::after_routes`]. This `controllers` module holds the
//! newer, loco-native controller style (a `routes()` returning a
//! [`axa::routes::Routes`], added in
//! [`crate::app::App::routes`]) — currently the FHIR R5 surface.

pub mod fhir;
