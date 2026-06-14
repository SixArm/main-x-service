//! Normalisation helpers — case-fold + trim + Unicode-NFKC. Pure
//! library code.

use unicode_normalization::UnicodeNormalization;

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
#[must_use]
pub fn fold(s: &str) -> String {
    // Order matters: trim surrounding whitespace, apply NFKC to collapse
    // compatibility forms (e.g. the "ﬁ" ligature → "fi"), then lowercase.
    // NFKC before lowercasing keeps casing decisions consistent across
    // composed/decomposed inputs. Diacritics are PRESERVED (no stripping).
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise a pathway-code: keep only alphanumerics, uppercased. Drops
/// whitespace, hyphens, and other punctuation so `"STROKE-01"` and
/// `"stroke 01"` compare equal.
#[must_use]
pub fn pathway_code(s: &str) -> String {
    // Codes are typically formatted inconsistently across systems
    // ("STROKE-01" vs "stroke 01"), so strip everything that is not an
    // ASCII alphanumeric and uppercase the rest to get a canonical key.
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
        .map(|s| fold(s)) // canonicalise each entry
        .filter(|s| !s.is_empty()) // drop blanks so they can't pad the set
        .collect();
    // Sort then `dedup` gives a deduplicated set representation. `dedup`
    // only removes *adjacent* duplicates, so the prior `sort` is required
    // for it to remove all of them. The result feeds Jaccard set maths.
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Core `fold` behaviour: outer whitespace trimmed, characters lowercased.
    #[test]
    fn fold_lowercases_and_trims() {
        assert_eq!(fold("  Hello WORLD  "), "hello world");
    }

    // Pathway-code canonicalisation: spaces dropped, result uppercased.
    #[test]
    fn pathway_code_strips_whitespace_and_uppercases() {
        assert_eq!(pathway_code(" cp 12 "), "CP12");
    }

    // `fold_set` folds, drops the blank, dedupes "a"/" A " to one, sorts.
    #[test]
    fn fold_set_dedupes_and_sorts() {
        let v = vec!["B".into(), "a".into(), " A ".into(), String::new()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }

    // Empty and whitespace-only inputs fold to the empty string (not panic).
    #[test]
    fn fold_empty_and_whitespace_only_is_empty() {
        assert_eq!(fold(""), "");
        assert_eq!(fold("   "), "");
    }

    // Diacritics survive folding — folding lowercases but never strips
    // accents, so "É"/"Ü" stay as accented lowercase letters.
    #[test]
    fn fold_preserves_diacritics() {
        assert_eq!(fold("  CAFÉ Über  "), "café über");
    }

    // NFKC collapses the "ﬁ" ligature compatibility form to plain "fi".
    #[test]
    fn fold_nfkc_normalises_compatibility_forms() {
        assert_eq!(fold("ﬁle"), "file");
    }

    // Pathway-code folding removes interior spaces and tabs/newlines too,
    // since the filter keeps only ASCII alphanumerics.
    #[test]
    fn pathway_code_handles_internal_whitespace() {
        assert_eq!(pathway_code("stroke 01"), "STROKE01");
        assert_eq!(pathway_code("\tcp\n12 "), "CP12");
    }

    // Empty / all-blank input to `fold_set` yields an empty set.
    #[test]
    fn fold_set_empty_input_is_empty() {
        assert!(fold_set(&[]).is_empty());
        let blanks = vec![String::new(), "   ".to_string()];
        assert!(fold_set(&blanks).is_empty());
    }
}
