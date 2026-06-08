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

use crate::config::MatchingConfig;
use crate::models::thing::Thing;
use scoring::{MatchConfidence, MatchResult, MatchWeights, compute_match};

/// Service-tier matcher facade carried by the REST
/// [`AppState`](crate::api::rest::AppState).
///
/// Wraps the native weighted [`compute_match`](scoring::compute_match)
/// scorer with a configurable `is_match` threshold.
pub struct ThingMatcher {
    /// Tunable weights for the component scores.
    weights: MatchWeights,
    /// `is_match` cut-off score in `[0.0, 1.0]`.
    threshold: f64,
}

impl ThingMatcher {
    /// Build a matcher from the service [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            weights: MatchWeights::default(),
            threshold: config.threshold_score,
        }
    }

    /// Score two things, returning the full [`MatchResult`](scoring::MatchResult).
    pub fn score(&self, a: &Thing, b: &Thing) -> MatchResult {
        compute_match(a, b, &self.weights)
    }

    /// Whether two things score at or above the configured threshold.
    pub fn is_match(&self, a: &Thing, b: &Thing) -> bool {
        self.score(a, b).score >= self.threshold
    }

    /// The configured `is_match` threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

/// Classify a confidence band into a stable string label for API responses.
pub fn confidence_label(c: &MatchConfidence) -> &'static str {
    match c {
        MatchConfidence::Certain => "certain",
        MatchConfidence::Probable => "probable",
        MatchConfidence::Possible => "possible",
        MatchConfidence::Unlikely => "unlikely",
    }
}
