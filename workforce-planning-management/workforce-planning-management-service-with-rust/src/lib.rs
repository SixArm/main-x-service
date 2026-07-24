//! `workforce-planning-management-service` — a loco.rs consumer application
//! for **workforce planning management** across the employee lifecycle:
//! requisitions and the applicant-tracking pipeline, onboarding
//! checklists, employee records with the derived org chart, time &
//! attendance, leave, shift scheduling, benefits, performance reviews,
//! training enrollments, succession planning, payroll runs with
//! derived payslips, and salary benchmarking.
//!
//! WPM **owns the employment relationship and its operational state**
//! (its own tables) and **references identities**: humans are
//! person-service records, professional identities worker-service,
//! employers organization-service, training courses course-service —
//! as `EntityRef` URNs, never duplicated demographics.
//!
//! ## Modules
//!
//! - [`app`] — loco `Hooks` wiring (routes, truncate, seed).
//! - [`controllers`] — Axum controllers, one per pillar.
//! - [`rules`] — the **pure core**: lifecycle machines, leave / time
//!   arithmetic, org-chart cycle check, payslip arithmetic, benchmark
//!   flags. DB-free and exhaustively unit-tested.
//! - [`models`] — `SeaORM` entities + CRUD helpers.
//! - [`clients`] — upstream service lookups (stub-first).
//! - [`auth`] — offline PASETO verification + ABAC + masking.
//! - [`streaming`] — event envelope + in-memory / outbox transports.
//!
//! See `../spec/index.md` for the living specification.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
// The hand-written OpenAPI document is one large nested `json!` literal.
#![recursion_limit = "512"]

pub mod app;
pub mod auth;
pub mod clients;
/// Backward-compatibility shims for the 2026-07-23 `HCM` → `WPM` rename.
pub mod compat;
pub mod controllers;
pub mod metrics;
pub mod models;
pub mod openapi;
pub mod rules;
pub mod streaming;
pub mod tasks;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
