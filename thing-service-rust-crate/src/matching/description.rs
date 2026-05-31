use strsim::jaro_winkler;

/// Compare two descriptions, returning a similarity score 0.0–1.0.
///
/// Case-insensitive Jaro-Winkler. Used for matching `description` and
/// `disambiguating_description`.
pub fn description_similarity(a: &str, b: &str) -> f64 {
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
    fn test_exact() {
        assert!((description_similarity("A novel by Jane Austen", "A novel by Jane Austen") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_case_insensitive() {
        let s = description_similarity("A NOVEL by Jane Austen", "a novel by jane austen");
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similar() {
        let s = description_similarity("A novel by Jane Austen", "A novel by Jne Austen");
        assert!(s > 0.9, "{s}");
    }

    #[test]
    fn test_different() {
        let s = description_similarity("A novel by Jane Austen", "Standard library for asynchronous Rust");
        assert!(s < 0.6, "{s}");
    }

    #[test]
    fn test_both_empty() {
        assert_eq!(description_similarity("", ""), 1.0);
    }

    #[test]
    fn test_one_empty() {
        assert_eq!(description_similarity("", "non-empty"), 0.0);
    }
}
