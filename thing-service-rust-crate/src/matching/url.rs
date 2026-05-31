/// Compute similarity between two URLs.
///
/// Returns 1.0 for identical (normalized) URLs, 0.75 for same host but
/// different path, 0.0 otherwise. Used for matching `url`,
/// `main_entity_of_page`, `same_as`, and similar URL-valued properties.
pub fn url_similarity(a: &str, b: &str) -> f64 {
    let a_n = normalize(a);
    let b_n = normalize(b);
    if a_n.is_empty() || b_n.is_empty() {
        return 0.0;
    }
    if a_n == b_n {
        return 1.0;
    }
    let (host_a, path_a) = split_host_path(&a_n);
    let (host_b, path_b) = split_host_path(&b_n);
    if host_a == host_b {
        if path_a == path_b { 1.0 } else { 0.75 }
    } else {
        0.0
    }
}

/// Best-pair URL similarity across two URL lists.
pub fn url_list_similarity(a: &[String], b: &[String]) -> f64 {
    let mut best = 0.0_f64;
    for ua in a {
        for ub in b {
            let s = url_similarity(ua, ub);
            if s > best {
                best = s;
            }
        }
    }
    best
}

fn normalize(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    let stripped = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    stripped.trim_end_matches('/').to_string()
}

fn split_host_path(s: &str) -> (&str, &str) {
    match s.find('/') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical() {
        assert_eq!(url_similarity("https://example.com", "https://example.com"), 1.0);
    }

    #[test]
    fn test_scheme_insensitive() {
        assert_eq!(url_similarity("http://example.com", "https://example.com"), 1.0);
    }

    #[test]
    fn test_trailing_slash_normalized() {
        assert_eq!(url_similarity("https://example.com/", "https://example.com"), 1.0);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(url_similarity("https://EXAMPLE.com", "https://example.com"), 1.0);
    }

    #[test]
    fn test_same_host_different_path() {
        let s = url_similarity("https://example.com/a", "https://example.com/b");
        assert!((s - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_different_host() {
        let s = url_similarity("https://example.com", "https://other.com");
        assert_eq!(s, 0.0);
    }

    #[test]
    fn test_empty() {
        assert_eq!(url_similarity("", "https://example.com"), 0.0);
        assert_eq!(url_similarity("", ""), 0.0);
    }

    #[test]
    fn test_list_best_match() {
        let a = vec!["https://en.wikipedia.org/wiki/Pride_and_Prejudice".to_string()];
        let b = vec![
            "https://www.wikidata.org/wiki/Q170583".to_string(),
            "https://en.wikipedia.org/wiki/Pride_and_Prejudice".to_string(),
        ];
        assert_eq!(url_list_similarity(&a, &b), 1.0);
    }

    #[test]
    fn test_list_empty() {
        let a: Vec<String> = vec![];
        let b = vec!["https://example.com".to_string()];
        assert_eq!(url_list_similarity(&a, &b), 0.0);
    }
}
