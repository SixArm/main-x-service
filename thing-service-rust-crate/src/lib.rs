//! Thing Service — a generic registry for arbitrary discrete objects.
//!
//! `thing-service` is the most general entity in the Main X Index family of
//! crates. A *thing* is anything that does not fit one of the more
//! opinionated sibling crates (`person`, `worker`, `event`): a book, a
//! paper, a digital asset, a device, a product, a piece of software. The
//! domain model is mapped one-to-one onto
//! [schema.org/Thing](https://schema.org/Thing).
//!
//! # Crate layout
//!
//! The library is organized into five top-level modules, each a thin,
//! dependency-light layer that can be used on its own:
//!
//! - [`models`] — the [`Thing`](models::thing::Thing) domain entity and its
//!   supporting value objects ([`ThingIdentifier`](models::identifier::ThingIdentifier),
//!   [`Consent`](models::consent::Consent)).
//! - [`matching`] — pairwise record-comparison algorithms (name, identifier,
//!   description, URL, phonetic) plus the weighted
//!   [`compute_match`](matching::scoring::compute_match) scorer and the
//!   adapter to the canonical `thing-matcher` crate.
//! - [`metrics`] — process-wide Prometheus counters and histograms.
//! - [`validation`] — required-field, URL-format, and identifier-format
//!   checks plus in-place normalization.
//! - [`privacy`] — per-field masking and GDPR data export.
//!
//! # Examples
//!
//! Construct, validate, and match a [`Thing`](models::thing::Thing):
//!
//! ```
//! use thing_service::models::thing::Thing;
//! use thing_service::matching::scoring::{compute_match, MatchWeights};
//! use thing_service::validation::validate_thing;
//!
//! let a = Thing::new("Pride and Prejudice");
//! let b = Thing::new("Prde and Prejudice"); // typo
//!
//! // A fresh thing with a non-empty name is valid.
//! assert!(validate_thing(&a).is_empty());
//!
//! // A close name produces a high probabilistic score.
//! let result = compute_match(&a, &b, &MatchWeights::default());
//! assert!(result.score > 0.85);
//! ```

/// REST API surface and shared response envelope.
pub mod api;
/// Service configuration loaded from the environment.
pub mod config;
/// PostgreSQL persistence: connection pool, repository, audit log.
pub mod db;
/// Crate-level error type and result alias.
pub mod error;
/// Matching engine: per-component similarity functions, the weighted
/// scorer, and the adapter to the canonical `thing-matcher` crate.
pub mod matching;
/// Prometheus metric registry and handles for the service.
pub mod metrics;
/// Domain models: the [`Thing`](models::thing::Thing) entity and its
/// supporting value objects.
pub mod models;
/// Privacy controls: per-field masking and GDPR export.
pub mod privacy;
/// Tantivy-backed full-text search engine.
pub mod search;
/// Event-streaming publisher for CRUD/merge operations.
pub mod streaming;
/// Data-quality validation and in-place normalization.
pub mod validation;

pub use error::{Error, Result};
