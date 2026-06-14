//! Error types for the event-service crate.
//!
//! [`Error`] is the single `thiserror`-derived error enum used across
//! every layer (DB, search, matching, API, …). [`Result`] is the crate
//! local alias used by nearly every fallible function.
//!
//! # Examples
//!
//! ```
//! use event_service::error::Error;
//!
//! let e = Error::validation("name is required");
//! assert_eq!(e.to_string(), "Validation error: name is required");
//! ```

use thiserror::Error;

/// Convenience alias for `Result<T, event_service::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// The crate-wide error type. Each variant maps to a distinct failure
/// domain and carries a human-readable message (or the underlying
/// source error, for [`Error::Database`]).
#[derive(Error, Debug)]
pub enum Error {
    /// A `SeaORM` database operation failed (wraps the source error).
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// A connection-pool acquisition or lifecycle failure.
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// A Tantivy search index or query failure.
    #[error("Search error: {0}")]
    Search(String),

    /// No event was found for the given identifier.
    #[error("Event not found: {0}")]
    EventNotFound(String),

    /// Input failed a data-quality / validation rule (maps to `422`).
    #[error("Validation error: {0}")]
    Validation(String),

    /// A failure inside the matching engine.
    #[error("Matching error: {0}")]
    Matching(String),

    /// A generic API-layer failure (bad request, serialization, …).
    #[error("API error: {0}")]
    Api(String),

    /// Invalid or missing configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// An event-streaming publish/consume failure.
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// A failure in the (stubbed) FHIR layer.
    #[error("FHIR error: {0}")]
    Fhir(String),

    /// An unexpected internal error not covered by the other variants.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Build a [`Error::Database`] from a free-text message (wraps it in
    /// `DbErr::Custom`).
    pub fn database(msg: impl Into<String>) -> Self {
        Error::Database(sea_orm::DbErr::Custom(msg.into()))
    }

    /// Build a [`Error::Validation`] from a free-text message.
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Build a [`Error::Internal`] from a free-text message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Error::Internal(msg.into())
    }
}
