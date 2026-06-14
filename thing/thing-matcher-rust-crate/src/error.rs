//! Error types for thing-matcher operations.
//!
//! The crate uses a single sum-type, [`MatchingError`], for every fallible
//! operation, and a [`Result`] alias to keep call-sites concise.
//!
//! The matching engine itself is **infallible**: scoring two things always
//! produces a [`crate::MatchResult`]. Errors arise from explicit validation
//! steps such as [`crate::Thing::validate`].
//!
//! ## Example
//!
//! ```
//! use thing_matcher::{MatchingError, Thing};
//!
//! let empty = Thing::builder().build();
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
/// use thing_matcher::Result;
/// fn doubled(x: i32) -> Result<i32> { Ok(x * 2) }
/// assert_eq!(doubled(3).unwrap(), 6);
/// ```
pub type Result<T> = std::result::Result<T, MatchingError>;

/// Errors that may be returned by thing-matcher operations.
///
/// The matching engine itself is infallible — scoring two things always
/// produces a [`crate::MatchResult`]. The only fallible operation in the
/// public surface today is [`crate::Thing::validate`], which returns
/// [`MatchingError::MissingField`] when the primary `name` is absent.
/// Configuration builders ([`crate::MatchConfig::default`], `strict`,
/// `lenient`) are infallible.
///
/// The enum is `#[non_exhaustive]` so future fallible code paths can add
/// variants without breaking `SemVer` for downstream pattern-matches.
///
/// ```
/// use thing_matcher::MatchingError;
///
/// let e = MatchingError::MissingField("name".into());
/// // `Display` is provided by `thiserror`.
/// assert!(e.to_string().contains("Missing required field"));
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MatchingError {
    /// A required field was absent. Returned by [`crate::Thing::validate`].
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the `thiserror`-generated `Display` text for `MissingField`,
    /// including the interpolated field name.
    #[test]
    fn missing_field_display() {
        let e = MatchingError::MissingField("name".into());
        assert_eq!(e.to_string(), "Missing required field: name");
    }

    /// Pins that the [`Result`] type alias resolves and can carry an `Ok`
    /// value — a compile-plus-runtime check on the alias itself.
    #[test]
    fn result_alias_resolves() {
        // Deliberately wraps an infallible value to exercise the alias.
        #[allow(clippy::unnecessary_wraps)]
        fn make() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(make().unwrap(), 42);
    }

    /// Pins the `Send + Sync` bound on `MatchingError` via a compile-time
    /// trait assertion, so the error can cross thread boundaries (e.g. be
    /// returned from a worker thread). Regressions here would surface as a
    /// build failure rather than a runtime one.
    #[test]
    fn errors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MatchingError>();
    }
}
