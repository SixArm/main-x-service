//! `contact-relationship-management-service` — a loco.rs consumer
//! application for **contact relationship management** across four
//! modules: sales automation (contacts / accounts / leads with
//! deterministic scoring / deal pipelines / forecasting), marketing
//! automation (consent-first campaigns / segments / nurture),
//! customer service & support (tickets / SLA / knowledge base), and
//! analytics (derived dashboards).
//!
//! CRM **owns relationship state** (its own tables) and **references
//! identities**: contacts are person-service records, accounts
//! organization-service, reps/agents worker-service — as `EntityRef`
//! URNs, never duplicated demographics. Identity dedup stays with the
//! upstream matchers.
//!
//! ## Modules
//!
//! - [`app`] — loco `Hooks` wiring (routes, truncate, seed).
//! - [`controllers`] — Axum controllers, one per module.
//! - [`rules`] — the **pure core**: lifecycle machines, lead scoring
//!   with breakdown, KPI arithmetic, SLA derivation, segment
//!   evaluation with the structural consent gate.
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
