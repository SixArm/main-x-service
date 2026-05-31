//! Error types for place-matcher operations.
//!
//! The crate uses a single sum-type, [`MatchingError`], for every fallible
//! operation, and a [`Result`] alias to keep call-sites concise.
//!
//! The matching engine itself is **infallible**: scoring two places always
//! produces a [`crate::MatchResult`]. Errors arise from explicit validation
//! steps such as [`crate::Place::validate`].
//!
//! ## Example
//!
//! ```
//! use place_matcher::{MatchingError, Place};
//!
//! let empty = Place::builder().build();
//! match empty.validate() {
//!     Err(MatchingError::MissingField(msg)) => {
//!         assert!(msg.contains("required"));
//!     }
//!     other => panic!("unexpected: {other:?}"),
//! }
//! ```

use thiserror::Error;

/// Result alias used throughout the crate.
///
/// Equivalent to `std::result::Result<T, MatchingError>`.
///
/// ```
/// use place_matcher::Result;
/// fn doubled(x: i32) -> Result<i32> { Ok(x * 2) }
/// assert_eq!(doubled(3).unwrap(), 6);
/// ```
pub type Result<T> = std::result::Result<T, MatchingError>;

/// Errors that may be returned by place-matcher operations.
///
/// The matching engine itself is infallible — scoring two places always
/// produces a [`crate::MatchResult`]. The only fallible operation in the
/// public surface today is [`crate::Place::validate`], which returns
/// [`MatchingError::MissingField`] when the primary `name` is absent.
/// Configuration builders ([`crate::MatchConfig::default`], `strict`,
/// `lenient`) are infallible.
///
/// The enum is `#[non_exhaustive]` so future fallible code paths can add
/// variants without breaking SemVer for downstream pattern-matches.
///
/// ```
/// use place_matcher::MatchingError;
///
/// let e = MatchingError::MissingField("name".into());
/// // `Display` is provided by `thiserror`.
/// assert!(e.to_string().contains("Missing required field"));
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MatchingError {
    /// A required field was absent. Returned by [`crate::Place::validate`].
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_field_display() {
        let e = MatchingError::MissingField("name".into());
        assert_eq!(e.to_string(), "Missing required field: name");
    }

    #[test]
    fn result_alias_resolves() {
        fn make() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(make().unwrap(), 42);
    }

    #[test]
    fn errors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MatchingError>();
    }
}
