//! Probabilistic matching for [`Thing`](crate::models::thing::Thing) records.
//!
//! This module is the service-side matching layer. It is split into one
//! sub-module per scoring component plus a
//! [`scoring`](crate::matching::scoring) orchestrator and an
//! [`adapter`](crate::matching::adapter) to the canonical sibling
//! `thing-matcher` crate:
//!
//! - [`scoring`](crate::matching::scoring) —
//!   [`scoring::compute_match`](crate::matching::scoring::compute_match), the
//!   public entry point that combines components into a
//!   [`scoring::MatchResult`](crate::matching::scoring::MatchResult).
//! - [`name`](crate::matching::name) — name similarity (Jaro-Winkler).
//! - [`identifier`](crate::matching::identifier) — exact identifier-pair match
//!   plus the deterministic short-circuit predicate.
//! - [`description`](crate::matching::description) — free-text description
//!   similarity (Jaro-Winkler).
//! - [`url`](crate::matching::url) — scheme/case-normalized URL and URL-list
//!   similarity.
//! - [`phonetic`](crate::matching::phonetic) — Soundex, used as a small bonus.
//! - [`adapter`](crate::matching::adapter) — converts a service
//!   [`Thing`](crate::models::thing::Thing) into the canonical `thing-matcher`
//!   shape.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::scoring::{compute_match, MatchWeights};
//! use thing_service::models::thing::Thing;
//!
//! let a = Thing::new("Pride and Prejudice");
//! let b = Thing::new("Pride and Prejudice");
//! let result = compute_match(&a, &b, &MatchWeights::default());
//! assert!(result.score > 0.95);
//! ```

/// Adapter from the service [`Thing`](crate::models::thing::Thing) to the
/// canonical `thing-matcher` representation.
pub mod adapter;
/// Free-text description similarity (Jaro-Winkler).
pub mod description;
/// Identifier-pair matching and the deterministic short-circuit predicate.
pub mod identifier;
/// Name similarity (Jaro-Winkler).
pub mod name;
/// Soundex phonetic coding, applied as a scoring bonus.
pub mod phonetic;
/// Weighted scoring orchestrator that combines all components.
pub mod scoring;
/// Scheme/case-normalized URL and URL-list similarity.
pub mod url;

/// Re-export the canonical `thing-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, and
/// the `Thing` builder without taking a separate dependency. Pair this
/// with [`adapter::to_matcher_thing`] to score two service `Thing`
/// records through the reference algorithm.
pub use ::thing_matcher as matcher_lib;
