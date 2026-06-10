//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code, no allocations on the happy path beyond what
//! `String::from` requires.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
#[must_use]
pub fn fold(s: &str) -> String {
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a course-code: uppercase + strip whitespace.
#[must_use]
pub fn course_code(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
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
    fn course_code_strips_whitespace_and_uppercases() {
        assert_eq!(course_code(" cs 101 "), "CS101");
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
        // Case-folds but does not strip accents — they carry identity.
        assert_eq!(fold("  CAFÉ Über  "), "café über");
    }

    #[test]
    fn fold_nfkc_normalises_compatibility_forms() {
        // Ligature "ﬁ" (U+FB01) decomposes to "fi" under NFKC.
        assert_eq!(fold("ﬁle"), "file");
    }

    #[test]
    fn course_code_handles_internal_and_edge_whitespace() {
        assert_eq!(course_code("math 220"), "MATH220");
        assert_eq!(course_code("\tcs\n101 "), "CS101");
    }

    #[test]
    fn course_code_empty_is_empty() {
        assert_eq!(course_code(""), "");
        assert_eq!(course_code("   "), "");
    }

    #[test]
    fn fold_set_empty_input_is_empty() {
        assert!(fold_set(&[]).is_empty());
        let blanks = vec![String::new(), "   ".to_string()];
        assert!(fold_set(&blanks).is_empty());
    }
}
