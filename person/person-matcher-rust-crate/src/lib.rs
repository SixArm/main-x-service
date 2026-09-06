//! # Person matcher
//!
//! A Rust library for matching person records in healthcare information
//! exchanges. The crate implements both **deterministic** and **probabilistic**
//! matching algorithms grounded in peer-reviewed research on person
//! identification (see [`spec/index.md`](https://github.com/sixarm/person-matcher-rust-crate/blob/main/spec/index.md) §5).
//!
//! The library is **deterministic**, **stateless**, **panic-free** in library
//! code, and **`Send + Sync`** so it can be used freely across threads.
//!
//! ## What it does
//!
//! Given two [`Person`] records — typically drawn from different source
//! systems — the [`MatchingEngine`] decides whether they refer to the same
//! human being. The output is either a hard boolean (deterministic) or a
//! scored [`MatchResult`] with a per-field [`matcher::MatchBreakdown`] so a
//! clinician or downstream system can audit the decision.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |---|---|
//! | [`models`]       | Data structures: [`Person`], [`PersonBuilder`], [`Address`], [`BloodType`], [`Gender`], [`PassportBook`], [`RelationshipRef`], [`RelationKind`]. |
//! | [`identifiers`]  | 42 national personal-identifier parsers + 9 passport-format validators (one parser per scheme; never cross-matched). |
//! | [`nicknames`]    | Opt-in [`NicknameTable`] that lifts the given-name score for known nickname classes (Michael ↔ Mike, …). |
//! | [`normalizer`]   | Text normalisation: names, postcodes, phone numbers, phonetic codes. |
//! | [`scorer`]       | String-similarity primitives: Jaro-Winkler, Levenshtein, exact, combined. |
//! | [`matcher`]      | Orchestration: [`MatchingEngine`], [`MatchConfig`], [`MatchResult`]. |
//! | [`error`]        | Error enum [`MatchingError`] and [`Result`] alias. |
//!
//! See [`agents/architecture.md`](https://github.com/sixarm/person-matcher-rust-crate/blob/main/agents/architecture.md)
//! for the layering rules.
//!
//! ## Quick start — probabilistic match
//!
//! ```
//! use person_matcher::{Gender, MatchingEngine, MatchConfig, Person};
//! use chrono::NaiveDate;
//!
//! let alice = Person::builder()
//!     .given_name("Alice")
//!     .family_name("Williams")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
//!     .gender(Gender::Female)
//!     .build();
//!
//! let alyce = Person::builder()
//!     .given_name("Alyce")   // alternate spelling
//!     .family_name("Williams")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
//!     .gender(Gender::Female)
//!     .build();
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let result = engine.match_persons(&alice, &alyce);
//!
//! assert!(result.is_match, "Alice and Alyce should be a fuzzy match");
//! assert!(result.score > 0.85);
//! ```
//!
//! ## Quick start — deterministic match
//!
//! ```
//! use person_matcher::{MatchingEngine, Person};
//!
//! // United Kingdom National Health Service Numbers in two textual layouts.
//! let a = Person::builder().united_kingdom_national_health_service_number("943 476 5919").build();
//! let b = Person::builder().united_kingdom_national_health_service_number("9434765919").build();
//!
//! let engine = MatchingEngine::default_config();
//! assert!(engine.deterministic_match(&a, &b),
//!     "same United Kingdom National Health Service Number with different formatting must match deterministically");
//! ```
//!
//! ## Inspecting the per-field breakdown
//!
//! Every probabilistic match returns a per-field score so the decision is
//! auditable end-to-end. Missing or unparseable fields score `None` rather
//! than zero — they do not penalise the person.
//!
//! ```
//! use person_matcher::{MatchingEngine, Person};
//! use chrono::NaiveDate;
//!
//! let p1 = Person::builder()
//!     .given_name("John")
//!     .family_name("Smith")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
//!     .build();
//! let p2 = p1.clone();
//!
//! let result = MatchingEngine::default_config().match_persons(&p1, &p2);
//!
//! assert_eq!(result.breakdown.date_of_birth_score, Some(1.0));
//! assert!(result.breakdown.given_name_score.unwrap() > 0.99);
//! assert!(result.breakdown.family_name_score.unwrap() > 0.99);
//! // United Kingdom National Health Service Number was missing on both —
//! // score is `None`, not `0.0`.
//! assert_eq!(result.breakdown.united_kingdom_national_health_service_number_score, None);
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
//! use person_matcher::{MatchConfig, MatchingEngine};
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
//! - [`spec/index.md`](https://github.com/sixarm/person-matcher-rust-crate/blob/main/spec/index.md) — the living specification.
//! - [`agents/matching-algorithm.md`](https://github.com/sixarm/person-matcher-rust-crate/blob/main/agents/matching-algorithm.md) — practitioner's view of the algorithm.
//! - [`agents/normalization.md`](https://github.com/sixarm/person-matcher-rust-crate/blob/main/agents/normalization.md) — text normalisation rules.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Pedantic sub-lints that are intentional design choices here, not defects:
#![allow(
    // Builder and parser methods deliberately omit `#[must_use]`; annotating
    // every one is noise on an already-large public surface.
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    // The per-field `score_*` methods form one consistent method API on the
    // engine even where an individual scorer needs no `&self` state.
    clippy::unused_self,
    // Identity field names are inherently paired (`person1`/`person2`,
    // `norm1`/`norm2`); the similarity is meaningful, not accidental.
    clippy::similar_names,
    // National-identifier parsers are flat match ladders that read best whole.
    clippy::too_many_lines,
    // Parser lookup tables are defined next to their single use site on purpose.
    clippy::items_after_statements,
    // Calendar/index arithmetic over small, bounded integers (month/day, digit
    // weights); the casts are in-range by construction.
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

pub mod error;
pub mod identifiers;
pub mod matcher;
pub mod models;
pub mod nicknames;
pub mod normalizer;
pub mod scorer;

pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchBreakdown, MatchConfig, MatchResult, MatchingEngine};
pub use models::{
    Address, BloodType, Gender, PassportBook, Person, PersonBuilder, RelationKind, RelationshipRef,
};
pub use nicknames::NicknameTable;
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
