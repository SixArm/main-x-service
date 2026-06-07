//! Place-name similarity via [Jaro-Winkler](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance).
//!
//! Name similarity is the heaviest-weighted component of the place match
//! score (default 0.35). The comparison is case-insensitive and benefits
//! from Jaro-Winkler's prefix bonus, so two names that share a leading run
//! (`"Central Park"` vs `"Central Gardens"`) score higher than ones that
//! only share a trailing run.
//!
//! # Examples
//!
//! ```
//! use place_service::matching::name::name_similarity;
//!
//! assert!((name_similarity("Central Park", "central park") - 1.0).abs() < 1e-9);
//! assert!(name_similarity("Central Park", "Centrl Park") > 0.8);
//! ```

use strsim::jaro_winkler;

/// Compare two place names, returning a similarity score `0.0..=1.0`.
///
/// The comparison lowercases both inputs first, so it is case-insensitive.
/// Two empty strings count as a perfect match (1.0); exactly one empty
/// string counts as a complete mismatch (0.0).
///
/// # Examples
///
/// ```
/// use place_service::matching::name::name_similarity;
///
/// assert_eq!(name_similarity("", ""), 1.0);
/// assert_eq!(name_similarity("", "Park"), 0.0);
/// assert!(name_similarity("Park", "Central Park") < 1.0);
/// ```
pub fn name_similarity(a: &str, b: &str) -> f64 {
    // Two empty names are vacuously identical.
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    // One empty, one not: no shared content.
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    // Case-fold so "PARK" and "park" compare equal.
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    jaro_winkler(&a_lower, &b_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identical names score a perfect 1.0.
    #[test]
    fn test_exact_name_match() {
        let score = name_similarity("Central Park", "Central Park");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// Comparison ignores case.
    #[test]
    fn test_case_insensitive_match() {
        let score = name_similarity("central park", "CENTRAL PARK");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// A single-character typo still scores highly.
    #[test]
    fn test_similar_names() {
        let score = name_similarity("Central Park", "Centrl Park");
        assert!(score > 0.8, "Score: {score}");
    }

    /// Unrelated names score low.
    #[test]
    fn test_different_names() {
        let score = name_similarity("Central Park", "Golden Gate Bridge");
        assert!(score < 0.6, "Score: {score}");
    }

    /// One empty name against a non-empty one scores 0.0.
    #[test]
    fn test_empty_name() {
        let score = name_similarity("", "Central Park");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    /// Two empty names score 1.0 (vacuously equal).
    #[test]
    fn test_both_empty() {
        let score = name_similarity("", "");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// A substring scores partial: above 0.0 but below 1.0.
    #[test]
    fn test_substring_match() {
        let score = name_similarity("Park", "Central Park");
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    /// Shared prefixes score higher than shared suffixes (Winkler bonus).
    #[test]
    fn test_jaro_winkler_prefix_bonus() {
        let score_prefix = name_similarity("Central Park", "Central Gardens");
        let score_no_prefix = name_similarity("Park Central", "Gardens Central");
        assert!(score_prefix > score_no_prefix,
            "prefix: {score_prefix}, no_prefix: {score_no_prefix}");
    }
}
