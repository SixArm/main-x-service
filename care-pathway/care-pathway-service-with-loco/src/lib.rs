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
pub mod controllers;
pub mod data;
/// HL7 FHIR R5 interop: the `PlanDefinition` resource + envelope wire
/// types, and search-parameter parsing for the mounted `/fhir` endpoints.
pub mod fhir;
pub mod initializers;
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
