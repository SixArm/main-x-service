//! # Event matcher
//!
//! A Rust library for matching event records following the
//! [schema.org/Event](https://schema.org/Event) data model. The crate
//! implements both **deterministic** and **probabilistic** matching
//! algorithms.
//!
//! The library is **deterministic**, **stateless**, **panic-free** in
//! library code, and **`Send + Sync`** so it can be used freely across
//! threads.
//!
//! ## What it does
//!
//! Given two [`Event`] records — typically drawn from different source
//! systems — the [`MatchingEngine`] decides whether they refer to the
//! same event. The output is either a hard boolean (deterministic) or a
//! scored [`MatchResult`] with a per-field [`matcher::MatchBreakdown`] so
//! an auditor or downstream system can inspect the decision.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |---|---|
//! | [`models`]       | Data types: [`Event`], [`EventBuilder`], [`Address`], [`Location`], [`EventCategory`], [`EventStatus`], [`EventAttendanceMode`], [`EventId`], [`EventIdScheme`]. |
//! | [`normalizer`]   | Text and ISO 8601 normalisation: names, postcodes, addresses, phonetic codes, date-times. |
//! | [`scorer`]       | String, geographic, and temporal similarity primitives (Jaro-Winkler, Levenshtein, Haversine, Gaussian decay). |
//! | [`matcher`]      | Orchestration: [`MatchingEngine`], [`MatchConfig`], [`MatchResult`]. |
//! | [`error`]        | Error enum [`MatchingError`] and [`Result`] alias. |
//!
//! ## Quick start — probabilistic match
//!
//! ```
//! use event_matcher::{MatchingEngine, MatchConfig, Event};
//!
//! let a = Event::builder()
//!     .name("Glastonbury Festival 2024")
//!     .add_alternate_name("Glasto 2024")
//!     .start_date("2024-06-26T09:00:00Z")
//!     .end_date("2024-06-30T23:59:00Z")
//!     .build();
//!
//! let b = Event::builder()
//!     .name("Glasto 2024")
//!     .start_date("2024-06-26T09:15:00Z")
//!     .end_date("2024-06-30T23:59:00Z")
//!     .build();
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let result = engine.match_events(&a, &b);
//!
//! assert!(result.is_match);
//! ```
//!
//! ## Inspecting the per-field breakdown
//!
//! Every probabilistic match returns a per-field score so the decision is
//! auditable end-to-end. Missing or unparseable fields score `None` rather
//! than zero — they do not penalise the event.
//!
//! ```
//! use event_matcher::{MatchingEngine, Event};
//!
//! let p = Event::builder()
//!     .name("RustConf 2024")
//!     .start_date("2024-09-10T09:00:00Z")
//!     .build();
//! let q = p.clone();
//!
//! let result = MatchingEngine::default_config().match_events(&p, &q);
//! assert!(result.breakdown.name_score.unwrap() > 0.99);
//! assert!(result.breakdown.start_date_score.unwrap() > 0.99);
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
//! use event_matcher::{MatchConfig, MatchingEngine};
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

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod matcher;
pub mod models;
pub mod normalizer;
pub mod scorer;

pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{
    Address, Event, EventAttendanceMode, EventBuilder, EventCategory, EventId, EventIdScheme,
    EventStatus, Location,
};
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
