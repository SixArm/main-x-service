//! Error type for the `portfolio-matcher` crate.

use thiserror::Error;

/// Errors produced by work-item matching and normalization.
///
/// Reserved for future fallible APIs: every current entry point
/// ([`crate::MatchingEngine::match_work_items`] and the component
/// functions) is total and returns a [`crate::MatchResult`] directly, so
/// nothing constructs an `Error` today. The type stays part of the
/// `SemVer` surface so a future fallible path (e.g. validated
/// construction) can be added without a breaking re-export.
#[derive(Debug, Error)]
pub enum Error {
    /// A failure during the matching computation.
    #[error("Matching error: {0}")]
    Matching(String),
    /// A failure during input normalization.
    #[error("Normalization error: {0}")]
    Normalization(String),
}

/// Convenience alias for results returning a crate [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;
