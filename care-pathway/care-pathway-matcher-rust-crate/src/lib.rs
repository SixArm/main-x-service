//! `care-pathway-matcher` — pairwise care-pathway record matching.
//!
//! A *care pathway* (clinical / critical / integrated care pathway) is a
//! structured, evidence-based, multidisciplinary plan of care for a
//! specific clinical condition over a defined episode. The matcher takes
//! two `CarePathway` records and produces a `MatchResult` with a score
//! in `[0.0, 1.0]`, a `Confidence` band, an `is_match` boolean, and a
//! per-component `MatchBreakdown`.
//!
//! ## Strategies
//!
//! - **Probabilistic** — weighted fuzzy score over name, target
//!   condition codes (Jaccard), provider-scoped pathway code, care
//!   setting, interventions, and keywords; renormalised over the
//!   components both records carry.
//! - **Deterministic** — rule-based short-circuit: an exact match on a
//!   deterministic identifier scheme (DOI, Wikidata, `GuidelineId`, URI,
//!   UUID), a same-provider pathway-code match, or a `same_as` URL
//!   overlap pins the score to `1.0`.
//!
//! ## Usage
//!
//! ```
//! use care_pathway_matcher::{CarePathway, MatchConfig, MatchingEngine};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let a = CarePathway::new("Acute Stroke Care Pathway");
//! let b = CarePathway::new("Acute Stroke Pathway");
//! let result = engine.match_care_pathways(&a, &b);
//! assert!(result.score >= 0.0 && result.score <= 1.0);
//! ```
//!
//! ## Public types
//!
//! - [`CarePathway`], [`PathwayIdentifier`], [`IdentifierScheme`],
//!   [`ConditionCode`], [`CodeSystem`], [`CareSetting`].
//! - [`MatchingEngine`], [`MatchConfig`], [`MatchResult`],
//!   [`MatchBreakdown`], [`Confidence`].

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod care_pathway;
pub mod config;
pub mod error;
pub mod matcher;
pub mod normalize;
pub mod phonetic;
pub mod scoring;

pub use care_pathway::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, PathwayIdentifier,
};
pub use config::MatchConfig;
pub use error::{Error, Result};
pub use matcher::MatchingEngine;
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
