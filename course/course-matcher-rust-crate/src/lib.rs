//! `course-matcher` — pairwise course-record matching.
//!
//! Models loosely follow `schema.org/Course`. The matcher takes two
//! `Course` records and produces a `MatchResult` with a score in
//! `[0.0, 1.0]`, a `Confidence` band, a `is_match` boolean, and a
//! per-component `MatchBreakdown`.
//!
//! ## Strategies
//!
//! - **Probabilistic** — weighted fuzzy score, renormalised over the
//!   components both records actually carry (no penalty for absent
//!   data).
//! - **Deterministic** — rule-based short-circuit: an exact match on a
//!   deterministic identifier scheme (DOI, Wikidata, LOM, OER, URI,
//!   UUID), or a same-provider course-code match, pins the score to
//!   `1.0`.
//!
//! ## Usage
//!
//! ```
//! use course_matcher::{Course, MatchConfig, MatchingEngine};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let a = Course::new("Introduction to Computer Science");
//! let b = Course::new("Intro to Computer Science");
//! let result = engine.match_courses(&a, &b);
//! assert!(result.score >= 0.0 && result.score <= 1.0);
//! ```
//!
//! ## Public types
//!
//! - [`Course`], [`CourseIdentifier`], [`IdentifierScheme`],
//!   [`EducationalLevel`], [`LearningResourceType`].
//! - [`MatchingEngine`], [`MatchConfig`], [`MatchResult`],
//!   [`MatchBreakdown`], [`Confidence`].
//! - [`Evidence`], [`Contradiction`], [`MatchStrength`] — how much the
//!   compared data *proves*, beside the similarity score (spec §18a).

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod config;
pub mod course;
pub mod error;
pub mod evidence;
pub mod matcher;
pub mod normalize;
pub mod phonetic;
pub mod scoring;

pub use config::MatchConfig;
pub use course::{
    Course, CourseIdentifier, EducationalLevel, IdentifierScheme, LearningResourceType,
    RelationKind, RelationshipRef,
};
pub use error::{Error, Result};
pub use evidence::{
    ComponentBits, Contradiction, Evidence, FrequencyTable, MatchStrength, NoFrequencies,
    ValueFrequencies,
};
pub use matcher::MatchingEngine;
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
