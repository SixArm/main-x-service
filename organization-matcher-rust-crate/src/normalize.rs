//! Normalisation helpers — case-fold + trim + Unicode-NFKC, plus
//! organization-name legal-suffix stripping and URL→domain extraction.
//! Pure library code.

use unicode_normalization::UnicodeNormalization;

/// Common legal-form suffixes/markers stripped when comparing
/// organization names. Lower-cased, punctuation already removed by the
/// caller. Order does not matter; all are removed as whole tokens.
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
    let cleaned: String = folded
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let kept: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|tok| !LEGAL_SUFFIXES.contains(tok))
        .collect();
    if kept.is_empty() {
        // All tokens were legal suffixes (e.g. "The Co"); fall back to
        // the cleaned form so we never return an empty key.
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
    fn legal_name_strips_suffixes_and_punctuation() {
        assert_eq!(legal_name("Acme, Inc."), "acme");
        assert_eq!(legal_name("ACME"), "acme");
        assert_eq!(legal_name("Acme Corporation"), "acme");
        assert_eq!(legal_name("Müller GmbH"), "müller");
        assert_eq!(legal_name("The Boring Company"), "boring");
    }

    #[test]
    fn legal_name_never_empty() {
        // All tokens are suffixes — fall back to the cleaned form.
        assert_eq!(legal_name("The Co"), "the co");
    }

    #[test]
    fn domain_extracts_registered_host() {
        assert_eq!(domain("https://www.Acme.com/about"), "acme.com");
        assert_eq!(domain("http://acme.com"), "acme.com");
        assert_eq!(domain("acme.com"), "acme.com");
        assert_eq!(domain("https://user@acme.com:8080/x?y=1"), "acme.com");
        assert_eq!(domain("https://sub.acme.co.uk/"), "sub.acme.co.uk");
    }

    #[test]
    fn fold_set_dedupes_and_sorts() {
        let v = vec!["B".into(), "a".into(), " A ".into(), String::new()];
        assert_eq!(fold_set(&v), vec!["a".to_string(), "b".to_string()]);
    }
}
