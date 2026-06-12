//! Description similarity for [`Thing`](crate::models::thing::Thing) matching.
//!
//! A low-weight component (default 0.10) that compares free-text
//! `description` (or `disambiguating_description`) fields with the same
//! case-insensitive [Jaro-Winkler](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance)
//! similarity used for names. Description is a supporting signal: it nudges
//! the score but rarely decides a match on its own.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::description::description_similarity;
//!
//! let a = "A novel of manners by Jane Austen";
//! let b = "A novel of manners by Jne Austen"; // typo
//! assert!(description_similarity(a, b) > 0.9);
//! ```

use strsim::jaro_winkler;

/// Compare two descriptions, returning a similarity score 0.0–1.0.
///
/// Case-insensitive Jaro-Winkler. Used for matching `description` and
/// `disambiguating_description`. Both empty returns 1.0; one empty side
/// returns 0.0.
///
/// # Examples
///
/// ```
/// use thing_service::matching::description::description_similarity;
///
/// assert_eq!(description_similarity("", ""), 1.0);
/// assert_eq!(description_similarity("", "non-empty"), 0.0);
/// ```
pub fn description_similarity(a: &str, b: &str) -> f64 {
    // Same empty-string conventions as name matching: both empty → perfect,
    // one empty → mismatch.
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    jaro_winkler(&a.to_lowercase(), &b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identical descriptions score 1.0.
    #[test]
    fn test_exact() {
        assert!(
            (description_similarity("A novel by Jane Austen", "A novel by Jane Austen") - 1.0)
                .abs()
                < f64::EPSILON
        );
    }

    /// Case is folded away before comparison.
    #[test]
    fn test_case_insensitive() {
        let s = description_similarity("A NOVEL by Jane Austen", "a novel by jane austen");
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    /// A near-identical description scores above 0.9.
    #[test]
    fn test_similar() {
        let s = description_similarity("A novel by Jane Austen", "A novel by Jne Austen");
        assert!(s > 0.9, "{s}");
    }

    /// Unrelated descriptions score low.
    #[test]
    fn test_different() {
        let s = description_similarity(
            "A novel by Jane Austen",
            "Standard library for asynchronous Rust",
        );
        assert!(s < 0.6, "{s}");
    }

    /// Two empty descriptions score 1.0.
    #[test]
    fn test_both_empty() {
        assert_eq!(description_similarity("", ""), 1.0);
    }

    /// One empty description scores 0.0.
    #[test]
    fn test_one_empty() {
        assert_eq!(description_similarity("", "non-empty"), 0.0);
    }
}
