//! # Worker matcher
//!
//! A Rust library for matching worker records in healthcare information
//! exchanges. The crate implements both **deterministic** and **probabilistic**
//! matching algorithms grounded in peer-reviewed research on worker
//! identification (see [`spec.md`](https://github.com/sixarm/worker-matcher/blob/main/spec.md) §5).
//!
//! The library is **deterministic**, **stateless**, **panic-free** in library
//! code, and **`Send + Sync`** so it can be used freely across threads.
//!
//! ## What it does
//!
//! Given two [`Worker`] records — typically drawn from different source
//! systems — the [`MatchingEngine`] decides whether they refer to the same
//! human being. The output is either a hard boolean (deterministic) or a
//! scored [`MatchResult`] with a per-field [`matcher::MatchBreakdown`] so a
//! clinician or downstream system can audit the decision.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |---|---|
//! | [`models`]       | Data structures: [`Worker`], [`WorkerBuilder`], [`Address`], [`Gender`]. |
//! | [`identifiers`]  | National healthcare identifier parsers — UK NHS, FR NIR, ES TSI, IE IHI, UK H&C. |
//! | [`normalizer`]   | Text normalisation: names, postcodes, phone numbers, phonetic codes. |
//! | [`scorer`]       | String-similarity primitives: Jaro-Winkler, Levenshtein, exact, combined. |
//! | [`matcher`]      | Orchestration: [`MatchingEngine`], [`MatchConfig`], [`MatchResult`]. |
//! | [`error`]        | Error enum [`MatchingError`] and [`Result`] alias. |
//!
//! See [`AGENTS/architecture.md`](https://github.com/sixarm/worker-matcher/blob/main/AGENTS/architecture.md)
//! for the layering rules.
//!
//! ## Quick start — probabilistic match
//!
//! ```
//! use worker_matcher::{Gender, MatchingEngine, MatchConfig, Worker};
//! use jiff::civil::Date;
//!
//! let alice = Worker::builder()
//!     .given_name("Alice")
//!     .family_name("Williams")
//!     .date_of_birth(jiff::civil::date(1980, 5, 15))
//!     .gender(Gender::Female)
//!     .build();
//!
//! let alyce = Worker::builder()
//!     .given_name("Alyce")   // alternate spelling
//!     .family_name("Williams")
//!     .date_of_birth(jiff::civil::date(1980, 5, 15))
//!     .gender(Gender::Female)
//!     .build();
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let result = engine.match_workers(&alice, &alyce);
//!
//! assert!(result.is_match, "Alice and Alyce should be a fuzzy match");
//! assert!(result.score > 0.85);
//! ```
//!
//! ## Quick start — deterministic match
//!
//! ```
//! use worker_matcher::{MatchingEngine, Worker};
//!
//! // NHS-format numbers in two textual layouts.
//! let a = Worker::builder().uk_nhs_number("943 476 5919").build();
//! let b = Worker::builder().uk_nhs_number("9434765919").build();
//!
//! let engine = MatchingEngine::default_config();
//! assert!(engine.deterministic_match(&a, &b),
//!     "same NHS number with different formatting must match deterministically");
//! ```
//!
//! ## Inspecting the per-field breakdown
//!
//! Every probabilistic match returns a per-field score so the decision is
//! auditable end-to-end. Missing or unparseable fields score `None` rather
//! than zero — they do not penalise the worker.
//!
//! ```
//! use worker_matcher::{MatchingEngine, Worker};
//! use jiff::civil::Date;
//!
//! let p1 = Worker::builder()
//!     .given_name("John")
//!     .family_name("Smith")
//!     .date_of_birth(jiff::civil::date(1980, 5, 15))
//!     .build();
//! let p2 = p1.clone();
//!
//! let result = MatchingEngine::default_config().match_workers(&p1, &p2);
//!
//! assert_eq!(result.breakdown.date_of_birth_score, Some(1.0));
//! assert!(result.breakdown.given_name_score.unwrap() > 0.99);
//! assert!(result.breakdown.family_name_score.unwrap() > 0.99);
//! // NHS number was missing on both — score is `None`, not `0.0`.
//! assert_eq!(result.breakdown.uk_nhs_number_score, None);
//! ```
//!
//! ## Configuration presets
//!
//! Three configurations cover most use cases. Use [`MatchConfig::strict`]
//! when a clinician must rely on the answer; use [`MatchConfig::lenient`]
//! to triage large candidate sets where false negatives are worse than
//! false positives.
//!
//! ```
//! use worker_matcher::{MatchConfig, MatchingEngine};
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
//! - **Deterministic.** Same inputs ⇒ same outputs. No clocks, no RNGs, no
//!   environment variables.
//! - **No `unsafe`.** This is a clinical-adjacent library.
//! - **No IO.** The library does not log, read files, or open sockets.
//! - **No panics** in library code paths; every fallible input returns
//!   `None` from a scorer or a [`MatchingError`].
//!
//! ## Further reading
//!
//! - [`spec.md`](https://github.com/sixarm/worker-matcher/blob/main/spec.md) — the living specification.
//! - [`AGENTS/matching-algorithm.md`](https://github.com/sixarm/worker-matcher/blob/main/AGENTS/matching-algorithm.md) — practitioner's view of the algorithm.
//! - [`AGENTS/normalization.md`](https://github.com/sixarm/worker-matcher/blob/main/AGENTS/normalization.md) — text normalisation rules.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod error;
pub mod identifiers;
pub mod matcher;
pub mod models;
pub mod nicknames;
pub mod normalizer;
pub mod scorer;

pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{Address, BloodType, Gender, PassportBook, Worker, WorkerBuilder};
pub use nicknames::NicknameTable;
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
