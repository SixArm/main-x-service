//! # Place matcher
//!
//! A Rust library for matching geographic-place records. The crate implements
//! both **deterministic** and **probabilistic** matching algorithms.
//!
//! The library is **deterministic**, **stateless**, **panic-free** in library
//! code, and **`Send + Sync`** so it can be used freely across threads.
//!
//! ## What it does
//!
//! Given two [`Place`] records — typically drawn from different source
//! systems — the [`MatchingEngine`] decides whether they refer to the same
//! place. The output is either a hard boolean (deterministic) or a scored
//! [`MatchResult`] with a per-field [`matcher::MatchBreakdown`] so an
//! auditor or downstream system can inspect the decision.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |---|---|
//! | [`models`]       | Data types: [`Place`], [`PlaceBuilder`], [`Address`], [`PlaceCategory`], [`PlaceId`]. |
//! | [`normalizer`]   | Text normalisation: names, postcodes, phone numbers, phonetic codes. |
//! | [`scorer`]       | String-similarity and geographic primitives (Jaro-Winkler, Levenshtein, Haversine, Gaussian-decay). |
//! | [`matcher`]      | Orchestration: [`MatchingEngine`], [`MatchConfig`], [`MatchResult`]. |
//! | [`error`]        | Error enum [`MatchingError`] and [`Result`] alias. |
//!
//! ## Quick start — probabilistic match
//!
//! ```
//! use place_matcher::{MatchingEngine, MatchConfig, Place};
//!
//! let a = Place::builder()
//!     .name("Eiffel Tower")
//!     .add_alternate_name("La Tour Eiffel")
//!     .latitude(48.858_222)
//!     .longitude(2.294_500)
//!     .build();
//!
//! let b = Place::builder()
//!     .name("Tour Eiffel")
//!     .latitude(48.858_3)
//!     .longitude(2.294_5)
//!     .build();
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let result = engine.match_places(&a, &b);
//!
//! assert!(result.is_match);
//! ```
//!
//! ## Inspecting the per-field breakdown
//!
//! Every probabilistic match returns a per-field score so the decision is
//! auditable end-to-end. Missing or unparseable fields score `None` rather
//! than zero — they do not penalise the place.
//!
//! ```
//! use place_matcher::{MatchingEngine, Place};
//!
//! let p = Place::builder()
//!     .name("Big Ben")
//!     .latitude(51.500_7)
//!     .longitude(-0.124_7)
//!     .build();
//! let q = p.clone();
//!
//! let result = MatchingEngine::default_config().match_places(&p, &q);
//! assert!(result.breakdown.name_score.unwrap() > 0.99);
//! assert!(result.breakdown.coordinates_score.unwrap() > 0.99);
//! ```
//!
//! ## Configuration presets
//!
//! Three configurations cover most use cases. Use [`MatchConfig::strict`]
//! when callers must rely on the answer; use [`MatchConfig::lenient`]
//! to triage large candidate sets where false negatives are worse than
//! false positives.
//!
//! ```
//! use place_matcher::{MatchConfig, MatchingEngine};
//!
//! let strict   = MatchingEngine::new(MatchConfig::strict());
//! let default  = MatchingEngine::default_config();
//! let lenient  = MatchingEngine::new(MatchConfig::lenient());
//!
//! // All three engines share the same scoring pipeline; only the
//! // threshold and a couple of weights differ.
//! # let _ = (strict, default, lenient);
//! ```
//!
//! ## Determinism and safety
//!
//! - **Deterministic.** Same inputs => same outputs. No clocks, no RNGs, no
//!   environment variables.
//! - **No `unsafe`.** This crate forbids `unsafe` code.
//! - **No IO.** The library does not log, read files, or open sockets.
//! - **No panics** in library code paths; every fallible input returns
//!   `None` from a scorer or a [`MatchingError`].

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod error;
pub mod matcher;
pub mod models;
pub mod normalizer;
pub mod scorer;

pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{Address, Place, PlaceBuilder, PlaceCategory, PlaceId, PlaceIdScheme};
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
