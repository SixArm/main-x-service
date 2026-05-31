use strsim::jaro_winkler;

/// Compare two names, returning a similarity score 0.0–1.0.
///
/// Case-insensitive Jaro-Winkler with the standard prefix bonus.
/// Empty + empty returns 1.0; one empty side returns 0.0.
pub fn name_similarity(a: &str, b: &str) -> f64 {
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

    #[test]
    fn test_exact_name_match() {
        let score = name_similarity("Pride and Prejudice", "Pride and Prejudice");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_case_insensitive_match() {
        let score = name_similarity("pride and prejudice", "PRIDE AND PREJUDICE");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similar_names() {
        let score = name_similarity("Pride and Prejudice", "Prde and Prejudice");
        assert!(score > 0.8, "Score: {score}");
    }

    #[test]
    fn test_different_names() {
        let score = name_similarity("Pride and Prejudice", "Rust Programming");
        assert!(score < 0.55, "Score: {score}");
    }

    #[test]
    fn test_empty_name() {
        let score = name_similarity("", "Pride and Prejudice");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_both_empty() {
        let score = name_similarity("", "");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_substring_match() {
        let score = name_similarity("Prejudice", "Pride and Prejudice");
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

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
