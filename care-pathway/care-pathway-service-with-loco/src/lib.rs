//! `care-pathway-service` — a loco.rs registry for clinical
//! **care-pathway** records (CRUD + matching).
//!
//! The API DTO is `care_pathway_matcher::CarePathway` itself: the
//! service stores it verbatim (JSONB in the `care_pathways` table) and
//! matches with the canonical [`care_pathway_matcher`] engine, so there
//! is no separate domain model to drift.
//!
//! ## Modules
//!
//! - [`app`] — loco `Hooks` wiring (routes, workers, truncate/seed).
//! - [`controllers`] — Axum controllers: CRUD, `match`, `check-duplicates`,
//!   plus the root `/metrics.prom` Prometheus endpoint.
//! - [`metrics`] — process-wide Prometheus registry + text rendering.
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
/// Regulatory-compliance controls: the tamper-evident audit chain,
/// read/disclosure auditing, GDPR Art. 17 erasure, the SOUP/SBOM register,
/// FHIR Bulk Data, and the runtime posture surface. The family's reference
/// implementation — see `agents/share/compliance-for-healthcare.md` §2.
/// Bulk operations: durable `bulk_jobs` state and artifact storage.
pub mod bulk;
pub mod compliance;
pub mod controllers;
pub mod data;
/// HL7 FHIR R5 interop: the `PlanDefinition` resource + envelope wire
/// types, and search-parameter parsing for the mounted `/fhir` endpoints.
pub mod fhir;
pub mod initializers;
/// Pure rules for the care-pathway instance layer.
pub mod instances;
pub mod merge;
pub mod metrics;
pub mod models;
pub mod openapi;
/// Durable event bus Phase 3: the outbox relay (drain → sink → mark
/// published) + retention purge. See [`agents/share/event-bus.md`].
pub mod relay;
pub mod streaming;
pub mod tasks;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
pub mod workers;
