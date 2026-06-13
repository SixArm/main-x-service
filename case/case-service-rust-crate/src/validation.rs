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

use case_matcher::Case;

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
    if let Some(date) = &case.opened_date {
        if !is_valid_iso_date(date.trim()) {
            out.push(format!(
                "opened_date: {date:?} is not a valid ISO-8601 date"
            ));
        }
    }
    for (i, ident) in case.identifiers.iter().enumerate() {
        if ident.value.trim().is_empty() {
            out.push(format!("identifiers[{i}]: value must not be blank"));
        }
    }
    for (i, subject) in case.subjects.iter().enumerate() {
        if subject.trim().is_empty() {
            out.push(format!("subjects[{i}]: must not be blank"));
        }
    }
    for (i, keyword) in case.keywords.iter().enumerate() {
        if keyword.trim().is_empty() {
            out.push(format!("keywords[{i}]: must not be blank"));
        }
    }
    out
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

    fn ident(value: &str) -> CaseIdentifier {
        CaseIdentifier {
            scheme: IdentifierScheme::Docket,
            value: value.to_string(),
        }
    }

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

    #[test]
    fn bare_year_is_a_valid_opened_date() {
        let case = Case {
            opened_date: Some("2024".into()),
            ..Case::new("Housing benefit appeal")
        };
        assert!(problems(&case).is_empty());
    }

    #[test]
    fn blank_title_is_a_problem() {
        assert_eq!(
            problems(&Case::new("   ")),
            vec!["title is required".to_string()]
        );
    }

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
}
