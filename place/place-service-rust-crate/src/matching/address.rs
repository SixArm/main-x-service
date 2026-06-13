//! Postal-address similarity via weighted, field-by-field Jaro-Winkler.
//!
//! Addresses are compared component-wise and combined into a weighted
//! average. Crucially the average is *adaptive*: only fields present in
//! **both** addresses contribute to the score and to the weight
//! denominator, so a sparse-but-agreeing address (just locality + region)
//! is not penalized for the missing street/postal fields. If no field is
//! shared, the score is 0.0.
//!
//! Default per-field weights: postal code 0.30, locality 0.25, street 0.25,
//! region 0.10, country 0.10.
//!
//! # Examples
//!
//! ```
//! use place_service::models::address::PostalAddress;
//! use place_service::matching::address::address_similarity;
//!
//! let a = PostalAddress {
//!     street_address: Some("14 E 60th St".into()),
//!     address_locality: Some("New York".into()),
//!     address_region: Some("NY".into()),
//!     address_country: Some("US".into()),
//!     postal_code: Some("10022".into()),
//! };
//! assert!((address_similarity(&a, &a) - 1.0).abs() < 1e-9);
//! ```

use crate::models::address::PostalAddress;
use strsim::jaro_winkler;

/// Compare two addresses, returning a similarity score `0.0..=1.0`.
///
/// Only fields populated in both addresses contribute. The per-field
/// weights are postal code 0.30, locality 0.25, street 0.25, region 0.10,
/// country 0.10; the raw weighted sum is divided by the sum of the weights
/// of the fields that actually participated. Returns 0.0 when the addresses
/// share no populated field.
///
/// # Examples
///
/// ```
/// use place_service::models::address::PostalAddress;
/// use place_service::matching::address::address_similarity;
///
/// let mut a = PostalAddress::new();
/// a.address_locality = Some("New York".into());
/// let mut b = PostalAddress::new();
/// b.address_country = Some("US".into());
/// // No field is populated in both, so there is nothing to compare.
/// assert_eq!(address_similarity(&a, &b), 0.0);
/// ```
pub fn address_similarity(a: &PostalAddress, b: &PostalAddress) -> f64 {
    // Accumulate weight*similarity for shared fields, plus the weight that
    // actually participated, so the final divide normalizes correctly.
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

    // Normalize by the participating weight, not the full 1.0, so sparse
    // addresses are not unfairly penalized. No shared field ⇒ 0.0.
    if weight_sum > 0.0 {
        score / weight_sum
    } else {
        0.0
    }
}

/// Case-insensitive similarity of one address field.
///
/// Returns 1.0 for a case-insensitive exact match (a fast path that also
/// avoids any Jaro-Winkler rounding), otherwise the Jaro-Winkler score of
/// the lowercased strings.
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

    /// A fully-populated reference address used across the tests.
    fn full_address() -> PostalAddress {
        PostalAddress {
            street_address: Some("14 E 60th St".into()),
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: Some("10022".into()),
        }
    }

    /// Identical addresses score a perfect 1.0.
    #[test]
    fn test_identical_addresses() {
        let a = full_address();
        let score = address_similarity(&a, &a);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// Two unrelated full addresses score low.
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

    /// A sparse address that agrees on its shared fields scores high
    /// (missing fields are not penalized).
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

    /// Addresses with no field populated in both score 0.0.
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

    /// Field comparison ignores case, so a case-only diff scores 1.0.
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
