//! `portfolio-service` — a loco.rs registry for **work-item** records
//! across four collections (portfolio / project / product / program),
//! with CRUD + matching.
//!
//! The API DTO is `portfolio_matcher::WorkItem` itself: the service stores
//! it verbatim (JSONB in the `work_items` table) and matches with the
//! canonical [`portfolio_matcher`] engine, so there is no separate domain
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
pub mod initializers;
pub mod merge;
pub mod metrics;
pub mod models;
pub mod openapi;
pub mod streaming;
pub mod tasks;
pub mod validation;
pub mod workers;
