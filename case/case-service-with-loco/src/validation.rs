//! Field-level validation for incoming `Case` payloads.
//!
//! The service stores the matcher's `Case` verbatim, so payload
//! validation is the *service's* responsibility — the matcher is a pure
//! scoring library and deliberately performs no validation. These checks
//! return human-readable problem strings that the controller surfaces as
//! a single `422 Unprocessable Entity`.
//!
//! ## Rules
//!
//! - **`title`** — required; must not be blank.
//! - **`opened_date`** — when present, must be ISO-8601 `YYYY` or
//!   `YYYY-MM-DD` (a real calendar date; e.g. `2024`, `2024-01-31`).
//! - **`identifiers[i].value`** — must not be blank.
//! - **`subjects[i]` / `keywords[i]`** — each entry must not be blank.
//! - **Input-size caps (SEC-M1)** — every scalar text field, array
//!   cardinality, and per-entry string length is bounded, so a single
//!   huge string or huge array cannot be used as a CPU/memory `DoS`
//!   against the matcher's O(n·m) Jaro-Winkler / Levenshtein / Jaccard
//!   scoring (amplified across the `check-duplicates` scan). Oversized
//!   input is rejected with a `422` *before* the record is stored or
//!   matched.

use case_matcher::Case;

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single scalar text field (`title`, `case_number`, `agency_id`,
/// `agency_name`). Bounds the per-field cost of the matcher's
/// character-level string comparisons.
const MAX_TEXT_LEN: usize = 1024;

/// Maximum number of entries in any array field (`alternate_titles`,
/// `subjects`, `keywords`, `identifiers`, `same_as`, `in_language`).
/// Bounds the O(n·m) Jaccard / overlap work the matcher does over arrays.
const MAX_ARRAY_LEN: usize = 256;

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single string entry inside an array field.
const MAX_ITEM_LEN: usize = 512;

/// Collect every validation problem for `case`. An empty vector means
/// the payload is valid.
///
/// The controller joins these into one `422` response, so the operator
/// sees all problems at once rather than fixing them one round-trip at a
/// time.
#[must_use]
pub fn problems(case: &Case) -> Vec<String> {
    let mut out = Vec::new();
    if case.title.trim().is_empty() {
        out.push("title is required".to_string());
    }
    if let Some(date) = &case.opened_date
        && !is_valid_iso_date(date.trim())
    {
        out.push(format!(
            "opened_date: {date:?} is not a valid ISO-8601 date"
        ));
    }

    // Input-size caps (SEC-M1): scalar text fields.
    check_text(&mut out, "title", &case.title);
    if let Some(v) = &case.case_number {
        check_text(&mut out, "case_number", v);
    }
    if let Some(v) = &case.agency_id {
        check_text(&mut out, "agency_id", v);
    }
    if let Some(v) = &case.agency_name {
        check_text(&mut out, "agency_name", v);
    }

    // Input-size caps (SEC-M1): array cardinality.
    check_array(&mut out, "alternate_titles", case.alternate_titles.len());
    check_array(&mut out, "subjects", case.subjects.len());
    check_array(&mut out, "keywords", case.keywords.len());
    check_array(&mut out, "identifiers", case.identifiers.len());
    check_array(&mut out, "same_as", case.same_as.len());
    check_array(&mut out, "in_language", case.in_language.len());

    // Per-entry blank checks (existing) + size caps (SEC-M1). Every one
    // walks `inspected`, not the whole array — see its docs (SEC-M8).
    for (i, ident) in inspected(&case.identifiers) {
        if ident.value.trim().is_empty() {
            out.push(format!("identifiers[{i}]: value must not be blank"));
        }
        check_item(&mut out, "identifiers", i, &ident.value);
    }
    for (i, subject) in inspected(&case.subjects) {
        if subject.trim().is_empty() {
            out.push(format!("subjects[{i}]: must not be blank"));
        }
        check_item(&mut out, "subjects", i, subject);
    }
    for (i, keyword) in inspected(&case.keywords) {
        if keyword.trim().is_empty() {
            out.push(format!("keywords[{i}]: must not be blank"));
        }
        check_item(&mut out, "keywords", i, keyword);
    }
    for (i, title) in inspected(&case.alternate_titles) {
        check_item(&mut out, "alternate_titles", i, title);
    }
    for (i, url) in inspected(&case.same_as) {
        check_item(&mut out, "same_as", i, url);
    }
    for (i, lang) in inspected(&case.in_language) {
        check_item(&mut out, "in_language", i, lang);
    }

    out
}

/// The entries a per-entry check actually inspects: at most
/// [`MAX_ARRAY_LEN`] of them, paired with their index.
///
/// An array over the cardinality cap is **already rejected** by its own
/// problem, so inspecting the tail decides nothing — while reporting a
/// problem for every entry turns a small request into a large response
/// (ten thousand blank entries produced ten thousand strings, all joined
/// into one `422` body). Bounding the *report* is the same
/// input-bounding rule as bounding the work (SEC-M1/SEC-M8).
///
/// Named rather than inlined as `.take(MAX_ARRAY_LEN)` at each call site
/// so a per-entry loop added later without it reads as different from the
/// ones that have it.
fn inspected<T>(items: &[T]) -> impl Iterator<Item = (usize, &T)> {
    items.iter().take(MAX_ARRAY_LEN).enumerate()
}

/// Push a problem when a scalar text `field` exceeds [`MAX_TEXT_LEN`]
/// Unicode scalar values.
fn check_text(out: &mut Vec<String>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        out.push(format!("{field}: exceeds {MAX_TEXT_LEN} characters"));
    }
}

/// Push a problem when an array `field` holds more than [`MAX_ARRAY_LEN`]
/// entries.
fn check_array(out: &mut Vec<String>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        out.push(format!("{field}: exceeds {MAX_ARRAY_LEN} entries"));
    }
}

/// Push a problem when the `index`-th entry of an array `field` exceeds
/// [`MAX_ITEM_LEN`] Unicode scalar values.
fn check_item(out: &mut Vec<String>, field: &str, index: usize, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        out.push(format!(
            "{field}[{index}]: exceeds {MAX_ITEM_LEN} characters"
        ));
    }
}

/// Accept an ISO-8601 calendar date as either a bare year (`YYYY`) or a
/// full date (`YYYY-MM-DD`). Month/day ranges and per-month day counts
/// are checked (so `2024-13-99` and `2024-02-30` are rejected).
#[must_use]
fn is_valid_iso_date(s: &str) -> bool {
    match s.len() {
        4 => s.bytes().all(|b| b.is_ascii_digit()),
        10 => is_valid_ymd(s),
        _ => false,
    }
}

/// Validate a strict `YYYY-MM-DD` string.
fn is_valid_ymd(s: &str) -> bool {
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let (Some(year), Some(month), Some(day)) = (
        parse_digits(&s[0..4]),
        parse_digits(&s[5..7]),
        parse_digits(&s[8..10]),
    ) else {
        return false;
    };
    if !(1..=12).contains(&month) || day < 1 {
        return false;
    }
    day <= days_in_month(year, month)
}

/// Parse an all-ASCII-digit slice into a `u32`, or `None` if any byte is
/// not a digit.
fn parse_digits(s: &str) -> Option<u32> {
    if s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// Number of days in `month` of `year` (Gregorian leap-year rules).
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Gregorian leap-year test.
fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use case_matcher::{CaseIdentifier, IdentifierScheme};

    /// Build a `Docket`-scheme `CaseIdentifier` with the given value, for
    /// the validation tests.
    fn ident(value: &str) -> CaseIdentifier {
        CaseIdentifier {
            scheme: IdentifierScheme::Docket,
            value: value.to_string(),
        }
    }

    /// Pins the all-valid baseline: a fully populated, well-formed case
    /// yields zero problems.
    #[test]
    fn valid_case_has_no_problems() {
        let case = Case {
            opened_date: Some("2024-01-31".into()),
            subjects: vec!["person:abc".into()],
            keywords: vec!["housing".into()],
            identifiers: vec![ident("CV-2024-001234")],
            ..Case::new("Housing benefit appeal")
        };
        assert!(problems(&case).is_empty());
    }

    /// Pins that a bare `YYYY` year is an accepted `opened_date` form.
    #[test]
    fn bare_year_is_a_valid_opened_date() {
        let case = Case {
            opened_date: Some("2024".into()),
            ..Case::new("Housing benefit appeal")
        };
        assert!(problems(&case).is_empty());
    }

    /// Pins the required-title rule: a whitespace-only title is the single
    /// `"title is required"` problem.
    #[test]
    fn blank_title_is_a_problem() {
        assert_eq!(
            problems(&Case::new("   ")),
            vec!["title is required".to_string()]
        );
    }

    /// Pins the date rejections: out-of-range, wrong-format, impossible
    /// (`2024-02-30`), non-zero-padded, non-date, and empty all fail.
    #[test]
    fn malformed_opened_dates_are_rejected() {
        for date in [
            "2024-13-99",
            "24/01/01",
            "2024-02-30",
            "2024-1-1",
            "not-a-date",
            "",
        ] {
            let case = Case {
                opened_date: Some(date.into()),
                ..Case::new("Housing benefit appeal")
            };
            assert_eq!(
                problems(&case).len(),
                1,
                "should reject opened_date {date:?}"
            );
        }
    }

    /// Pins the date acceptances, including the leap-day `2024-02-29`.
    #[test]
    fn valid_opened_dates_are_accepted() {
        for date in ["2024", "2024-01-01", "2024-02-29", "2000-12-31"] {
            let case = Case {
                opened_date: Some(date.into()),
                ..Case::new("Housing benefit appeal")
            };
            assert!(
                problems(&case).is_empty(),
                "should accept opened_date {date:?}"
            );
        }
    }

    /// Pins the per-identifier rule: a blank identifier value is one
    /// problem, reported with its index.
    #[test]
    fn blank_identifier_value_is_a_problem() {
        let case = Case {
            identifiers: vec![ident("   ")],
            ..Case::new("Housing benefit appeal")
        };
        let p = problems(&case);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("identifiers[0]"));
    }

    /// Pins the per-subject and per-keyword rules: a blank entry in either
    /// list is a problem, each reported with its index.
    #[test]
    fn blank_subject_and_keyword_are_problems() {
        let case = Case {
            subjects: vec!["ok".into(), "  ".into()],
            keywords: vec![String::new()],
            ..Case::new("Housing benefit appeal")
        };
        let p = problems(&case);
        assert_eq!(p.len(), 2);
        assert!(p.iter().any(|m| m.contains("subjects[1]")));
        assert!(p.iter().any(|m| m.contains("keywords[0]")));
    }

    /// Pins the report-everything contract: multiple problems (bad title,
    /// bad date, bad identifier) are all collected in one pass so the
    /// operator fixes them in a single round-trip.
    #[test]
    fn problems_reports_every_issue_with_index() {
        let case = Case {
            opened_date: Some("2024-13-01".into()),     // bad
            identifiers: vec![ident("ok"), ident(" ")], // [1] bad
            ..Case::new("")                             // blank title bad
        };
        let p = problems(&case);
        // title + opened_date + identifiers[1]
        assert_eq!(p.len(), 3);
        assert!(p.iter().any(|m| m.contains("title is required")));
        assert!(p.iter().any(|m| m.contains("opened_date")));
        assert!(p.iter().any(|m| m.contains("identifiers[1]")));
    }

    /// SEC-M1: an oversized scalar text field is exactly one problem.
    #[test]
    fn oversized_text_field_is_a_problem() {
        let case = Case::new("x".repeat(MAX_TEXT_LEN + 1));
        assert_eq!(
            problems(&case),
            vec![format!("title: exceeds {MAX_TEXT_LEN} characters")]
        );
    }

    /// SEC-M1: an over-long array is exactly one problem (its entries are
    /// each within `MAX_ITEM_LEN`).
    #[test]
    fn oversized_array_is_a_problem() {
        let case = Case {
            subjects: vec!["ok".to_string(); MAX_ARRAY_LEN + 1],
            ..Case::new("Housing benefit appeal")
        };
        assert_eq!(
            problems(&case),
            vec![format!("subjects: exceeds {MAX_ARRAY_LEN} entries")]
        );
    }

    /// SEC-M1: an oversized single entry inside an array is exactly one
    /// problem, reported with its index.
    #[test]
    fn oversized_array_item_is_a_problem() {
        let case = Case {
            subjects: vec!["ok".into(), "x".repeat(MAX_ITEM_LEN + 1)],
            ..Case::new("Housing benefit appeal")
        };
        assert_eq!(
            problems(&case),
            vec![format!("subjects[1]: exceeds {MAX_ITEM_LEN} characters")]
        );
    }

    /// SEC-M1: a large-but-within-caps record (every field at exactly its
    /// limit) is accepted — the caps reject only what exceeds them.
    #[test]
    fn within_caps_large_record_has_no_problems() {
        let case = Case {
            title: "x".repeat(MAX_TEXT_LEN),
            alternate_titles: vec!["a".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            subjects: vec!["s".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            keywords: vec!["k".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            identifiers: vec![ident(&"i".repeat(MAX_ITEM_LEN)); MAX_ARRAY_LEN],
            ..Case::new("placeholder")
        };
        assert!(problems(&case).is_empty());
    }
    /// SEC-M1: the *report* is bounded too, not just the work. A caller
    /// sending ten thousand blank entries used to get ten thousand
    /// problem strings back — a small request buying a large response.
    /// The cardinality problem alone is enough to reject the payload.
    #[test]
    fn an_oversized_array_of_blanks_does_not_produce_a_problem_per_entry() {
        let case = Case {
            subjects: vec![String::new(); 10_000],
            ..Case::new("Housing benefit appeal")
        };
        let out = problems(&case);
        assert!(
            out.len() <= MAX_ARRAY_LEN + 1,
            "expected at most the cardinality problem plus one per inspected entry, got {}",
            out.len()
        );
        assert!(
            out.contains(&format!("subjects: exceeds {MAX_ARRAY_LEN} entries")),
            "the cardinality problem must still be reported"
        );
    }
}
