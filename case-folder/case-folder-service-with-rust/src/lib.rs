//! Case-Folder Service — NHS paper case-note folder **location** tracker.
//!
//! This crate is a deliberate *consumer* application within the Main-X
//! family: it answers the operational question "where is the physical
//! case-note folder for NHS Number X right now?" by recording every
//! barcode / QR / RFID scan as a move event and replaying that move log
//! into a live location and a tamper-evident audit trail.
//!
//! It is **not** a master-patient index nor a matcher service: it does
//! not own patient / place / worker identity. Those live in the upstream
//! Main-X-Services, which this crate reaches through thin HTTP clients
//! (see the `main_*_service` modules). The folder + volume + move data
//! is this crate's own, and it joins against the upstream services for
//! human-readable names, place hierarchy, and worker roles.
//!
//! Notable domain rules implemented here:
//! - **NHS Number Modulus-11** check-digit validation (`nhs`).
//! - **Geofence-breach alerts** — a move that carries a folder across a
//!   building boundary is surfaced for review (`controllers::alerts`).
//! - **Move audit trail** — append-only scan history is the source of
//!   truth; current location is derived, never edited in place.
//!
//! Built on [Loco](https://loco.rs/) (Axum) in backend-only mode: a JSON
//! API with no view/template tier. The application wiring (initializer
//! order, mounted routes, tasks) lives in `app`.

// SEC-I3: this service has no reason to reach for `unsafe`; forbid it
// outright to match the family's other crate roots.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

/// Loco application wiring: the `App` struct and its `Hooks`
/// implementation (boot, initializer order, route mounting, tasks).
pub mod app;

/// Passwordless magic-link authentication state: identity allowlist,
/// magic + session token minting/verification, session cookie handling.
pub mod auth;

/// JSON API controllers (Axum handlers + route tables), one module per
/// resource. Mounted by `app::App::routes`.
pub mod controllers;

/// Loco initializers — boot-time wiring run in a defined order: the five
/// upstream service clients, then the test/dev stubs, then auth last so
/// its session guard wraps every other route.
pub mod initializers;

/// HTTP client for the upstream Main Event Service: the source of the
/// folder move log (`MoveEvent`s) replayed for history and alerts.
pub mod main_event_service;

/// HTTP client for the upstream Main Patient Service: resolves patient
/// identity (name, NHS number) referenced by folders and moves.
pub mod main_patient_service;

/// HTTP client for the upstream Main Place Service: the place hierarchy
/// (building → records room → file cabinet) used to locate folders and
/// to detect cross-building geofence breaches.
pub mod main_place_service;

/// HTTP client for the upstream Main Thing Service: resolves generic
/// "thing" records associated with folders where applicable.
pub mod main_thing_service;

/// HTTP client for the upstream Main Worker Service: resolves the worker
/// (and role snapshot) recorded as the actor on each move.
pub mod main_worker_service;

/// Domain models owned by this crate (folders, volumes, moves, and their
/// request/response shapes) — distinct from the upstream-owned types.
pub mod models;

/// NHS Number handling: format normalization and Modulus-11 check-digit
/// validation.
pub mod nhs;

/// Shared HTTP response helpers (uniform JSON envelopes and error
/// responses: list wrappers, `422`, `401`, `503`, …).
pub mod responses;

/// Loco background / CLI tasks (e.g. seed), registered in
/// `app::App::register_tasks`.
pub mod tasks;
