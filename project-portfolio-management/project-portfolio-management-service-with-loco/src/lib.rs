//! `project-portfolio-management-service` — a loco.rs registry for **work-item** records
//! across four collections (portfolio / project / product / program),
//! with CRUD + matching.
//!
//! The API DTO is `project_portfolio_management_matcher::WorkItem` itself: the service stores
//! it verbatim (JSONB in the `work_items` table) and matches with the
//! canonical [`project_portfolio_management_matcher`] engine, so there is no separate domain
//! model to drift. The four collections share one table; a row's `kind`
//! is its collection, and matching is always scoped within one collection
//! (the matcher's kind gate).
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
/// Pure PPM governance rules (proposal pipeline, phase gates, risks, money).
pub mod governance;
/// Pure derivations for the executive insight areas (CEO / CFO / CTO).
pub mod insights;
/// Pure PPM strategy rules (scenario evaluation, OKR weights, ROI).
pub mod strategy;
/// Pure PPM visibility rules (schedule math, RAG, capacity, CSV).
pub mod visibility;
pub mod initializers;
pub mod merge;
pub mod metrics;
pub mod models;
pub mod openapi;
/// Durable event bus Phase 3: the outbox relay loop + retention purge.
pub mod relay;
/// Point-in-time estate snapshots (board / CRO trends).
pub mod snapshots;
pub mod streaming;
pub mod tasks;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
pub mod workers;
