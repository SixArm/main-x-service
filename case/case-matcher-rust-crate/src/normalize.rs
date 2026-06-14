//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
///
/// The canonical text normaliser used before every fuzzy or exact text
/// comparison in the crate, so that case, leading/trailing whitespace,
/// and Unicode compatibility variants (e.g. the `ﬁ` ligature → `fi`) do
/// not produce spurious mismatches. Diacritics are *preserved* (NFKC,
/// not a stripping normaliser) because they carry meaning in names.
///
/// `s` is the raw input. Returns the normalised owned `String`.
#[must_use]
pub fn fold(s: &str) -> String {
    // Order matters: trim first to drop surrounding space, NFKC to fold
    // compatibility forms, then lowercase for case-insensitive compares.
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a case-number: keep only alphanumerics, uppercased. Drops
/// whitespace, hyphens, and other punctuation so `"CV-2024-001234"` and
/// `"cv 2024 001234"` compare equal.
///
/// Case numbers are formatted inconsistently across systems (spaces vs
/// hyphens vs none), so a stricter, punctuation-free canonical form is
/// used for the `R-1` short-circuit and the case-number component — more
/// aggressive than [`fold`], which keeps separators.
///
/// `s` is the raw case number. Returns the uppercased alphanumeric-only
/// form (empty when `s` has no alphanumeric characters).
#[must_use]
pub fn case_number(s: &str) -> String {
    s.chars()
        // ASCII-only filter: case numbers are administrative codes, not
        // free text, so non-ASCII alphanumerics are not expected.
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_uppercase()
}

/// Normalise a `same_as` URL for the deterministic overlap rule: trim +
/// NFKC + lowercase, then drop a trailing slash so trivial formatting
/// differences do not defeat the comparison.
///
/// `s` is the raw URL string. Returns the folded URL with at most one
/// trailing slash removed. A lone `"/"` is left intact (stripping it
/// would yield an empty string that could falsely overlap), and the
/// comparison stays purely textual — no scheme/host parsing.
#[must_use]
pub fn url(s: &str) -> String {
    let folded = fold(s);
    match folded.strip_suffix('/') {
        // Strip a trailing slash only when something non-empty remains,
        // so `".../case/1/"` and `".../case/1"` compare equal.
        Some(trimmed) if !trimmed.is_empty() => trimmed.to_string(),
        // No trailing slash, or the string was just "/": keep as-is.
        _ => folded,
    }
}

/// Lower-case + trim + dedupe a `Vec<String>`, dropping empty entries.
///
/// Turns a raw list (`subjects` / `keywords`) into the canonical set the
/// Jaccard component compares: folding normalises each element, empties
/// are discarded, and sort-then-dedupe yields a stable, duplicate-free
/// set so ordering and repeats do not affect the similarity.
///
/// `items` is the raw slice. Returns the sorted, deduplicated, folded set
/// (empty when every element folds away).
#[must_use]
pub fn fold_set(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = items
        .iter()
        .map(|s| fold(s))
        .filter(|s| !s.is_empty())
        .collect();
    // Sort before dedup: `Vec::dedup` only removes *consecutive*
    // duplicates, so it must run on a sorted vector to dedupe globally.
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins the basic fold contract: surrounding space trimmed, casing
    // lowered, internal single space preserved.
    #[test]
    fn fold_lowercases_and_trims() {
        assert_eq!(fold("  Hello WORLD  "), "hello world");
    }

    // Pins that case-number normalisation drops spaces and uppercases.
    #[test]
    fn case_number_strips_whitespace_and_uppercases() {
        assert_eq!(case_number(" cv 24 "), "CV24");
    }

    // Pins the key equivalence: hyphenated and space-separated forms of
    // the same case number normalise identically.
    #[test]
    fn case_number_drops_hyphens() {
        assert_eq!(case_number("CV-2024-001234"), "CV2024001234");
        assert_eq!(case_number("cv 2024 001234"), "CV2024001234");
    }

    // Pins URL normalisation: whitespace trimmed and one trailing slash
    // removed, but a lone "/" is preserved (not stripped to empty).
    #[test]
    fn url_trims_and_drops_trailing_slash() {
        assert_eq!(
            url("  https://courts.example.gov/case/1/  "),
            "https://courts.example.gov/case/1"
        );
        assert_eq!(url("/"), "/");
    }

    // Pins set folding: `"a"`, `" A "` collapse to one `"a"`, output is
    // sorted, and blank entries are dropped.
    #[test]
    fn fold_set_dedupes_and_sorts() {
        let v = vec!["B".into(), "a".into(), " A ".into(), String::new()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }

    // Pins that empty / whitespace-only input folds to the empty string
    // (the signal the components use to skip a missing field).
    #[test]
    fn fold_empty_and_whitespace_only_is_empty() {
        assert_eq!(fold(""), "");
        assert_eq!(fold("   "), "");
    }

    // Pins that diacritics survive folding (NFKC, not diacritic-strip).
    #[test]
    fn fold_preserves_diacritics() {
        assert_eq!(fold("  CAFÉ Über  "), "café über");
    }

    // Pins NFKC compatibility folding: the `ﬁ` ligature becomes `fi`.
    #[test]
    fn fold_nfkc_normalises_compatibility_forms() {
        assert_eq!(fold("ﬁle"), "file");
    }

    // Pins that internal whitespace (spaces, tabs, newlines) is stripped
    // from case numbers, not just leading/trailing.
    #[test]
    fn case_number_handles_internal_whitespace() {
        assert_eq!(case_number("cv 24"), "CV24");
        assert_eq!(case_number("\tcv\n24 "), "CV24");
    }

    // Pins that an empty or all-blank input set folds to an empty set.
    #[test]
    fn fold_set_empty_input_is_empty() {
        assert!(fold_set(&[]).is_empty());
        let blanks = vec![String::new(), "   ".to_string()];
        assert!(fold_set(&blanks).is_empty());
    }
}
