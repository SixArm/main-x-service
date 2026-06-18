//! URL similarity for [`Thing`](crate::models::thing::Thing) matching.
//!
//! Many schema.org/Thing properties are URLs (`url`, `main_entity_of_page`,
//! `same_as`, `subject_of`). This module compares them after light
//! normalization — lowercasing, dropping the scheme, and trimming a trailing
//! slash — so that `http://Example.com/` and `https://example.com` are
//! treated as the same resource. Scoring is coarse and deliberate:
//!
//! - **1.0** — identical after normalization (or same host + same path).
//! - **0.75** — same host, different path (likely the same site/entity).
//! - **0.0** — different host (or either side empty).
//!
//! [`url_list_similarity`](crate::matching::url::url_list_similarity) lifts the
//! pairwise score to the best match across
//! two lists, which is how `same_as` cross-references are scored.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::url::url_similarity;
//!
//! // Scheme and trailing slash are ignored.
//! assert_eq!(url_similarity("http://example.com/", "https://example.com"), 1.0);
//! // Same host, different path.
//! assert_eq!(url_similarity("https://example.com/a", "https://example.com/b"), 0.75);
//! // Different host.
//! assert_eq!(url_similarity("https://example.com", "https://other.com"), 0.0);
//! ```

/// Compute similarity between two URLs.
///
/// Returns 1.0 for identical (normalized) URLs, 0.75 for same host but
/// different path, 0.0 otherwise. Used for matching `url`,
/// `main_entity_of_page`, `same_as`, and similar URL-valued properties.
///
/// # Examples
///
/// ```
/// use thing_service::matching::url::url_similarity;
///
/// assert_eq!(url_similarity("https://EXAMPLE.com", "https://example.com"), 1.0);
/// assert_eq!(url_similarity("", "https://example.com"), 0.0);
/// ```
#[must_use]
pub fn url_similarity(a: &str, b: &str) -> f64 {
    let a_n = normalize(a);
    let b_n = normalize(b);
    // Either side empty (e.g. blank input) is treated as no evidence.
    if a_n.is_empty() || b_n.is_empty() {
        return 0.0;
    }
    // Fast path: byte-for-byte equal after normalization.
    if a_n == b_n {
        return 1.0;
    }
    // Otherwise fall back to host-vs-path comparison: matching hosts are
    // strong evidence even when the paths differ.
    let (host_a, path_a) = split_host_path(&a_n);
    let (host_b, path_b) = split_host_path(&b_n);
    if host_a == host_b {
        if path_a == path_b { 1.0 } else { 0.75 }
    } else {
        0.0
    }
}

/// Best-pair URL similarity across two URL lists.
///
/// Returns the maximum [`url_similarity`] over the Cartesian product of the
/// two lists, i.e. the score of the most similar pair. Used to compare
/// `same_as` cross-reference lists, where a single shared authoritative URL
/// is strong evidence. Returns 0.0 if either list is empty.
///
/// # Examples
///
/// ```
/// use thing_service::matching::url::url_list_similarity;
///
/// let a = vec!["https://en.wikipedia.org/wiki/Linux".to_string()];
/// let b = vec![
///     "https://www.wikidata.org/wiki/Q388".to_string(),
///     "https://en.wikipedia.org/wiki/Linux".to_string(),
/// ];
/// assert_eq!(url_list_similarity(&a, &b), 1.0);
/// ```
#[must_use]
pub fn url_list_similarity(a: &[String], b: &[String]) -> f64 {
    let mut best = 0.0_f64;
    // O(n*m) scan; the lists are small (a handful of authoritative URLs).
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

/// Normalize a URL for comparison: trim, lowercase, drop the `http(s)://`
/// scheme, and strip a trailing slash. The result is a `host[/path]` string
/// with no scheme, suitable for equality and host/path splitting.
fn normalize(s: &str) -> String {
    let lower = s.trim().to_lowercase();
    let stripped = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    stripped.trim_end_matches('/').to_string()
}

/// Split a scheme-less URL into `(host, path)` at the first `/`. When there
/// is no path, the path slice is empty. Operates on the already-[`normalize`]d
/// form.
fn split_host_path(s: &str) -> (&str, &str) {
    match s.find('/') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-identical URLs score 1.0.
    #[test]
    fn test_identical() {
        let s = url_similarity("https://example.com", "https://example.com");
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    /// `http` vs `https` normalizes to the same URL.
    #[test]
    fn test_scheme_insensitive() {
        let s = url_similarity("http://example.com", "https://example.com");
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    /// A trailing slash is stripped during normalization.
    #[test]
    fn test_trailing_slash_normalized() {
        let s = url_similarity("https://example.com/", "https://example.com");
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    /// Host case is folded away.
    #[test]
    fn test_case_insensitive() {
        let s = url_similarity("https://EXAMPLE.com", "https://example.com");
        assert!((s - 1.0).abs() < f64::EPSILON);
    }

    /// Same host with different paths scores 0.75.
    #[test]
    fn test_same_host_different_path() {
        let s = url_similarity("https://example.com/a", "https://example.com/b");
        assert!((s - 0.75).abs() < f64::EPSILON);
    }

    /// Different hosts score 0.0.
    #[test]
    fn test_different_host() {
        let s = url_similarity("https://example.com", "https://other.com");
        assert!(s.abs() < f64::EPSILON);
    }

    /// An empty URL on either side scores 0.0.
    #[test]
    fn test_empty() {
        assert!(url_similarity("", "https://example.com").abs() < f64::EPSILON);
        assert!(url_similarity("", "").abs() < f64::EPSILON);
    }

    /// List similarity returns the best-matching pair's score.
    #[test]
    fn test_list_best_match() {
        let a = vec!["https://en.wikipedia.org/wiki/Pride_and_Prejudice".to_string()];
        let b = vec![
            "https://www.wikidata.org/wiki/Q170583".to_string(),
            "https://en.wikipedia.org/wiki/Pride_and_Prejudice".to_string(),
        ];
        assert!((url_list_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    /// An empty list yields 0.0.
    #[test]
    fn test_list_empty() {
        let a: Vec<String> = vec![];
        let b = vec!["https://example.com".to_string()];
        assert!(url_list_similarity(&a, &b).abs() < f64::EPSILON);
    }
}
