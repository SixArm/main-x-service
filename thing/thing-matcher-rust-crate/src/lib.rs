//! # Thing matcher
//!
//! A Rust library for matching records that describe `schema.org/Thing`
//! entities. The crate implements both **deterministic** and
//! **probabilistic** matching algorithms.
//!
//! The library is **deterministic**, **stateless**, **panic-free** in
//! library code, and **`Send + Sync`** so it can be used freely across
//! threads.
//!
//! ## What it does
//!
//! Given two [`Thing`] records — typically drawn from different source
//! systems — the [`MatchingEngine`] decides whether they refer to the
//! same item. The output is either a hard boolean (deterministic) or a
//! scored [`MatchResult`] with a per-field [`matcher::MatchBreakdown`] so
//! an auditor or downstream system can inspect the decision.
//!
//! The data model follows `schema.org/Thing` — the root type used to
//! describe any kind of item on the web. The crate compares the
//! identity-bearing properties of that vocabulary: `name`,
//! `alternateName`, `description`, `disambiguatingDescription`,
//! `identifier`, `url`, `image`, `sameAs`, `mainEntityOfPage`,
//! `additionalType`, `subjectOf`, and `owner`.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |---|---|
//! | [`models`]       | Data types: [`Thing`], [`ThingBuilder`], [`Identifier`]. |
//! | [`normalizer`]   | Text normalisation: names, free text, URLs, phonetic codes. |
//! | [`scorer`]       | String-similarity and set-similarity primitives. |
//! | [`matcher`]      | Orchestration: [`MatchingEngine`], [`MatchConfig`], [`MatchResult`]. |
//! | [`error`]        | Error enum [`MatchingError`] and [`Result`] alias. |
//!
//! ## Quick start — probabilistic match
//!
//! ```
//! use thing_matcher::{MatchingEngine, MatchConfig, Thing};
//!
//! let a = Thing::builder()
//!     .name("Eiffel Tower")
//!     .add_alternate_name("La Tour Eiffel")
//!     .url("https://www.toureiffel.paris/")
//!     .build();
//!
//! let b = Thing::builder()
//!     .name("Tour Eiffel")
//!     .url("https://www.toureiffel.paris/")
//!     .build();
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let result = engine.match_things(&a, &b);
//!
//! assert!(result.is_match);
//! ```
//!
//! ## Inspecting the per-field breakdown
//!
//! Every probabilistic match returns a per-field score so the decision is
//! auditable end-to-end. Missing or unparseable fields score `None`
//! rather than zero — they do not penalise the thing.
//!
//! ```
//! use thing_matcher::{MatchingEngine, Thing};
//!
//! let p = Thing::builder()
//!     .name("Big Ben")
//!     .url("https://en.wikipedia.org/wiki/Big_Ben")
//!     .build();
//! let q = p.clone();
//!
//! let result = MatchingEngine::default_config().match_things(&p, &q);
//! assert!(result.breakdown.name_score.unwrap() > 0.99);
//! assert_eq!(result.breakdown.url_score, Some(1.0));
//! ```
//!
//! ## Configuration presets
//!
//! Three configurations cover most use cases. Use [`MatchConfig::strict`]
//! when callers must rely on the answer; use [`MatchConfig::lenient`] to
//! triage large candidate sets where false negatives are worse than false
//! positives.
//!
//! ```
//! use thing_matcher::{MatchConfig, MatchingEngine};
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
//! - **Deterministic.** Same inputs => same outputs. No clocks, no RNGs,
//!   no environment variables.
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
pub use models::{Identifier, RelationKind, RelationshipRef, Thing, ThingBuilder};
pub use normalizer::Normalizer;
pub use scorer::{Scorer, SimilarityAlgorithm};
