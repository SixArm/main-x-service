use crate::models::identifier::{IdentifierType, PlaceIdentifier};

/// Check if two places share a matching identifier.
pub fn identifier_similarity(a: &[PlaceIdentifier], b: &[PlaceIdentifier]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    for id_a in a {
        for id_b in b {
            if id_a.identifier_type == id_b.identifier_type && id_a.value == id_b.value {
                return 1.0;
            }
        }
    }

    0.0
}

/// Check specifically for GLN match (deterministic).
pub fn has_gln_match(a: &[PlaceIdentifier], b: &[PlaceIdentifier]) -> bool {
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

    #[test]
    fn test_matching_gln() {
        let a = vec![PlaceIdentifier::gln("1234567890123")];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_different_gln() {
        let a = vec![PlaceIdentifier::gln("1234567890123")];
        let b = vec![PlaceIdentifier::gln("9876543210987")];
        assert!((identifier_similarity(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_identifiers() {
        let a: Vec<PlaceIdentifier> = vec![];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!((identifier_similarity(&a, &b) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mixed_identifiers() {
        let a = vec![
            PlaceIdentifier::gln("1234567890123"),
            PlaceIdentifier::new(IdentifierType::Fips, "36061"),
        ];
        let b = vec![PlaceIdentifier::new(IdentifierType::Fips, "36061")];
        assert!((identifier_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_has_gln_match_true() {
        let a = vec![PlaceIdentifier::gln("1234567890123")];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!(has_gln_match(&a, &b));
    }

    #[test]
    fn test_has_gln_match_false_different_type() {
        let a = vec![PlaceIdentifier::new(IdentifierType::Fips, "1234567890123")];
        let b = vec![PlaceIdentifier::gln("1234567890123")];
        assert!(!has_gln_match(&a, &b));
    }

    #[test]
    fn test_no_gln_match() {
        let a = vec![PlaceIdentifier::gln("1111111111111")];
        let b = vec![PlaceIdentifier::gln("2222222222222")];
        assert!(!has_gln_match(&a, &b));
    }
}
