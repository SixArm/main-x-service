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
//!   [`ConditionCode`], [`CodeSystem`], [`CareSetting`],
//!   [`RelationshipRef`], [`RelationKind`].
//! - [`MatchingEngine`], [`MatchConfig`], [`MatchResult`],
//!   [`MatchBreakdown`], [`Confidence`].
//! - [`Error`], [`Result`] — the crate error type and its `Result` alias.

// Always start with high quality coding conventions.
// `forbid(unsafe_code)` — this is a pure, deterministic library; there
// is no reason to reach for `unsafe`, so forbid it outright.
#![forbid(unsafe_code)]
// `deny(missing_docs)` — every public item must carry a `///` doc;
// a missing doc is a hard compile error, which keeps the surface
// self-describing.
#![deny(missing_docs)]
// `warn(clippy::pedantic)` — opt into Clippy's stricter lint set so the
// crate stays idiomatic; warnings (not errors) keep iteration friction low.
#![warn(clippy::pedantic)]

// ── Module tree ──────────────────────────────────────────────────
// Each module owns one concern; `lib.rs` only declares them and curates
// the public re-export surface below.
pub mod care_pathway; // domain model (`CarePathway` and its value types)
pub mod config; // tunable weights + threshold (`MatchConfig`)
pub mod error; // crate error type + `Result` alias
pub mod matcher; // the `MatchingEngine` and scoring pipeline
pub mod normalize; // case-fold / NFKC / pathway-code helpers
pub mod phonetic; // Soundex encoder (name-component bonus)
pub mod scoring; // result shape + renormalised weighted average

// ── Curated public surface ───────────────────────────────────────
// Re-export the types a downstream caller needs so they can be reached
// directly as `care_pathway_matcher::Foo` without knowing the module
// layout. Grouped by concern for readability.

// Domain model — the input records and their value types.
pub use care_pathway::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, PathwayIdentifier,
    RelationKind, RelationshipRef,
};
// Configuration — weights and probable-match threshold.
pub use config::MatchConfig;
// Error handling — typed error and the crate-local `Result` alias.
pub use error::{Error, Result};
// Engine — the entry point that scores pairs.
pub use matcher::MatchingEngine;
// Result — the scored outcome and its per-component breakdown.
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
