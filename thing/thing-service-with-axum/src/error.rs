//! Crate-level error type.
//!
//! [`enum@Error`] is the single error enum threaded through every fallible
//! path in the service tier. The REST layer maps each variant to an HTTP
//! status (`NotFound` → 404, `Validation` → 422, `Conflict` → 409, else 500).

use thiserror::Error;

/// All errors the Thing Service can raise.
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

    /// A Tantivy index operation failed.
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

/// Every sqlx failure is a [`Error::Database`], carrying sqlx's own
/// message, so `?` on a query reads the same everywhere in `src/db/`.
///
/// A violated unique constraint is not singled out: it is a database
/// error like any other, exactly as it was under `SeaORM`. A caller that
/// wants it as a `409` checks it first with `axa::sqlx::is_unique_violation`.
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

/// Crate-wide convenience alias for `std::result::Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;
