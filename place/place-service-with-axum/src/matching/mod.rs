//! Pairwise place matching: per-component similarity plus weighted scoring.
//!
//! This module turns two [`Place`](crate::models::place::Place) records into
//! a confidence score in `[0.0, 1.0]`. The component submodules each score
//! one facet; [`scoring`] combines them with configurable weights and a
//! confidence classification, applying a deterministic GLN short-circuit and
//! a phonetic bonus.
//!
//! - [`name`] — Jaro-Winkler name similarity.
//! - [`address`] — weighted field-by-field address similarity.
//! - [`geo`] — Haversine-distance geo similarity (with decay and radius helpers).
//! - [`identifier`] — exact identifier match, with GLN special-casing.
//! - [`phonetic`] — Soundex codes for sounds-alike name matching.
//! - [`scoring`] — the [`compute_match`](scoring::compute_match) entry point.
//! - [`adapter`] — projection to the canonical `place-matcher` crate.
//!
//! For the algorithm contract see `agents/matching.md` and
//! `agents/share/match.md`.

pub mod adapter;
pub mod address;
pub mod geo;
pub mod identifier;
pub mod name;
pub mod phonetic;
pub mod scoring;

/// Re-export the canonical `place-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Place` builder, and all `PlaceCategory` / `PlaceIdScheme` variants
/// without taking a separate dependency. Pair this with
/// [`adapter::to_matcher_place`] to score two service `Place` records
/// through the reference algorithm.
pub use ::place_matcher as matcher_lib;

use crate::config::MatchingConfig;
use crate::models::place::Place;
use scoring::{MatchConfidence, MatchResult, MatchWeights, compute_match};

/// Service-tier matcher facade carried by the REST
/// [`AppState`](crate::api::rest::AppState).
///
/// Wraps the native weighted [`compute_match`](scoring::compute_match)
/// scorer with a configurable `is_match` threshold so handlers have one
/// entry point for scoring and classification.
pub struct PlaceMatcher {
    /// Tunable weights for the component scores.
    weights: MatchWeights,
    /// `is_match` cut-off score in `[0.0, 1.0]`.
    threshold: f64,
}

impl PlaceMatcher {
    /// Build a matcher from the service [`MatchingConfig`].
    #[must_use]
    pub fn new(config: &MatchingConfig) -> Self {
        Self {
            weights: MatchWeights::default(),
            threshold: config.threshold_score,
        }
    }

    /// Score two places, returning the full [`MatchResult`](scoring::MatchResult).
    #[must_use]
    pub fn score(&self, a: &Place, b: &Place) -> MatchResult {
        compute_match(a, b, &self.weights)
    }

    /// Whether two places score at or above the configured threshold.
    #[must_use]
    pub fn is_match(&self, a: &Place, b: &Place) -> bool {
        self.score(a, b).score >= self.threshold
    }

    /// The configured `is_match` threshold.
    #[must_use]
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

/// Classify a confidence band into a stable string label for API responses.
#[must_use]
pub fn confidence_label(c: &MatchConfidence) -> &'static str {
    match c {
        MatchConfidence::Certain => "certain",
        MatchConfidence::Probable => "probable",
        MatchConfidence::Possible => "possible",
        MatchConfidence::Unlikely => "unlikely",
    }
}
