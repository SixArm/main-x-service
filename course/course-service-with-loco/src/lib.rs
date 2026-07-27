//! Course Service — schema.org/Course-aligned course-identity registry.
//!
//! Library entry-point. Re-exports the domain models and the
//! HTTP-layer plumbing so downstream callers (test harnesses, the
//! `course-service` binary, future gRPC stubs) can build a
//! [`api::rest::AppState`] without poking at internal modules.
//!
//! For the canonical behaviour reference, see [`../spec.md`](../spec.md).
//! For per-area detail (domain model, matching, REST surface, testing),
//! see the `AGENTS/*` files under [`AGENTS/`](../AGENTS/).

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod api;
pub mod app;
/// Compliance controls: row-level integrity and keyed MACs.
pub mod compliance;
pub mod config;
pub mod db;
pub mod error;
/// HL7 FHIR R5 **non-standard** `Basic` (`code = course`) representation:
/// resource wire types, DTO↔resource conversions, and search-parameter
/// parsing for the mounted `/fhir` endpoints. Non-standard because no FHIR R5
/// resource models an educational course (see the module docs).
pub mod fhir;
pub mod matching;
/// Process-wide Prometheus metrics, served at `/metrics.prom`.
pub mod metrics;
pub mod models;
pub mod privacy;
/// Durable event bus **Phase 3** — the `course_outbox` relay worker
/// (drains unpublished rows to an [`relay::EventSink`], stamps
/// `published_at`, purges old rows). A no-op unless
/// `COURSE_EVENT_TRANSPORT=outbox` and `COURSE_EVENT_RELAY` are set.
pub mod relay;
pub mod search;
pub mod streaming;
pub mod validation;

/// Crate-wide [`enum@Error`] enum and [`Result`] alias, re-exported at the
/// crate root for convenient `course_service::{Error, Result}` use.
pub use error::{Error, Result};
