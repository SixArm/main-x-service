use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Matching error: {0}")]
    Matching(String),
    #[error("Normalization error: {0}")]
    Normalization(String),
}

pub type Result<T> = std::result::Result<T, Error>;
