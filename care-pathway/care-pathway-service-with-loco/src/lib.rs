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

/// Event-log and journey-feature export codecs (spec `13-tasks.md`
/// T-14a) — pure row-shaping over [`tba::InstanceAnalysis`] plus the
/// instance layer's segments/steps/events/team, DB-free and
/// unit-tested. See [`controllers::exports`] for the HTTP surface.
pub mod analytics;
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
/// Journey data-quality and missingness report: eight closed-vocabulary
/// defect codes (matching `data::journeys::DEFECT_CODES` name for
/// name) plus per-stage missingness percentage and entropy (spec
/// `13-tasks.md` T-14h). Never imputes — the report is the finding.
pub mod data_quality;
/// HL7 FHIR R5 interop: the `PlanDefinition` resource + envelope wire
/// types, and search-parameter parsing for the mounted `/fhir` endpoints.
pub mod fhir;
/// The time-based-analysis flow-gauge refresh loop: default-off,
/// bounded by a series cap, and suppressed below a cohort floor.
pub mod flow_metrics;
pub mod initializers;
/// Pure rules for the care-pathway instance layer.
pub mod instances;
/// Stitched journeys: following a `continues_as` chain across service
/// boundaries so time-based analysis measures the whole journey.
pub mod journey;
pub mod merge;
pub mod metrics;
pub mod models;
/// Structured logging + real OpenTelemetry OTLP export (repo `tasks.md`
/// PRO-H12, slice 5 of 7). Installed via [`app::App::init_logger`];
/// [`observability::trace_mw`] is layered in `App::after_routes`.
pub mod observability;
pub mod openapi;
/// Field masking + the GDPR right-of-access export envelope.
pub mod privacy;
/// Durable event bus Phase 3: the outbox relay (drain → sink → mark
/// published) + retention purge. See [`agents/share/event-bus.md`].
pub mod relay;
/// Tantivy full-text search: index schema, engine, and query surface.
pub mod search;
/// Rule-based cohort splits: `contains=`/`excludes=` predicates and
/// the matched/complement partition, plus the two-cell suppression
/// table that protects a withheld side from subtraction against the
/// published unsplit total (spec `13-tasks.md` T-14f). Named `split`,
/// not `rules`, because `crate::instances` is already aliased `rules`
/// throughout the controller layer.
pub mod split;
pub mod streaming;
/// Disclosure control: the deployment-configurable cell-count floor,
/// `Withhold`/`Remove` rendering, and secondary suppression of
/// marginals for a stratified table (spec `13-tasks.md` T-14k).
pub mod suppression;
pub mod tasks;
/// Time-based analysis (TBA): pure computation over an instance's
/// clock and recorded segments — the value-adding ratio, constraint
/// ranking, cohort percentiles, queueing-theory flow, (spec
/// `13-tasks.md` T-14d) per-stage anchors, adjacent-pair delays, and
/// compliance scored against a named two-stage interval instead of
/// the whole clock, and (T-14e) a Kaplan–Meier survival estimator
/// treating an open instance as right-censored rather than mixing its
/// running lead time in as if it had already closed, plus a two-sample
/// log-rank test ready for T-14f's cohort split to call.
pub mod tba;
pub mod validation;
/// Journey variants (pathway strings): named, defaulted, echoed
/// transform parameters over a cohort's segments into a frequency/
/// coverage Pareto plus per-position duration quantiles (spec
/// `13-tasks.md` T-14c). Pure, DB-free.
pub mod variants;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
pub mod workers;
