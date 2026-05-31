use crate::models::identifier::ThingIdentifier;

/// Best-pair identifier similarity.
///
/// Returns 1.0 if any (`property_id`, `value`) pair matches across the
/// two identifier lists; 0.0 otherwise. Empty input on either side
/// returns 0.0.
pub fn identifier_similarity(a: &[ThingIdentifier], b: &[ThingIdentifier]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
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
/// match short-circuits scoring to 1.0.
pub fn has_deterministic_match(a: &[ThingIdentifier], b: &[ThingIdentifier]) -> bool {
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

    #[test]
    fn test_matching_isbn() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780141439518")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_different_isbn() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780199536566")];
        assert_eq!(identifier_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_empty_identifiers() {
        let a: Vec<ThingIdentifier> = vec![];
        let b = vec![ThingIdentifier::isbn("9780141439518")];
        assert_eq!(identifier_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_mixed_identifiers() {
        let a = vec![
            ThingIdentifier::isbn("9780141439518"),
            ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
        ];
        let b = vec![ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_has_deterministic_match_isbn() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780141439518")];
        assert!(has_deterministic_match(&a, &b));
    }

    #[test]
    fn test_has_deterministic_match_doi() {
        let a = vec![ThingIdentifier::doi("10.1000/xyz123")];
        let b = vec![ThingIdentifier::doi("10.1000/xyz123")];
        assert!(has_deterministic_match(&a, &b));
    }

    #[test]
    fn test_has_deterministic_match_sku_excluded() {
        // SKU is not globally unique, so it must not short-circuit.
        let a = vec![ThingIdentifier::sku("WIDGET-42")];
        let b = vec![ThingIdentifier::sku("WIDGET-42")];
        assert!(!has_deterministic_match(&a, &b));
    }

    #[test]
    fn test_has_deterministic_match_custom_excluded() {
        let a = vec![ThingIdentifier::new(IdentifierType::Custom("Internal".into()), "X1")];
        let b = vec![ThingIdentifier::new(IdentifierType::Custom("Internal".into()), "X1")];
        assert!(!has_deterministic_match(&a, &b));
    }

    #[test]
    fn test_no_match_different_values() {
        let a = vec![ThingIdentifier::isbn("9780141439518")];
        let b = vec![ThingIdentifier::isbn("9780199536566")];
        assert!(!has_deterministic_match(&a, &b));
    }
}
