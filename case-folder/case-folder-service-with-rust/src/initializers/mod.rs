//! Loco initializers for the case-folder tracker.
//!
//! Each submodule registers one external-service HTTP client as an Axum
//! `Extension` so controllers can pull it out of the request. In offline
//! / e2e mode the [`bootstrap_stubs`] initializer swaps every client for
//! an in-process stub, and [`auth`] installs the session guard
//! middleware over `/api/*`.

/// Builds `AuthState` and layers the `/api/*` session-guard middleware.
pub mod auth;
/// Offline / e2e mode: swap every client for an in-process stub and seed demo data.
pub mod bootstrap_stubs;
/// Registers the Main Event Service client (move-event audit log).
pub mod main_event_service_client;
/// Registers the Main Patient Service client (patient identity lookup).
pub mod main_patient_service_client;
/// Registers the Main Place Service client (buildings / rooms / cabinets).
pub mod main_place_service_client;
/// Registers the Main Thing Service client (folders + volumes).
pub mod main_thing_service_client;
/// Registers the Main Worker Service client (staff directory).
pub mod main_worker_service_client;
