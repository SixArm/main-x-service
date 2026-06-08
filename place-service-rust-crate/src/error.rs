//! Crate-level error type.
//!
//! [`enum@Error`] is the single error enum threaded through every fallible
//! path in the service tier, paired with the [`Result`] alias. The REST
//! layer maps each variant to an HTTP status code in
//! `crate::api::rest::handlers` (`NotFound` → 404, `Validation` → 422,
//! `Conflict` → 409, everything else → 500).

use thiserror::Error;

/// All errors the Place Service can raise.
#[derive(Debug, Error)]
pub enum Error {
    /// Configuration could not be loaded or a value failed to parse.
    #[error("Configuration error: {0}")]
    Config(String),

    /// A database query, transaction, or JSONB (de)serialization failed.
    #[error("Database error: {0}")]
    Database(String),

    /// The connection pool could not be opened.
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// A Tantivy index operation (open, write, query, reload) failed.
    #[error("Search error: {0}")]
    Search(String),

    /// The matching layer failed.
    #[error("Matching error: {0}")]
    Matching(String),

    /// A record failed data-quality validation. Maps to HTTP 422.
    #[error("Validation error: {0}")]
    Validation(String),

    /// The requested record does not exist (or is soft-deleted). Maps to 404.
    #[error("Not found")]
    NotFound,

    /// The request conflicts with existing state (e.g. a probable duplicate).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// A transport-layer failure in the REST server (bind, serve).
    #[error("API error: {0}")]
    Api(String),

    /// The event-streaming publisher failed to emit an event.
    #[error("Streaming error: {0}")]
    Streaming(String),
}

/// Crate-wide convenience alias for `std::result::Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;
