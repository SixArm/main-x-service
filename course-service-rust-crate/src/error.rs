//! Crate-level error type.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Connection pool error: {0}")]
    Pool(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Matching error: {0}")]
    Matching(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found")]
    NotFound,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Streaming error: {0}")]
    Streaming(String),
}

pub type Result<T> = std::result::Result<T, Error>;
