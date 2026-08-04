//! `project-portfolio-management-matcher` — pairwise plan record matching.
//!
//! A *plan* is a named unit of intended work in a portfolio /
//! project-management registry, forming one recursive tree: any work
//! item may contain any other via `parent_ref`. The former `Portfolio` /
//! `Project` / `Product` / `Program` kinds were unified into this single
//! type; `kind` survives only as **optional descriptive metadata** and
//! no longer gates matching. The matcher takes two [`Plan`] records
//! and produces a [`MatchResult`] with a score in `[0.0, 1.0]`, a
//! [`Confidence`] band, an `is_match` boolean, and a per-component
//! [`MatchBreakdown`].
//!
//! ## No kind gate
//!
//! Because the kinds were unified, **any two plans may match**
//! regardless of their (optional) `kind`. The `MatchBreakdown` retains a
//! vestigial `kind_gate_blocked` field, now always `false`.
//!
//! ## Strategies
//!
//! - **Deterministic** — rule-based short-circuit: an exact match on a
//!   deterministic identifier scheme (the PM-tool/registry ids plus URI /
//!   UUID), a same-owner `code` match, or a `same_as` URL overlap pins
//!   the score to `1.0`.
//! - **Probabilistic** — a weighted fuzzy score over name (Jaro-Winkler +
//!   Soundex), goal titles (Jaccard), owner-scoped code, owner org,
//!   parent plan (`parent_ref`), timeframe (Gaussian date proximity),
//!   keywords, relationships (typed-set Jaccard), and tags; renormalised
//!   over the components both records carry.
//!
//! ## Usage
//!
//! ```
//! use project_portfolio_management_matcher::{Plan, MatchConfig, MatchingEngine};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let a = Plan::new("Apollo platform migration");
//! let b = Plan::new("Apollo platform migrate");
//! let result = engine.match_plans(&a, &b);
//! assert!(result.score >= 0.0 && result.score <= 1.0);
//! ```
//!
//! ## Public types
//!
//! - [`Plan`], [`PlanKind`], [`PlanStatus`], [`Goal`],
//!   [`GoalStatus`], [`PlanIdentifier`], [`IdentifierScheme`],
//!   [`PlanRelationship`], [`RelationKind`].
//! - [`MatchingEngine`], [`MatchConfig`], [`MatchResult`],
//!   [`MatchBreakdown`], [`Confidence`].

// Always start with high quality coding conventions.
#![forbid(unsafe_code)] // pure library: no `unsafe` is ever needed.
#![deny(missing_docs)] // every public item must carry a doc comment.
#![warn(clippy::pedantic)] // opt in to Clippy's stricter lint set.

// ─── Modules ─────────────────────────────────────────────────────────
// One concern per module; `lib.rs` re-exports the stable surface below so
// callers never depend on module paths.

/// Tunable weights, timeframe σ, and threshold for the probabilistic
/// strategy.
pub mod config;
/// Crate error type and its [`Result`] alias.
pub mod error;
/// The [`MatchingEngine`] entry point and the matching algorithm.
pub mod matcher;
/// Input normalisation helpers (fold / code / URL / set / ISO date).
pub mod normalize;
/// Soundex phonetic encoder backing the name-component bonus.
pub mod phonetic;
/// Domain model: the [`Plan`] record and its categorical enums.
pub mod plan;
/// Match-result shape and the renormalised weighted-average helper.
pub mod scoring;

// ─── Public re-exports ───────────────────────────────────────────────
// The flat, stable API surface. These are the only paths downstream
// crates should name; the module layout above is free to evolve.

pub use config::MatchConfig;
pub use error::{Error, Result};
pub use matcher::MatchingEngine;
pub use plan::{
    Goal, GoalStatus, IdentifierScheme, Plan, PlanIdentifier, PlanKind, PlanRelationship,
    PlanStatus, RelationKind,
};
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
