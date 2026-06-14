//! Name similarity for [`Thing`](crate::models::thing::Thing) matching.
//!
//! Names are the highest-weighted component of the match score (default
//! weight 0.40). This module wraps [Jaro-Winkler](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance)
//! similarity with case-folding and explicit handling of the empty-string
//! edge cases, so the rest of the matcher does not have to.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::name::name_similarity;
//!
//! // A single-character typo still scores high.
//! assert!(name_similarity("Pride and Prejudice", "Prde and Prejudice") > 0.8);
//! // Unrelated names score low.
//! assert!(name_similarity("Pride and Prejudice", "Rust Programming") < 0.55);
//! ```

use strsim::jaro_winkler;

/// Compare two names, returning a similarity score 0.0–1.0.
///
/// Case-insensitive Jaro-Winkler with the standard prefix bonus.
/// Empty + empty returns 1.0; one empty side returns 0.0.
///
/// # Examples
///
/// ```
/// use thing_service::matching::name::name_similarity;
///
/// assert_eq!(name_similarity("Linux", "LINUX"), 1.0); // case-insensitive
/// assert_eq!(name_similarity("", ""), 1.0);           // both empty
/// assert_eq!(name_similarity("", "Linux"), 0.0);      // one empty
/// ```
#[must_use]
pub fn name_similarity(a: &str, b: &str) -> f64 {
    // Two absent names are treated as a perfect match (nothing to disagree
    // on); one absent name is a definite mismatch.
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    // Lowercase both sides so the comparison is case-insensitive.
    jaro_winkler(&a.to_lowercase(), &b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identical names score exactly 1.0.
    #[test]
    fn test_exact_name_match() {
        let score = name_similarity("Pride and Prejudice", "Pride and Prejudice");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// Case differences are folded away before comparison.
    #[test]
    fn test_case_insensitive_match() {
        let score = name_similarity("pride and prejudice", "PRIDE AND PREJUDICE");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// A single-character typo still scores above 0.8.
    #[test]
    fn test_similar_names() {
        let score = name_similarity("Pride and Prejudice", "Prde and Prejudice");
        assert!(score > 0.8, "Score: {score}");
    }

    /// Unrelated names score well below the match threshold.
    #[test]
    fn test_different_names() {
        let score = name_similarity("Pride and Prejudice", "Rust Programming");
        assert!(score < 0.55, "Score: {score}");
    }

    /// One empty name yields 0.0 (definite mismatch).
    #[test]
    fn test_empty_name() {
        let score = name_similarity("", "Pride and Prejudice");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    /// Two empty names yield 1.0 (nothing to disagree on).
    #[test]
    fn test_both_empty() {
        let score = name_similarity("", "");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    /// A substring scores between 0 and 1 (partial, not exact).
    #[test]
    fn test_substring_match() {
        let score = name_similarity("Prejudice", "Pride and Prejudice");
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    /// Jaro-Winkler rewards a shared leading prefix over a shared suffix.
    #[test]
    fn test_jaro_winkler_prefix_bonus() {
        // Same prefix should score higher than same suffix.
        let score_prefix = name_similarity("Pride and Prejudice", "Pride and Persuasion");
        let score_no_prefix = name_similarity("Prejudice Pride", "Persuasion Pride");
        assert!(
            score_prefix > score_no_prefix,
            "prefix: {score_prefix}, no_prefix: {score_no_prefix}"
        );
    }
}
