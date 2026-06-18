//! Crate-level error type.
//!
//! [`enum@Error`] is the single error enum threaded through every fallible
//! path in the crate, paired with the [`Result`] alias. The REST layer
//! maps each variant to an HTTP status code in
//! `crate::api::rest::handlers::error_response` (`NotFound` → 404,
//! `Validation` → 422, `Conflict` → 409, everything else → 500), so the
//! variant a layer returns directly determines the response a client
//! sees.

use thiserror::Error;

/// All errors the Course Service can raise.
///
/// Each variant carries a human-readable detail string (except
/// [`Error::NotFound`], which is parameterless). Variants are produced
/// at the layer named: `Config`/`Pool`/`Database` from `config` + `db`,
/// `Search` from `search`, `Matching` from `matching`, `Validation`
/// from `validation`, `Streaming` from `streaming`, and
/// `Api`/`Conflict`/`NotFound` from the REST layer.
#[derive(Debug, Error)]
pub enum Error {
    /// Configuration could not be loaded or a value failed to parse
    /// (e.g. a non-numeric `SERVER_PORT`). Raised by `Config::from_env`.
    #[error("Configuration error: {0}")]
    Config(String),

    /// A database query, transaction, or (de)serialization of a JSONB
    /// column failed. Maps to HTTP 500.
    #[error("Database error: {0}")]
    Database(String),

    /// The connection pool could not be opened. Distinct from
    /// [`Error::Database`] so boot-time failures are easy to spot.
    #[error("Connection pool error: {0}")]
    Pool(String),

    /// A Tantivy index operation (open, write, query, reload) failed.
    #[error("Search error: {0}")]
    Search(String),

    /// The matching layer failed. Reserved; the current matcher facade
    /// is infallible, but the variant keeps the surface stable.
    #[error("Matching error: {0}")]
    Matching(String),

    /// A record failed data-quality validation. Maps to HTTP 422.
    #[error("Validation error: {0}")]
    Validation(String),

    /// The requested record does not exist (or is soft-deleted). Maps
    /// to HTTP 404.
    #[error("Not found")]
    NotFound,

    /// The request conflicts with existing state (e.g. a probable
    /// duplicate). Maps to HTTP 409.
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
///
/// Use this everywhere a function can fail with the crate's [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;
