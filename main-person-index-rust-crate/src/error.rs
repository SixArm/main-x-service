//! Error types for the MPI system

use thiserror::Error;

/// Result type alias for MPI operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the Main Person Index system
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Connection pool error: {0}")]
    Pool(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Person not found: {0}")]
    PersonNotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Matching error: {0}")]
    Matching(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Streaming error: {0}")]
    Streaming(String),

    #[error("FHIR error: {0}")]
    Fhir(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a new database error
    pub fn database(msg: impl Into<String>) -> Self {
        Error::Database(sea_orm::DbErr::Custom(msg.into()))
    }

    /// Create a new validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a new internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Error::Internal(msg.into())
    }
}
