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
#![warn(clippy::clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod matching;
pub mod metrics;
pub mod models;
pub mod privacy;
pub mod search;
pub mod streaming;
pub mod validation;

pub use error::{Error, Result};
