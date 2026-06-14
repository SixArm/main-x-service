//! Identifier matching for [`Thing`](crate::models::thing::Thing) records.
//!
//! Identifiers are the second-highest-weighted component (default 0.30) and
//! the source of the matcher's most powerful rule: a shared *deterministic*
//! identifier short-circuits the entire score to 1.0. This module provides
//! both the graded component score
//! ([`identifier_similarity`](crate::matching::identifier::identifier_similarity))
//! and the short-circuit predicate
//! ([`has_deterministic_match`](crate::matching::identifier::has_deterministic_match)).
//!
//! Matching is exact on the `(property_id, value)` pair — there is no fuzzy
//! comparison, because an identifier is either the same or it is not.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::identifier::{has_deterministic_match, identifier_similarity};
//! use thing_service::models::identifier::ThingIdentifier;
//!
//! let a = vec![ThingIdentifier::isbn("9780141439518")];
//! let b = vec![ThingIdentifier::isbn("9780141439518")];
//! assert_eq!(identifier_similarity(&a, &b), 1.0);
//! assert!(has_deterministic_match(&a, &b)); // ISBN is globally unique
//! ```

use crate::models::identifier::ThingIdentifier;

/// Best-pair identifier similarity.
///
/// Returns 1.0 if any (`property_id`, `value`) pair matches across the
/// two identifier lists; 0.0 otherwise. Empty input on either side
/// returns 0.0. Unlike [`has_deterministic_match`], this considers *all*
/// identifier schemes, including non-unique ones (SKU, URI, Custom).
///
/// # Examples
///
/// ```
/// use thing_service::matching::identifier::identifier_similarity;
/// use thing_service::models::identifier::ThingIdentifier;
///
/// let a = vec![ThingIdentifier::sku("WIDGET-42")];
/// let b = vec![ThingIdentifier::sku("WIDGET-42")];
/// assert_eq!(identifier_similarity(&a, &b), 1.0); // exact pair match
/// ```
#[must_use]
pub fn identifier_similarity(a: &[ThingIdentifier], b: &[ThingIdentifier]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    // Any exact (scheme, value) pair is conclusive evidence of the same item.
    for id_a in a {
        for id_b in b {
            if id_a.property_id == id_b.property_id && id_a.value == id_b.value {
                return 1.0;
            }
        }
    }
    0.0
}

/// True if the two identifier sets share at least one *deterministic*
/// identifier (DOI, ISBN, ISSN, GTIN, MPN, serial number, or UUID).
///
/// Deterministic identifiers are globally unique by construction, so a
/// match short-circuits scoring to 1.0. Non-unique schemes (SKU, URI,
/// Custom) are filtered out before comparison and never trigger the
/// short-circuit.
///
/// # Examples
///
/// ```
/// use thing_service::matching::identifier::has_deterministic_match;
/// use thing_service::models::identifier::ThingIdentifier;
///
/// // Shared SKU is NOT deterministic (vendor-scoped).
/// let a = vec![ThingIdentifier::sku("WIDGET-42")];
/// let b = vec![ThingIdentifier::sku("WIDGET-42")];
/// assert!(!has_deterministic_match(&a, &b));
/// ```
#[must_use]
pub fn has_deterministic_match(a: &[ThingIdentifier], b: &[ThingIdentifier]) -> bool {
    // Consider only globally-unique schemes on both sides; a SKU collision,
    // for example, must not pin two unrelated products to a perfect score.
    for id_a in a.iter().filter(|i| i.is_deterministic()) {
        for id_b in b.iter().filter(|i| i.is_deterministic()) {
            if id_a.property_id == id_b.property_id && id_a.value == id_b.value {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::{IdentifierType, ThingIdentifier};

    /// A shared ISBN scores 1.0.
    #[test]
    fn test_matching_isbn() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780141439518")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    /// Different ISBNs score 0.0.
    #[test]
    fn test_different_isbn() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780199536566")];
        assert!(identifier_similarity(&a, &b).abs() < f64::EPSILON);
    }

    /// An empty identifier list scores 0.0.
    #[test]
    fn test_empty_identifiers() {
        let a: Vec<ThingIdentifier> = vec![];
        let b = vec![ThingIdentifier::isbn("9780141439518")];
        assert!(identifier_similarity(&a, &b).abs() < f64::EPSILON);
    }

    /// A match on any one shared pair (here a Custom id) scores 1.0.
    #[test]
    fn test_mixed_identifiers() {
        let a = vec![
            ThingIdentifier::isbn("9780141439518"),
            ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
        ];
        let b = vec![ThingIdentifier::new(
            IdentifierType::Custom("OpenLibrary".into()),
            "OL1394865W",
        )];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    /// A shared ISBN triggers the deterministic short-circuit.
    #[test]
    fn test_has_deterministic_match_isbn() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780141439518")];
        assert!(has_deterministic_match(&a, &b));
    }

    /// A shared DOI triggers the deterministic short-circuit.
    #[test]
    fn test_has_deterministic_match_doi() {
        let a = vec![ThingIdentifier::doi("10.1000/xyz123")];
        let b = vec![ThingIdentifier::doi("10.1000/xyz123")];
        assert!(has_deterministic_match(&a, &b));
    }

    /// A shared SKU does NOT short-circuit (not globally unique).
    #[test]
    fn test_has_deterministic_match_sku_excluded() {
        // SKU is not globally unique, so it must not short-circuit.
        let a = vec![ThingIdentifier::sku("WIDGET-42")];
        let b = vec![ThingIdentifier::sku("WIDGET-42")];
        assert!(!has_deterministic_match(&a, &b));
    }

    /// A shared Custom identifier does NOT short-circuit.
    #[test]
    fn test_has_deterministic_match_custom_excluded() {
        let a = vec![ThingIdentifier::new(
            IdentifierType::Custom("Internal".into()),
            "X1",
        )];
        let b = vec![ThingIdentifier::new(
            IdentifierType::Custom("Internal".into()),
            "X1",
        )];
        assert!(!has_deterministic_match(&a, &b));
    }

    /// Deterministic identifiers with different values do not short-circuit.
    #[test]
    fn test_no_match_different_values() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780199536566")];
        assert!(!has_deterministic_match(&a, &b));
    }
}
