//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
#[must_use]
pub fn fold(s: &str) -> String {
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a case-number: keep only alphanumerics, uppercased. Drops
/// whitespace, hyphens, and other punctuation so `"CV-2024-001234"` and
/// `"cv 2024 001234"` compare equal.
#[must_use]
pub fn case_number(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_uppercase()
}

/// Normalise a `same_as` URL for the deterministic overlap rule: trim +
/// NFKC + lowercase, then drop a trailing slash so trivial formatting
/// differences do not defeat the comparison.
#[must_use]
pub fn url(s: &str) -> String {
    let folded = fold(s);
    match folded.strip_suffix('/') {
        Some(trimmed) if !trimmed.is_empty() => trimmed.to_string(),
        _ => folded,
    }
}

/// Lower-case + trim + dedupe a `Vec<String>`, dropping empty entries.
#[must_use]
pub fn fold_set(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = items
        .iter()
        .map(|s| fold(s))
        .filter(|s| !s.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_lowercases_and_trims() {
        assert_eq!(fold("  Hello WORLD  "), "hello world");
    }

    #[test]
    fn case_number_strips_whitespace_and_uppercases() {
        assert_eq!(case_number(" cv 24 "), "CV24");
    }

    #[test]
    fn case_number_drops_hyphens() {
        assert_eq!(case_number("CV-2024-001234"), "CV2024001234");
        assert_eq!(case_number("cv 2024 001234"), "CV2024001234");
    }

    #[test]
    fn url_trims_and_drops_trailing_slash() {
        assert_eq!(
            url("  https://courts.example.gov/case/1/  "),
            "https://courts.example.gov/case/1"
        );
        assert_eq!(url("/"), "/");
    }

    #[test]
    fn fold_set_dedupes_and_sorts() {
        let v = vec!["B".into(), "a".into(), " A ".into(), String::new()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn fold_empty_and_whitespace_only_is_empty() {
        assert_eq!(fold(""), "");
        assert_eq!(fold("   "), "");
    }

    #[test]
    fn fold_preserves_diacritics() {
        assert_eq!(fold("  CAFÉ Über  "), "café über");
    }

    #[test]
    fn fold_nfkc_normalises_compatibility_forms() {
        assert_eq!(fold("ﬁle"), "file");
    }

    #[test]
    fn case_number_handles_internal_whitespace() {
        assert_eq!(case_number("cv 24"), "CV24");
        assert_eq!(case_number("\tcv\n24 "), "CV24");
    }

    #[test]
    fn fold_set_empty_input_is_empty() {
        assert!(fold_set(&[]).is_empty());
        let blanks = vec![String::new(), "   ".to_string()];
        assert!(fold_set(&blanks).is_empty());
    }
}
