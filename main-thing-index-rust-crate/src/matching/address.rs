use crate::models::address::PostalAddress;
use strsim::jaro_winkler;

/// Compare two addresses, returning similarity score 0.0 to 1.0.
/// Weights: postal_code 30%, locality 25%, street 25%, region 10%, country 10%.
pub fn address_similarity(a: &PostalAddress, b: &PostalAddress) -> f64 {
    let mut score = 0.0;
    let mut weight_sum = 0.0;

    if let (Some(a_pc), Some(b_pc)) = (&a.postal_code, &b.postal_code) {
        score += 0.30 * field_similarity(a_pc, b_pc);
        weight_sum += 0.30;
    }
    if let (Some(a_loc), Some(b_loc)) = (&a.address_locality, &b.address_locality) {
        score += 0.25 * field_similarity(a_loc, b_loc);
        weight_sum += 0.25;
    }
    if let (Some(a_st), Some(b_st)) = (&a.street_address, &b.street_address) {
        score += 0.25 * field_similarity(a_st, b_st);
        weight_sum += 0.25;
    }
    if let (Some(a_reg), Some(b_reg)) = (&a.address_region, &b.address_region) {
        score += 0.10 * field_similarity(a_reg, b_reg);
        weight_sum += 0.10;
    }
    if let (Some(a_co), Some(b_co)) = (&a.address_country, &b.address_country) {
        score += 0.10 * field_similarity(a_co, b_co);
        weight_sum += 0.10;
    }

    if weight_sum > 0.0 {
        score / weight_sum
    } else {
        0.0
    }
}

fn field_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    if a_lower == b_lower {
        1.0
    } else {
        jaro_winkler(&a_lower, &b_lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_address() -> PostalAddress {
        PostalAddress {
            street_address: Some("14 E 60th St".into()),
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: Some("10022".into()),
        }
    }

    #[test]
    fn test_identical_addresses() {
        let a = full_address();
        let score = address_similarity(&a, &a);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_different_addresses() {
        let a = full_address();
        let b = PostalAddress {
            street_address: Some("1600 Pennsylvania Ave".into()),
            address_locality: Some("Washington".into()),
            address_region: Some("DC".into()),
            address_country: Some("US".into()),
            postal_code: Some("20500".into()),
        };
        let score = address_similarity(&a, &b);
        assert!(score < 0.6, "Score: {score}");
    }

    #[test]
    fn test_partial_address_match() {
        let a = full_address();
        let b = PostalAddress {
            street_address: None,
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: None,
        };
        let score = address_similarity(&a, &b);
        assert!(score > 0.9, "Score: {score}");
    }

    #[test]
    fn test_no_overlapping_fields() {
        let a = PostalAddress {
            street_address: Some("123 Main".into()),
            address_locality: None,
            address_region: None,
            address_country: None,
            postal_code: None,
        };
        let b = PostalAddress {
            street_address: None,
            address_locality: Some("Town".into()),
            address_region: None,
            address_country: None,
            postal_code: None,
        };
        let score = address_similarity(&a, &b);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_case_insensitive_address() {
        let a = full_address();
        let b = PostalAddress {
            street_address: Some("14 E 60TH ST".into()),
            address_locality: Some("NEW YORK".into()),
            address_region: Some("ny".into()),
            address_country: Some("us".into()),
            postal_code: Some("10022".into()),
        };
        let score = address_similarity(&a, &b);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }
}
