//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code, no allocations on the happy path beyond what
//! `String::from` requires.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
pub fn fold(s: &str) -> String {
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a course-code: uppercase + strip whitespace.
pub fn course_code(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase()
}

/// Lower-case + trim + dedupe a `Vec<String>`, dropping empty entries.
pub fn fold_set(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = items.iter().map(|s| fold(s)).filter(|s| !s.is_empty()).collect();
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
        let v = vec!["B".into(), "a".into(), " A ".into(), "".into()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }
}
