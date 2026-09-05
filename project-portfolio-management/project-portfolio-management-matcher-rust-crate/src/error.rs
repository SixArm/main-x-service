//! Error type for the `project-portfolio-management-matcher` crate.

use thiserror::Error;

/// Errors produced by plan matching and normalization.
///
/// [`crate::MatchingEngine::match_plans`] and the component functions
/// stay total, returning a [`crate::MatchResult`] directly. The one
/// fallible entry point is [`crate::MatchConfig::validated`] (this
/// doc comment's own anticipated "future fallible path" — see
/// [`Error::InvalidConfig`]).
#[derive(Debug, Error)]
pub enum Error {
    /// A failure during the matching computation.
    #[error("Matching error: {0}")]
    Matching(String),
    /// A failure during input normalization.
    #[error("Normalization error: {0}")]
    Normalization(String),
    /// A hand-built `MatchConfig` failed
    /// [`crate::MatchConfig::validated`]: a weight or
    /// `timeframe_sigma_days` is negative, `NaN`, or infinite, or
    /// `threshold` is outside `[0.0, 1.0]`.
    #[error("Invalid match config: {0}")]
    InvalidConfig(String),
}

/// Convenience alias for results returning a crate [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;
