//! `patient-flow-service` — an axa consumer application for hospital
//! **patient flow and bed management**: wards / bays / beds with a live
//! bed state machine, inpatient stays from admission to discharge
//! (SAFER fields, `Red2Green` journal, DTOC), bed requests with
//! rule-checked allocation, infection-control flags, virtual wards, and
//! derived whiteboard / at-a-glance / locate / capacity reads.
//!
//! Patient Flow **owns operational state** (its own tables) and
//! **references identities**: patients are person-service records, staff
//! worker-service, sites place-service — as `EntityRef` URNs, never
//! duplicated demographics.
//!
//! ## Modules
//!
//! - [`app`] — axa `Hooks` wiring (routes, truncate, seed).
//! - [`controllers`] — Axum controllers: topology CRUD, bed states,
//!   stays, bed requests, boards, audits.
//! - [`flow`] — the **pure core**: bed state machine, allocation rules,
//!   `Red2Green` / DTOC logic. DB-free and exhaustively unit-tested.
//! - [`models`] — row structs and every SQL statement the service runs
//!   (sqlx `query!` literals), plus the migrator.
//! - [`clients`] — upstream service lookups (stub-first).
//! - [`auth`] — offline PASETO verification + ABAC + masking.
//! - [`streaming`] — event envelope + in-memory / outbox transports.
//!
//! See `../spec/index.md` for the living specification.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod app;
pub mod auth;
/// Request limits per client and a lockout after failed authentication.
pub mod client_limits;
pub mod clients;
pub mod controllers;
pub mod flow;
pub mod metrics;
pub mod models;
pub mod openapi;
pub mod streaming;
pub mod tasks;
pub mod validation;
