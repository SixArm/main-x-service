//! Normalisation helpers — case-fold + trim + Unicode-NFKC, plus
//! organization-name legal-suffix stripping and URL→domain extraction.
//! Pure library code.

use unicode_normalization::UnicodeNormalization;

/// Common legal-form suffixes/markers stripped when comparing
/// organization names. Lower-cased, punctuation already removed by the
/// caller. Order does not matter; all are removed as whole tokens.
///
/// Spans multiple jurisdictions (US `inc`/`llc`, UK `ltd`/`plc`,
/// German `gmbh`/`ag`, French `sa`/`sarl`, Italian `srl`/`spa`, …) plus
/// the noise words `the`/`and`/`&`, so that legal-form decoration does
/// not dominate the name similarity. Matched as whole whitespace tokens
/// only, so a legitimate name fragment that merely *contains* one of
/// these strings is never clipped.
const LEGAL_SUFFIXES: &[&str] = &[
    "inc",
    "incorporated",
    "corp",
    "corporation",
    "co",
    "company",
    "ltd",
    "limited",
    "llc",
    "llp",
    "lp",
    "plc",
    "gmbh",
    "ag",
    "sa",
    "sas",
    "sasu",
    "srl",
    "spa",
    "bv",
    "nv",
    "oy",
    "ab",
    "as",
    "pty",
    "pte",
    "kk",
    "kg",
    "ohg",
    "ug",
    "sl",
    "sarl",
    "eurl",
    "the",
    "and",
    "&",
];

/// Lowercase + trim + NFKC-normalise a string. Empty input returns an
/// empty string (never `None`).
#[must_use]
pub fn fold(s: &str) -> String {
    // NFKC before lower-casing so compatibility variants (e.g. full-width
    // forms, ligatures) collapse to a canonical form. Diacritics are
    // deliberately preserved — `Müller` must not fold to `Muller`.
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise an organization name for comparison: fold, strip
/// punctuation to spaces, drop common legal-form suffix tokens
/// (`Inc`, `Ltd`, `GmbH`, …) and the noise words `the`/`and`, then
/// collapse whitespace. `"Acme, Inc."` and `"ACME"` both fold to
/// `"acme"`.
#[must_use]
pub fn legal_name(s: &str) -> String {
    let folded = fold(s);
    // Replace every non-alphanumeric char with a space so punctuation
    // (commas, periods, hyphens) becomes a token boundary rather than
    // sticking to an adjacent word.
    let cleaned: String = folded
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    // Keep only tokens that are not legal-form suffixes / noise words.
    let kept: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|tok| !LEGAL_SUFFIXES.contains(tok))
        .collect();
    if kept.is_empty() {
        // All tokens were legal suffixes (e.g. "The Co"); fall back to
        // the cleaned form so we never return an empty key (an empty
        // key would spuriously match other empties at Jaro-Winkler 1.0).
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        kept.join(" ")
    }
}

/// Extract a comparable registered domain from a URL or bare host:
/// lower-cased host with any scheme, `www.` prefix, path, port, and
/// trailing dot removed. `"https://www.Acme.com/about"` → `"acme.com"`.
#[must_use]
pub fn domain(s: &str) -> String {
    let t = s.trim().to_lowercase();
    // Strip scheme.
    let no_scheme = t.split_once("://").map_or(t.as_str(), |(_, rest)| rest);
    // Host is everything up to the first '/', '?', or '#'.
    let host = no_scheme.split(['/', '?', '#']).next().unwrap_or(no_scheme);
    // Drop userinfo and port.
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    let host = host.trim_end_matches('.');
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

/// Fold (lower-case + trim + NFKC) every entry, drop blanks, then sort
/// and de-duplicate — producing a canonical set for Jaccard comparison.
/// Sorting before `dedup` is required because `Vec::dedup` only removes
/// *consecutive* duplicates.
#[must_use]
pub fn fold_set(items: &[String]) -> Vec<String> {
    let mut out: Vec<String> = items
        .iter()
        .map(|s| fold(s))
        .filter(|s| !s.is_empty())
        .collect();
    // Sort so that equal values become adjacent, then collapse them.
    out.sort();
    out.dedup();
    out
}

/// Unit tests for the normalisation helpers.
#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the basic fold: surrounding whitespace trimmed, case lowered.
    #[test]
    fn fold_lowercases_and_trims() {
        assert_eq!(fold("  Hello WORLD  "), "hello world");
    }

    /// Pins suffix/punctuation stripping AND diacritic preservation
    /// (`Müller` stays `müller`, not `muller`) and noise-word removal.
    #[test]
    fn legal_name_strips_suffixes_and_punctuation() {
        assert_eq!(legal_name("Acme, Inc."), "acme");
        assert_eq!(legal_name("ACME"), "acme");
        assert_eq!(legal_name("Acme Corporation"), "acme");
        assert_eq!(legal_name("Müller GmbH"), "müller");
        assert_eq!(legal_name("The Boring Company"), "boring");
    }

    /// Pins the empty-key guard: a name made entirely of suffix tokens
    /// falls back to the cleaned form instead of yielding "".
    #[test]
    fn legal_name_never_empty() {
        // All tokens are suffixes — fall back to the cleaned form.
        assert_eq!(legal_name("The Co"), "the co");
    }

    /// Pins domain extraction across scheme, `www.`, path, userinfo,
    /// port, and a multi-label host (`sub.acme.co.uk` is kept intact).
    #[test]
    fn domain_extracts_registered_host() {
        assert_eq!(domain("https://www.Acme.com/about"), "acme.com");
        assert_eq!(domain("http://acme.com"), "acme.com");
        assert_eq!(domain("acme.com"), "acme.com");
        assert_eq!(domain("https://user@acme.com:8080/x?y=1"), "acme.com");
        assert_eq!(domain("https://sub.acme.co.uk/"), "sub.acme.co.uk");
    }

    /// Pins that `fold_set` folds, drops the blank, sorts, and dedupes
    /// (`"a"` and `" A "` collapse to a single `"a"`).
    #[test]
    fn fold_set_dedupes_and_sorts() {
        let v = vec!["B".into(), "a".into(), " A ".into(), String::new()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }
}
