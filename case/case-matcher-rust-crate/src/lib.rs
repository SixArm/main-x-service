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
#![forbid(unsafe_code)] // pure library: no `unsafe` is ever needed.
#![deny(missing_docs)] // every public item must carry a doc comment.
#![warn(clippy::pedantic)] // opt in to Clippy's stricter lint set.

// ─── Modules ─────────────────────────────────────────────────────────
// The crate is split one concern per module; `lib.rs` re-exports the
// stable surface below so callers never depend on module paths.

/// Domain model: the [`Case`] record and its categorical enums.
pub mod case;
/// Tunable weights + threshold for the probabilistic strategy.
pub mod config;
/// Crate error type and its [`Result`] alias.
pub mod error;
/// The [`MatchingEngine`] entry point and the matching algorithm.
pub mod matcher;
/// Input normalisation helpers (fold / case-number / URL / set).
pub mod normalize;
/// Soundex phonetic encoder backing the title-component bonus.
pub mod phonetic;
/// Match-result shape and the renormalised weighted-average helper.
pub mod scoring;

// ─── Public re-exports ───────────────────────────────────────────────
// The flat, stable API surface. These are the only paths downstream
// crates should name; the module layout above is free to evolve.

pub use case::{Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, Priority};
pub use config::MatchConfig;
pub use error::{Error, Result};
pub use matcher::MatchingEngine;
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
