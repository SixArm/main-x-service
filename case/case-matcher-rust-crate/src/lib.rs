//! `case-matcher` — pairwise governmental case record matching.
//!
//! A *case* (case management / case tracking) is an open or historical
//! matter handled by a public agency on behalf of one or more subjects
//! — a benefit claim, legal action, social-services referral, licensing
//! application, complaint, appeal, and so on. The matcher takes two
//! `Case` records and produces a `MatchResult` with a score in
//! `[0.0, 1.0]`, a `Confidence` band, an `is_match` boolean, and a
//! per-component `MatchBreakdown`.
//!
//! ## Strategies
//!
//! - **Probabilistic** — weighted fuzzy score over title, subjects
//!   (Jaccard), agency-scoped case number, case type, status, and
//!   keywords; renormalised over the components both records carry.
//! - **Deterministic** — rule-based short-circuit: an exact match on a
//!   deterministic identifier scheme (`Docket`, `ExternalCaseId`, URI,
//!   UUID), a same-agency case-number match, or a `same_as` URL overlap
//!   pins the score to `1.0`.
//!
//! ## Usage
//!
//! ```
//! use case_matcher::{Case, MatchConfig, MatchingEngine};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let a = Case::new("Housing benefit appeal — J. Smith");
//! let b = Case::new("Housing benefit appeal — John Smith");
//! let result = engine.match_cases(&a, &b);
//! assert!(result.score >= 0.0 && result.score <= 1.0);
//! ```
//!
//! ## Public types
//!
//! - [`Case`], [`CaseIdentifier`], [`IdentifierScheme`], [`CaseType`],
//!   [`CaseStatus`], [`Priority`].
//! - [`MatchingEngine`], [`MatchConfig`], [`MatchResult`],
//!   [`MatchBreakdown`], [`Confidence`].

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod case;
pub mod config;
pub mod error;
pub mod matcher;
pub mod normalize;
pub mod phonetic;
pub mod scoring;

pub use case::{Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, Priority};
pub use config::MatchConfig;
pub use error::{Error, Result};
pub use matcher::MatchingEngine;
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
