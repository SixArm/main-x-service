//! `case-service` — an axa registry for governmental **case**
//! records (CRUD + matching).
//!
//! The API DTO is `case_matcher::Case` itself: the service stores it
//! verbatim (JSONB in the `cases` table) and matches with the canonical
//! [`case_matcher`] engine, so there is no separate domain model to
//! drift.
//!
//! ## Modules
//!
//! - [`app`] — axa `Hooks` wiring (routes, workers, truncate/seed).
//! - [`controllers`] — Axum controllers: CRUD, `match`, `check-duplicates`.
//! - [`models`] — the table rows and every SQL statement the service runs
//!   (sqlx `query!` literals), plus the migrator.
//! - [`workers`] — background workers (axa `BackgroundWorker`).
//! - [`tasks`], [`initializers`], [`data`] — axa extension points.
//!
//! See `spec/index.md` for the living specification.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod app;
pub mod auth;
/// Bulk import/export (BLK-5): async, job-based load/extract via a
/// Postgres-backed background worker.
/// See `agents/share/bulk-import-export.md`.
pub mod bulk;
/// Request limits per client and a lockout after failed authentication.
pub mod client_limits;
/// Regulatory-compliance controls: the tamper-evident audit chain and
/// read/disclosure auditing, adopted from the care-pathway reference
/// implementation (`spec/compliance` §8.5 step 3).
pub mod compliance;
pub mod controllers;
/// The secret that signs page cursors (`CURSOR_KEY`).
pub mod cursor_key;
pub mod data;
/// HL7 FHIR R5 interop — the `Task` resource mapping (best-effort) over
/// the stored `case_matcher::Case` DTO.
pub mod fhir;
/// Value frequencies for match evidence (`agents/share/match-evidence.md` §7).
pub mod frequencies;
pub mod initializers;
pub mod merge;
pub mod metrics;
pub mod models;
pub mod openapi;
/// Durable event bus Phase 3: the `event_outbox` relay + retention loop.
pub mod relay;
/// Tantivy full-text search: index schema, engine, and query surface.
pub mod search;
pub mod streaming;
pub mod tasks;
pub mod validation;
