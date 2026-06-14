//! Error types for the Worker Service (MPI).
//!
//! Defines the crate-wide [`Error`] enum and the [`Result`] alias used as the
//! return type throughout the service. Each variant maps to a layer of the
//! system (database, search, validation, API, …); the REST layer converts
//! these into appropriate HTTP status codes.

use thiserror::Error;

/// Convenience alias for `std::result::Result<T, Error>` used pervasively
/// across the crate so call sites can write `Result<Worker>` and so on.
pub type Result<T> = std::result::Result<T, Error>;

/// The crate-wide error type. Every variant carries a human-readable message
/// (via `thiserror`'s `Display`), and [`Database`](Error::Database) also wraps
/// the underlying SeaORM error for source-chain inspection.
#[derive(Error, Debug)]
pub enum Error {
    /// A database operation failed; wraps the SeaORM error verbatim. The
    /// REST layer maps this to HTTP `500 Internal Server Error`.
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// A connection-pool operation failed (e.g. acquisition timeout).
    /// Maps to HTTP `500 Internal Server Error`.
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// A search-index (Tantivy) operation failed. Maps to HTTP `500
    /// Internal Server Error`.
    #[error("Search error: {0}")]
    Search(String),

    /// The requested worker does not exist; carries the lookup key. Maps to
    /// HTTP `404 Not Found`.
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),

    /// Input failed data-quality validation. Maps to HTTP `422
    /// Unprocessable Entity`.
    #[error("Validation error: {0}")]
    Validation(String),

    /// A matching/scoring operation failed. Maps to HTTP `500 Internal
    /// Server Error`.
    #[error("Matching error: {0}")]
    Matching(String),

    /// A generic API-layer error (bad request, serialization, …). Maps to
    /// HTTP `400 Bad Request`.
    #[error("API error: {0}")]
    Api(String),

    /// Configuration could not be loaded or was invalid. Maps to HTTP `500
    /// Internal Server Error`.
    #[error("Configuration error: {0}")]
    Config(String),

    /// An event-streaming publish/consume operation failed. Maps to HTTP
    /// `500 Internal Server Error`.
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// A FHIR conversion or resource error. Maps to HTTP `400 Bad Request`
    /// (returned as a FHIR `OperationOutcome` by the FHIR layer).
    #[error("FHIR error: {0}")]
    Fhir(String),

    /// An unexpected internal error with no more specific variant. Maps to
    /// HTTP `500 Internal Server Error`.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Builds a [`Database`](Error::Database) error from a custom message,
    /// wrapping it in a `DbErr::Custom` so callers need not import SeaORM.
    pub fn database(msg: impl Into<String>) -> Self {
        Error::Database(sea_orm::DbErr::Custom(msg.into()))
    }

    /// Builds a [`Validation`](Error::Validation) error from a message.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::Error;
    ///
    /// let e = Error::validation("family name is required");
    /// assert_eq!(e.to_string(), "Validation error: family name is required");
    /// ```
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Builds an [`Internal`](Error::Internal) error from a message.
    pub fn internal(msg: impl Into<String>) -> Self {
        Error::Internal(msg.into())
    }
}
