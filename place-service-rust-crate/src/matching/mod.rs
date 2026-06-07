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
//! For the algorithm contract see `AGENTS/matching.md` and
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
