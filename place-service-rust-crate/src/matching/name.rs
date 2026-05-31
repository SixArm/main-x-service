use strsim::jaro_winkler;

/// Compare two place names, returning a similarity score 0.0 to 1.0.
pub fn name_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    jaro_winkler(&a_lower, &b_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_name_match() {
        let score = name_similarity("Central Park", "Central Park");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_case_insensitive_match() {
        let score = name_similarity("central park", "CENTRAL PARK");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similar_names() {
        let score = name_similarity("Central Park", "Centrl Park");
        assert!(score > 0.8, "Score: {score}");
    }

    #[test]
    fn test_different_names() {
        let score = name_similarity("Central Park", "Golden Gate Bridge");
        assert!(score < 0.6, "Score: {score}");
    }

    #[test]
    fn test_empty_name() {
        let score = name_similarity("", "Central Park");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_both_empty() {
        let score = name_similarity("", "");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_substring_match() {
        let score = name_similarity("Park", "Central Park");
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    #[test]
    fn test_jaro_winkler_prefix_bonus() {
        let score_prefix = name_similarity("Central Park", "Central Gardens");
        let score_no_prefix = name_similarity("Park Central", "Gardens Central");
        assert!(score_prefix > score_no_prefix,
            "prefix: {score_prefix}, no_prefix: {score_no_prefix}");
    }
}
