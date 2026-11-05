//! # Place Service
//!
//! The library crate behind the **Place Service** microservice — a
//! centralized registry of place identities modeled on
//! [schema.org/Place](https://schema.org/Place). It is one member of the
//! Main X Index family of identity-registry crates (Person, Worker, Place,
//! Thing, Event, Course), all sharing the same architecture, matching
//! algorithms, and operational conventions.
//!
//! This crate exposes the pure, dependency-light core of the service so it
//! can be reused as a Rust library: the domain models, the matching engine,
//! validation/normalization rules, and privacy helpers. The HTTP/gRPC
//! server, database persistence, search index, and event streaming live in
//! the binary tier and are intentionally not part of this library root.
//!
//! ## Module map
//!
//! - [`models`] — the domain types ([`Place`](crate::models::place::Place),
//!   [`PostalAddress`](crate::models::address::PostalAddress),
//!   [`GeoCoordinates`](crate::models::geo::GeoCoordinates),
//!   [`PlaceType`](crate::models::place_type::PlaceType),
//!   [`PlaceIdentifier`](crate::models::identifier::PlaceIdentifier), …).
//! - [`matching`] — pairwise place comparison: per-component similarity
//!   functions plus the weighted [`compute_match`](crate::matching::scoring::compute_match)
//!   scorer, and an [`adapter`](crate::matching::adapter) to the canonical
//!   `place-matcher` crate.
//! - [`validation`] — required-field, range, and format checks plus address
//!   normalization.
//! - [`privacy`] — field masking and GDPR data export.
//! - [`metrics`] — process-wide Prometheus counters and histograms.
//!
//! ## Example
//!
//! ```
//! use place_service::models::place::Place;
//! use place_service::matching::scoring::{compute_match, MatchWeights};
//!
//! // A typo'd duplicate still scores as a near-certain match.
//! let a = Place::new("Central Park");
//! let b = Place::new("Centrl Park");
//! let result = compute_match(&a, &b, &MatchWeights::default());
//! assert!(result.score > 0.8);
//! ```

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod api;
pub mod app;
/// Compliance controls: row-level integrity and keyed MACs.
pub mod compliance;
pub mod config;
/// Loco-idiomatic controllers (the mounted FHIR R5 surface).
pub mod controllers;
pub mod db;
pub mod error;
/// HL7 FHIR R5 interop: the `Location` resource conversions, wire
/// types, and search-parameter parsing for the mounted `/fhir` endpoints.
pub mod fhir;
pub mod matching;
pub mod metrics;
pub mod models;
/// Structured logging + OpenTelemetry OTLP export (traces + metrics).
pub mod observability;
pub mod privacy;
/// Durable event bus **Phase 3** — the transactional-outbox relay worker:
/// a background loop that drains unpublished `event_outbox` rows to an
/// [`relay::EventSink`], stamps `published_at`, and purges old published
/// rows. Off by default; runs only when `PLACE_EVENT_TRANSPORT=outbox`
/// and `PLACE_EVENT_RELAY` are both set (see `agents/share/event-bus.md`).
pub mod relay;
pub mod search;
pub mod streaming;
pub mod validation;

/// Re-export the crate's error type and its `Result` alias at the crate
/// root so callers can write `place_service::{Error, Result}` without
/// reaching into the `error` module.
pub use error::{Error, Result};
