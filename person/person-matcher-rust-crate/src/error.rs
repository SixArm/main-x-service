//! Error types for person-matcher operations.
//!
//! The crate uses a single sum-type, [`MatchingError`], for every fallible
//! operation, and a [`Result`] alias to keep call-sites concise.
//!
//! The matching engine itself is **infallible**: scoring two persons always
//! produces a [`crate::MatchResult`]. Errors arise from explicit validation
//! steps such as [`crate::Person::validate`].
//!
//! ## Example
//!
//! ```
//! use person_matcher::{MatchingError, Person};
//!
//! let empty = Person::builder().build();
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
/// use person_matcher::Result;
/// fn doubled(x: i32) -> Result<i32> { Ok(x * 2) }
/// assert_eq!(doubled(3).unwrap(), 6);
/// ```
pub type Result<T> = std::result::Result<T, MatchingError>;

/// Errors that may be returned by person-matcher operations.
///
/// The matching engine itself is infallible — scoring two persons always
/// produces a [`crate::MatchResult`]. The only fallible operation in the
/// public surface today is [`crate::Person::validate`], which returns
/// [`MatchingError::MissingField`] when neither a name nor an identifier
/// is populated. Identifier parsers in [`crate::identifiers`] return
/// `Option<String>` (the parser is the source of truth on validity), so
/// they never surface as errors. Configuration builders
/// ([`crate::MatchConfig::default`], `strict`, `lenient`) are infallible.
///
/// The enum is `#[non_exhaustive]` so future fallible code paths can add
/// variants without breaking `SemVer` for downstream pattern-matches.
///
/// ```
/// use person_matcher::MatchingError;
///
/// let e = MatchingError::MissingField("united_kingdom_national_health_service_number".into());
/// // `Display` is provided by `thiserror`.
/// assert!(e.to_string().contains("Missing required field"));
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MatchingError {
    /// A required field was absent. Returned by [`crate::Person::validate`].
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unnecessary_wraps)] // demonstrates the `Result` alias
    use super::*;

    /// Pins the `Display` text of `MissingField` — the `thiserror` format
    /// string `"Missing required field: {0}"` is a stable, user-visible
    /// contract, so a wording change must be a conscious edit.
    #[test]
    fn missing_field_display() {
        let e = MatchingError::MissingField("united_kingdom_national_health_service_number".into());
        assert_eq!(
            e.to_string(),
            "Missing required field: united_kingdom_national_health_service_number"
        );
    }

    /// Confirms the `Result<T>` alias resolves and is usable as a return
    /// type — a compile-plus-runtime smoke test of the crate's public alias.
    #[test]
    fn result_alias_resolves() {
        fn make() -> Result<i32> {
            Ok(42)
        }
        assert_eq!(make().unwrap(), 42);
    }

    /// Pins that `MatchingError` is `Send + Sync` so it can cross threads
    /// and be stored in `anyhow`/`Box<dyn Error + Send + Sync>` by callers.
    #[test]
    fn errors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MatchingError>();
    }
}
