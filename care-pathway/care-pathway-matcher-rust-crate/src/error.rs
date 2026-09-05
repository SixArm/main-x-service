//! Error type for the `care-pathway-matcher` crate.

use thiserror::Error;

/// Errors produced by care-pathway matching and normalization.
#[derive(Debug, Error)]
pub enum Error {
    /// A failure during the matching computation.
    #[error("Matching error: {0}")]
    Matching(String),
    /// A failure during input normalization.
    #[error("Normalization error: {0}")]
    Normalization(String),
    /// A [`crate::MatchConfig`] failed [`crate::MatchConfig::validated`]
    /// (CPM-T1): a negative/`NaN`/infinite weight, or a threshold
    /// outside `[0.0, 1.0]`.
    #[error("Invalid match config: {0}")]
    InvalidConfig(String),
}

/// Convenience alias for results returning a crate [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;
