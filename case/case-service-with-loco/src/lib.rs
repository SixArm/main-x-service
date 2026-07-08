//! `case-service` — a loco.rs registry for governmental **case**
//! records (CRUD + matching).
//!
//! The API DTO is `case_matcher::Case` itself: the service stores it
//! verbatim (JSONB in the `cases` table) and matches with the canonical
//! [`case_matcher`] engine, so there is no separate domain model to
//! drift.
//!
//! ## Modules
//!
//! - [`app`] — loco `Hooks` wiring (routes, workers, truncate/seed).
//! - [`controllers`] — Axum controllers: CRUD, `match`, `check-duplicates`.
//! - [`models`] — `SeaORM` entity + CRUD helpers over the stored payload.
//! - [`workers`] — background workers (loco `BackgroundWorker`).
//! - [`tasks`], [`initializers`], [`data`] — loco extension points.
//!
//! See `spec/index.md` for the living specification.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod app;
pub mod auth;
pub mod controllers;
pub mod data;
/// HL7 FHIR R5 interop — the `Task` resource mapping (best-effort) over
/// the stored `case_matcher::Case` DTO.
pub mod fhir;
pub mod initializers;
pub mod merge;
pub mod metrics;
pub mod models;
pub mod openapi;
/// Durable event bus Phase 3: the `event_outbox` relay + retention loop.
pub mod relay;
pub mod streaming;
pub mod tasks;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
pub mod workers;
