//! Normalisation helpers — case-fold + trim + Unicode-NFKC, plus the
//! owner-scoped code form, URL form, set folding, and ISO-date parsing
//! used by the timeframe component. Pure library code.

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
    s.trim().nfkc().collect::<String>().to_lowercase()
}

/// Normalise an owner-scoped `code`: keep only alphanumerics, uppercased.
/// Drops whitespace, hyphens, and other punctuation so `"PROJ-2026"` and
/// `"proj 2026"` compare equal.
///
/// Codes are formatted inconsistently across tools (spaces vs hyphens vs
/// none), so a stricter, punctuation-free canonical form is used for the
/// `R-1` short-circuit and the code component — more aggressive than
/// [`fold`], which keeps separators.
///
/// `s` is the raw code. Returns the uppercased alphanumeric-only form
/// (empty when `s` has no alphanumeric characters).
#[must_use]
pub fn code(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_uppercase()
}

/// Normalise a `same_as` URL for the deterministic overlap rule: trim +
/// NFKC + lowercase, then drop a trailing slash so trivial formatting
/// differences do not defeat the comparison.
///
/// `s` is the raw URL string. Returns the folded URL with at most one
/// trailing slash removed. A lone `"/"` is left intact, and the
/// comparison stays purely textual — no scheme/host parsing.
#[must_use]
pub fn url(s: &str) -> String {
    let folded = fold(s);
    match folded.strip_suffix('/') {
        Some(trimmed) if !trimmed.is_empty() => trimmed.to_string(),
        _ => folded,
    }
}

/// Lower-case + trim + dedupe a `Vec<String>`, dropping empty entries.
///
/// Turns a raw list (`keywords` / `tags` / folded goal titles) into the
/// canonical set the Jaccard components compare: folding normalises each
/// element, empties are discarded, and sort-then-dedupe yields a stable,
/// duplicate-free set so ordering and repeats do not affect the
/// similarity.
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
    out.sort();
    out.dedup();
    out
}

/// Parse an ISO-8601 calendar date (`YYYY`, `YYYY-MM`, or `YYYY-MM-DD`)
/// to a signed day count relative to the Unix epoch (1970-01-01 = 0).
///
/// The timeframe component compares `start_date` / `target_date` by day
/// gap, so each date is reduced to a single ordinal. A bare `YYYY`
/// defaults to January 1st and `YYYY-MM` to the 1st of the month. Parsing
/// is strict: non-numeric parts, out-of-range months/days, and trailing
/// junk all yield `None` (the component then treats the date as absent).
/// Pure arithmetic — no `chrono`/`time` dependency.
///
/// `s` is the raw date string. Returns `Some(days_since_epoch)` for a
/// well-formed date, else `None`.
#[must_use]
pub fn iso_date_to_days(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let mut parts = t.split('-');
    // Bound the year to the ISO-8601 4-digit range. An unbounded `i64` year
    // would overflow the `era * 146_097` term in `days_from_civil` (panic in
    // debug, wrap in release) on a crafted value like `"99999999999999-01-01"`
    // (SEC-M4). Project timeframes never need years outside `0..=9999`.
    let year: i64 = parts
        .next()?
        .parse()
        .ok()
        .filter(|v| (0..=9999).contains(v))?;
    // Month / day default to 1 when omitted; reject out-of-range values.
    let month: i64 = match parts.next() {
        Some(m) => m.parse().ok().filter(|v| (1..=12).contains(v))?,
        None => 1,
    };
    let day: i64 = match parts.next() {
        Some(d) => d.parse().ok().filter(|v| (1..=31).contains(v))?,
        None => 1,
    };
    // Any extra `-`-separated part is malformed input.
    if parts.next().is_some() {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

/// Days from the Unix epoch for a proleptic-Gregorian `(y, m, d)`, via
/// Howard Hinnant's `days_from_civil` algorithm (exact integer math,
/// valid for the full `i64` year range). `m` is `1..=12`, `d` is
/// `1..=31`; callers validate ranges. Returns the signed day count
/// (1970-01-01 → 0).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    // Shift so the year starts in March: leap day lands at the year's end.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // Mar=0 … Feb=11
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_lowercases_and_trims() {
        assert_eq!(fold("  Hello WORLD  "), "hello world");
    }

    #[test]
    fn code_strips_whitespace_and_uppercases() {
        assert_eq!(code(" proj 26 "), "PROJ26");
    }

    #[test]
    fn code_drops_hyphens() {
        assert_eq!(code("PROJ-2026-001"), "PROJ2026001");
        assert_eq!(code("proj 2026 001"), "PROJ2026001");
    }

    #[test]
    fn url_trims_and_drops_trailing_slash() {
        assert_eq!(
            url("  https://pm.example.com/p/1/  "),
            "https://pm.example.com/p/1"
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
    fn fold_set_empty_input_is_empty() {
        assert!(fold_set(&[]).is_empty());
        let blanks = vec![String::new(), "   ".to_string()];
        assert!(fold_set(&blanks).is_empty());
    }

    // The epoch itself is day 0; a known later date matches the civil
    // formula; ordering is preserved.
    #[test]
    fn iso_date_epoch_and_known_values() {
        assert_eq!(iso_date_to_days("1970-01-01"), Some(0));
        assert_eq!(iso_date_to_days("1970-01-02"), Some(1));
        assert_eq!(iso_date_to_days("1969-12-31"), Some(-1));
        // 2000-01-01 is 10957 days after the epoch.
        assert_eq!(iso_date_to_days("2000-01-01"), Some(10_957));
    }

    // A bare year and a year-month default to the 1st.
    #[test]
    fn iso_date_partial_precision_defaults_to_first() {
        assert_eq!(iso_date_to_days("2000"), iso_date_to_days("2000-01-01"));
        assert_eq!(iso_date_to_days("2000-03"), iso_date_to_days("2000-03-01"));
    }

    // The day gap between two dates is exact (leap year included).
    #[test]
    fn iso_date_gap_is_exact() {
        let a = iso_date_to_days("2024-01-01").unwrap();
        let b = iso_date_to_days("2024-03-01").unwrap();
        // Jan (31) + Feb (29, leap) = 60 days.
        assert_eq!(b - a, 60);
    }

    // Malformed input yields None.
    #[test]
    fn iso_date_rejects_malformed() {
        assert_eq!(iso_date_to_days(""), None);
        assert_eq!(iso_date_to_days("not-a-date"), None);
        assert_eq!(iso_date_to_days("2024-13-01"), None);
        assert_eq!(iso_date_to_days("2024-00-01"), None);
        assert_eq!(iso_date_to_days("2024-01-32"), None);
        assert_eq!(iso_date_to_days("2024-01-01-01"), None);
    }

    // SEC-M4: an out-of-range year must be rejected, not overflow the
    // `era * 146_097` term in `days_from_civil` (panic in debug / wrap in
    // release). The 4-digit ISO boundary is honoured; anything past it is
    // `None`, and a pathological value never panics.
    #[test]
    fn iso_date_year_is_bounded_and_never_overflows() {
        assert!(iso_date_to_days("9999-12-31").is_some()); // upper ISO bound ok
        assert_eq!(iso_date_to_days("10000-01-01"), None); // just past the bound
        assert_eq!(iso_date_to_days("99999999999999-01-01"), None); // would overflow
        assert_eq!(iso_date_to_days("9223372036854775807-01-01"), None); // i64::MAX
    }
}
