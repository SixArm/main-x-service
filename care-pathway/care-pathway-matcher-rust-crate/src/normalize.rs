//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
#[must_use]
pub fn fold(s: &str) -> String {
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a pathway-code: keep only alphanumerics, uppercased. Drops
/// whitespace, hyphens, and other punctuation so `"STROKE-01"` and
/// `"stroke 01"` compare equal.
#[must_use]
pub fn pathway_code(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_uppercase()
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
    fn pathway_code_strips_whitespace_and_uppercases() {
        assert_eq!(pathway_code(" cp 12 "), "CP12");
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
    fn pathway_code_handles_internal_whitespace() {
        assert_eq!(pathway_code("stroke 01"), "STROKE01");
        assert_eq!(pathway_code("\tcp\n12 "), "CP12");
    }

    #[test]
    fn fold_set_empty_input_is_empty() {
        assert!(fold_set(&[]).is_empty());
        let blanks = vec![String::new(), "   ".to_string()];
        assert!(fold_set(&blanks).is_empty());
    }
}
