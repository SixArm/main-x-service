//! Identifier matching: exact type+value comparison across identifier lists.
//!
//! Identifiers are the highest-trust matching signal because they reference
//! authoritative external registries. This module provides a binary
//! similarity ([`identifier_similarity`]) and a GLN-specific predicate
//! ([`has_gln_match`]) that the scorer uses to short-circuit to a certain
//! match.
//!
//! # Examples
//!
//! ```
//! use place_service::models::identifier::PlaceIdentifier;
//! use place_service::matching::identifier::{identifier_similarity, has_gln_match};
//!
//! let a = vec![PlaceIdentifier::gln("1234567890123")];
//! let b = vec![PlaceIdentifier::gln("1234567890123")];
//! assert_eq!(identifier_similarity(&a, &b), 1.0);
//! assert!(has_gln_match(&a, &b));
//! ```

use crate::models::identifier::{IdentifierType, PlaceIdentifier};

/// Returns 1.0 if the two lists share any identifier (same type **and**
/// value), else 0.0.
///
/// This is an all-or-nothing signal: a single shared identifier of any
/// scheme is treated as a full match on this axis. Empty input on either
/// side yields 0.0.
///
/// # Examples
///
/// ```
/// use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
/// use place_service::matching::identifier::identifier_similarity;
///
/// let a = vec![
///     PlaceIdentifier::gln("1234567890123"),
///     PlaceIdentifier::new(IdentifierType::Fips, "36061"),
/// ];
/// let b = vec![PlaceIdentifier::new(IdentifierType::Fips, "36061")];
/// // The shared FIPS code is enough to score 1.0.
/// assert_eq!(identifier_similarity(&a, &b), 1.0);
/// ```
#[must_use]
pub fn identifier_similarity(a: &[PlaceIdentifier], b: &[PlaceIdentifier]) -> f64 {
    // No identifiers to compare on one side ⇒ no signal.
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    // O(n*m) cross-scan; identifier lists are tiny in practice, so the
    // simple double loop is faster than building a hash set.
    for id_a in a {
        for id_b in b {
            if id_a.identifier_type == id_b.identifier_type && id_a.value == id_b.value {
                return 1.0;
            }
        }
    }

    0.0
}

/// Returns whether the two lists share a Global Location Number.
///
/// A GLN match is treated as *deterministic* by the scorer: it pins the
/// overall match score to 1.0. This predicate is stricter than
/// [`identifier_similarity`] in that it only considers
/// [`GlobalLocationNumber`](IdentifierType::GlobalLocationNumber)-typed
/// entries, so a value collision under a different scheme does not count.
///
/// # Examples
///
/// ```
/// use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
/// use place_service::matching::identifier::has_gln_match;
///
/// // Same value, but typed as FIPS on one side: not a GLN match.
/// let a = vec![PlaceIdentifier::new(IdentifierType::Fips, "1234567890123")];
/// let b = vec![PlaceIdentifier::gln("1234567890123")];
/// assert!(!has_gln_match(&a, &b));
/// ```
#[must_use]
pub fn has_gln_match(a: &[PlaceIdentifier], b: &[PlaceIdentifier]) -> bool {
    // Collect the GLN values on side `a` once, then scan side `b` against
    // them. Only GLN-typed entries qualify on either side.
    let a_glns: Vec<&str> = a
        .iter()
        .filter(|id| id.identifier_type == IdentifierType::GlobalLocationNumber)
        .map(|id| id.value.as_str())
        .collect();

    b.iter().any(|id| {
        id.identifier_type == IdentifierType::GlobalLocationNumber
            && a_glns.contains(&id.value.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two identical GLNs score 1.0.
    #[test]
    fn test_matching_gln() {
        let a = vec![PlaceIdentifier::gln("1234567890123")];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    /// Different GLN values score 0.0.
    #[test]
    fn test_different_gln() {
        let a = vec![PlaceIdentifier::gln("1234567890123")];
        let b = vec![PlaceIdentifier::gln("9876543210987")];
        assert!((identifier_similarity(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    /// An empty list on one side scores 0.0.
    #[test]
    fn test_empty_identifiers() {
        let a: Vec<PlaceIdentifier> = vec![];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!((identifier_similarity(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    /// A single shared non-GLN identifier is enough to score 1.0.
    #[test]
    fn test_mixed_identifiers() {
        let a = vec![
            PlaceIdentifier::gln("1234567890123"),
            PlaceIdentifier::new(IdentifierType::Fips, "36061"),
        ];
        let b = vec![PlaceIdentifier::new(IdentifierType::Fips, "36061")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    /// A shared GLN is detected.
    #[test]
    fn test_has_gln_match_true() {
        let a = vec![PlaceIdentifier::gln("1234567890123")];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!(has_gln_match(&a, &b));
    }

    /// Same value but non-GLN type is not a GLN match.
    #[test]
    fn test_has_gln_match_false_different_type() {
        let a = vec![PlaceIdentifier::new(IdentifierType::Fips, "1234567890123")];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!(!has_gln_match(&a, &b));
    }

    /// Different GLN values are not a GLN match.
    #[test]
    fn test_no_gln_match() {
        let a = vec![PlaceIdentifier::gln("1111111111111")];
        let b = vec![PlaceIdentifier::gln("2222222222222")];
        assert!(!has_gln_match(&a, &b));
    }
}
