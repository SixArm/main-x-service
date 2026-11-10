//! The crate-wide error type and [`Result`] alias.
//!
//! Every fallible operation in the service returns
//! [`Result<T>`](Result) = `std::result::Result<T, Error>`. [`Error`] is
//! a flat `thiserror` enum with one variant per failure domain; the API
//! layer maps each variant onto an HTTP status code. Only
//! [`sqlx::Error`] converts with `?` (into [`Error::Database`]); the rest
//! are constructed explicitly (often via the
//! [`database`](Error::database) / [`validation`](Error::validation) /
//! [`internal`](Error::internal) helpers) so the call site stays the
//! source of truth for the message.

use thiserror::Error;

/// Convenience alias for `Result<T, Error>` used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The unified error type for the Person Service.
///
/// Each variant carries a human-readable message. The `Display` text
/// (from the `#[error(...)]` attributes) is what surfaces in API error
/// responses and logs.
#[derive(Error, Debug)]
pub enum Error {
    /// A database failure, carrying the driver's message. Auto-converts
    /// from [`sqlx::Error`].
    #[error("Database error: {0}")]
    Database(String),

    /// A connection-pool failure (e.g. could not acquire a connection).
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// A full-text search / Tantivy failure.
    #[error("Search error: {0}")]
    Search(String),

    /// The requested person could not be found.
    #[error("Person not found: {0}")]
    PersonNotFound(String),

    /// Input data failed validation.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Another record already holds a value that must be unique, such as
    /// an identifier's `(system, value)`. Maps to HTTP `409 Conflict`.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// A failure inside the matching engine.
    #[error("Matching error: {0}")]
    Matching(String),

    /// A generic API-layer failure (binding, serialization, …).
    #[error("API error: {0}")]
    Api(String),

    /// Invalid or missing configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// An event-streaming / publisher failure.
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// A FHIR conversion or resource-handling failure.
    #[error("FHIR error: {0}")]
    Fhir(String),

    /// An unexpected internal error that fits no other variant.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Build a [`Database`](Error::Database) error from a custom message,
    /// prefixed `Custom Error: `, the wording callers and logs already
    /// match on.
    pub fn database(msg: impl Into<String>) -> Self {
        Error::Database(format!("Custom Error: {}", msg.into()))
    }

    /// Build a [`Validation`](Error::Validation) error from a message.
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Build an [`Internal`](Error::Internal) error from a message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Error::Internal(msg.into())
    }
}

/// Every sqlx failure is an [`Error::Database`], carrying sqlx's own
/// message, so `?` on a query reads the same everywhere in `src/db/`.
impl From<sqlx::Error> for Error {
    /// A unique-constraint violation becomes [`Error::Conflict`] (`409`),
    /// as the family's errors require; it was a `500`. Every other database
    /// error stays [`Error::Database`].
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &e
            && db.is_unique_violation()
        {
            return Self::Conflict(format!(
                "another person already holds this value ({})",
                db.constraint().unwrap_or("unique constraint")
            ));
        }
        Self::Database(e.to_string())
    }
}
