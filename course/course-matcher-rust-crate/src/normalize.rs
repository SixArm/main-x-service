//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code, no allocations on the happy path beyond what
//! `String::from` requires.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string.
///
/// This is the canonical comparison form for free-text fields (names,
/// keywords, `same_as` URLs, identifier values). Order of operations
/// matters: trim removes surrounding whitespace, NFKC folds Unicode
/// compatibility forms (ligatures, full-width variants) so visually
/// equal strings compare equal, and lowercasing makes the comparison
/// case-blind. Diacritics are deliberately *preserved* — `Café` and
/// `Cafe` are different course identities.
///
/// # Returns
///
/// The folded string. Empty or whitespace-only input returns an empty
/// `String` (never `None`); callers test `.is_empty()` to skip blanks.
///
/// # Examples
///
/// ```
/// use course_matcher::normalize::fold;
///
/// assert_eq!(fold("  Hello WORLD  "), "hello world");
/// ```
#[must_use]
pub fn fold(s: &str) -> String {
    // NFKC before lower-casing so compatibility decompositions (e.g.
    // the "ﬁ" ligature → "fi") happen first; trim strips edge space.
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a course-code: uppercase + strip ALL whitespace.
///
/// Course codes are written inconsistently (`CS101`, `CS 101`,
/// `cs101`), so the comparison form removes every interior and edge
/// space and upper-cases the rest. This lets `cs 101` and `CS101`
/// compare equal in the R-1 deterministic rule and the course-code
/// component.
///
/// # Examples
///
/// ```
/// use course_matcher::normalize::course_code;
///
/// assert_eq!(course_code(" cs 101 "), "CS101");
/// ```
#[must_use]
pub fn course_code(s: &str) -> String {
    s.chars()
        // Drop every whitespace char, not just the edges — `CS 101`
        // and `CS101` must collapse to the same token.
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

/// Fold, dedupe, and sort a slice of strings into a canonical set.
///
/// Used by the Jaccard components (keywords, teaches): a set has no
/// duplicates and no order, so this folds each entry, drops blanks,
/// sorts, and dedupes. Sorting before `dedup` is required because
/// `Vec::dedup` only removes *consecutive* duplicates.
///
/// # Examples
///
/// ```
/// use course_matcher::normalize::fold_set;
///
/// let v = vec!["B".to_string(), "a".to_string(), " A ".to_string()];
/// assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
/// ```
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

/// Unit tests for the normalisation helpers.
#[cfg(test)]
mod tests {
    use super::*;

    // Pins the core fold behaviour: lower-case + edge-trim.
    #[test]
    fn fold_lowercases_and_trims() {
        assert_eq!(fold("  Hello WORLD  "), "hello world");
    }

    // Pins course-code canonicalisation: interior + edge space removed,
    // result upper-cased.
    #[test]
    fn course_code_strips_whitespace_and_uppercases() {
        assert_eq!(course_code(" cs 101 "), "CS101");
    }

    // Pins that fold_set produces a sorted, deduped, blank-free set.
    #[test]
    fn fold_set_dedupes_and_sorts() {
        let v = vec!["B".into(), "a".into(), " A ".into(), String::new()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }

    // Pins that blank / whitespace-only input folds to "" so callers
    // can cheaply skip absent fields.
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
