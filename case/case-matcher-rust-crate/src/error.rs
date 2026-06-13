//! Error type for the `case-matcher` crate.

use thiserror::Error;

/// Errors produced by case matching and normalization.
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
