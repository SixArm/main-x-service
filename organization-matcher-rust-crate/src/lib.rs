//! `organization-matcher` — pairwise organization-record matching.
//!
//! Models loosely follow `schema.org/Organization`. The matcher takes
//! two `Organization` records and produces a `MatchResult` with a score
//! in `[0.0, 1.0]`, a `Confidence` band, an `is_match` boolean, and a
//! per-component `MatchBreakdown`.
//!
//! ## Strategies
//!
//! - **Probabilistic** — weighted fuzzy score, renormalised over the
//!   components both records actually carry (no penalty for absent
//!   data): name (legal-suffix-aware Jaro-Winkler), postal address,
//!   url / domain, jurisdiction, founding date, keywords.
//! - **Deterministic** — rule-based short-circuit: an exact match on a
//!   deterministic identifier scheme (LEI, DUNS, ISO 6523, GLN,
//!   Wikidata, ROR, ISNI, VAT), a same-jurisdiction tax-id match, or a
//!   `same_as` URL overlap pins the score to `1.0`.
//!
//! ## Usage
//!
//! ```
//! use organization_matcher::{Organization, MatchConfig, MatchingEngine};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let a = Organization::new("Acme Corporation");
//! let b = Organization::new("Acme, Inc.");
//! let result = engine.match_organizations(&a, &b);
//! assert!(result.score >= 0.0 && result.score <= 1.0);
//! ```
//!
//! ## Public types
//!
//! - [`Organization`], [`OrgIdentifier`], [`IdentifierScheme`],
//!   [`PostalAddress`].
//! - [`MatchingEngine`], [`MatchConfig`], [`MatchResult`],
//!   [`MatchBreakdown`], [`Confidence`].

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod config;
pub mod error;
pub mod matcher;
pub mod normalize;
pub mod organization;
pub mod phonetic;
pub mod scoring;

pub use config::MatchConfig;
pub use error::{Error, Result};
pub use matcher::MatchingEngine;
pub use organization::{IdentifierScheme, OrgIdentifier, Organization, PostalAddress};
pub use scoring::{Confidence, MatchBreakdown, MatchResult};
