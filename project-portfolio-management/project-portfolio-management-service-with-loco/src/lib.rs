//! `project-portfolio-management-service` — a loco.rs registry for **plan**
//! records in one recursive collection, with CRUD + matching.
//!
//! The API DTO is `project_portfolio_management_matcher::Plan` itself: the service stores
//! it verbatim (JSONB in the `plans` table) and matches with the
//! canonical [`project_portfolio_management_matcher`] engine, so there is no separate domain
//! model to drift. Every plan lives in one `plans` table; `kind` is an
//! optional descriptive label and matching is kind-agnostic (there is
//! no kind gate).
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
// The hand-written OpenAPI document is one large nested `json!` tree.
#![recursion_limit = "512"]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod app;
pub mod auth;
/// Pure workflow-automation rules (trigger matching, action shapes, due-ness).
pub mod automation;
/// Pure collaborative-review + assignee-workload rules.
pub mod collaboration;
/// Compliance controls: row-level integrity and keyed MACs.
pub mod compliance;
pub mod controllers;
pub mod data;
/// Pure rules for the engineering-team features (tasks / burndown / MoSCoW).
pub mod engineering;
/// The time-based-analysis flow-gauge refresh loop: default-off,
/// bounded by a series cap, and suppressed below a board-size floor.
pub mod flow_metrics;
/// Pure PPM governance rules (proposal pipeline, phase gates, risks, money).
pub mod governance;
pub mod initializers;
/// Pure derivations for the executive insight areas (CEO / CFO / CTO).
pub mod insights;
/// Pure bird.s-eye lifecycle rules (the funnel + next-phase readiness).
pub mod lifecycle;
pub mod merge;
pub mod metrics;
pub mod models;
pub mod openapi;
/// The pure, explainable Smart Score behind data-driven prioritisation.
pub mod prioritisation;
/// Field masking (owner org, lead ref) + the GDPR right-of-access export envelope.
pub mod privacy;
/// Durable event bus Phase 3: the outbox relay loop + retention purge.
pub mod relay;
/// The set-and-forget ticker: the optional scheduled-action sweep loop.
pub mod scheduler;
/// Tantivy full-text search: index schema, engine, and query surface.
pub mod search;
/// Point-in-time estate snapshots (board / CRO trends).
pub mod snapshots;
/// Pure PPM strategy rules (scenario evaluation, OKR weights, ROI).
pub mod strategy;
pub mod streaming;
pub mod tasks;
/// Time-based analysis (TBA): pure computation over the task
/// status-transition log — cycle versus lead time, flow efficiency,
/// rework and first-pass yield, the service level expectation,
/// constraint ranking, and queueing-theory flow.
pub mod tba;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
/// Pure PPM visibility rules (schedule math, RAG, capacity, CSV).
pub mod visibility;
pub mod workers;
