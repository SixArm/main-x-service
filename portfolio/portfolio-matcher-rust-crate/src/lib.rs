//! `portfolio-matcher` — pairwise work-item record matching.
//!
//! A *work item* is a named unit of intended work in a portfolio /
//! project-management registry. It comes in four **kinds**: a
//! `Portfolio` (the umbrella container) or a `Project` / `Product` /
//! `Program` that sits under a portfolio. The matcher takes two
//! [`WorkItem`] records and produces a [`MatchResult`] with a score in
//! `[0.0, 1.0]`, a [`Confidence`] band, an `is_match` boolean, and a
//! per-component [`MatchBreakdown`].
//!
//! ## The kind gate
//!
//! The four kinds are **distinct record types** in distinct
//! collections, so matching is **within a kind only**: comparing two
//! records of different kind short-circuits to `0.0` with
//! `breakdown.kind_gate_blocked = true`, before any other rule. A project
//! and a product are never the same identity.
//!
//! ## Strategies
//!
//! - **Deterministic** — rule-based short-circuit: an exact match on a
//!   deterministic identifier scheme (the PM-tool/registry ids plus URI /
//!   UUID), a same-owner `code` match, or a `same_as` URL overlap pins
//!   the score to `1.0`.
//! - **Probabilistic** — a weighted fuzzy score over name (Jaro-Winkler +
//!   Soundex), goal titles (Jaccard), owner-scoped code, owner org,
//!   parent portfolio, timeframe (Gaussian date proximity), keywords,
//!   relationships (typed-set Jaccard), and tags; renormalised over the
//!   components both records carry.
//!
//! ## Usage
//!
//! ```
//! use portfolio_matcher::{WorkItem, WorkItemKind, MatchConfig, MatchingEngine};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let a = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
//! let b = WorkItem::new(WorkItemKind::Project, "Apollo platform migrate");
//! let result = engine.match_work_items(&a, &b);
//! assert!(result.score >= 0.0 && result.score <= 1.0);
//! ```
//!
//! ## Public types
//!
//! - [`WorkItem`], [`WorkItemKind`], [`WorkItemStatus`], [`Goal`],
//!   [`GoalStatus`], [`WorkItemIdentifier`], [`IdentifierScheme`],
//!   [`WorkItemRelationship`], [`RelationKind`].
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
/// Match-result shape and the renormalised weighted-average helper.
pub mod scoring;
/// Domain model: the [`WorkItem`] record and its categorical enums.
pub mod work_item;

// ─── Public re-exports ───────────────────────────────────────────────
// The flat, stable API surface. These are the only paths downstream
// crates should name; the module layout above is free to evolve.

pub use config::MatchConfig;
pub use error::{Error, Result};
pub use matcher::MatchingEngine;
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
pub use work_item::{
    Goal, GoalStatus, IdentifierScheme, RelationKind, WorkItem, WorkItemIdentifier, WorkItemKind,
    WorkItemRelationship, WorkItemStatus,
};
