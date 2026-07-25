//! Pure rules for wellbeing health-entitlement prompts (WPM-R25).
//!
//! Eligibility is evaluated over **non-clinical facts only** — an age
//! band plus department / job-title lists (WPM-D17): the predicate
//! vocabulary has no place to put a health status, so a cohort like
//! "immunosuppressed" is unrepresentable by construction. Age
//! arithmetic never panics; an unknown birth date makes an age-banded
//! rule **not** eligible (honest: unknown is not a match).

use chrono::{Datelike, NaiveDate};

/// The closed acknowledgement vocabulary: what an employee can say
/// about a prompt. An HR workflow fact — never a clinical status.
pub const RESPONSES: &[&str] = &["booked", "done", "declined", "dismissed"];

/// The closed rule kinds (WPM-R26): `health` (vaccination cohorts and
/// the like) and `benefit` (benefits awareness — EAP, eye tests,
/// cycle-to-work). One engine, one predicate vocabulary; the kind is
/// a label, not a second mechanism (WPM-D18).
pub const ENTITLEMENT_KINDS: &[&str] = &["health", "benefit"];

/// The most doses a course can declare (a multi-dose course drives the
/// single optional reminder).
pub const MAX_DOSES: i32 = 6;

/// The oldest age an age band may name (bounds-checks operator input).
pub const MAX_AGE: i32 = 130;

/// Whether a declared dose count is sensible (1 ..= [`MAX_DOSES`]).
#[must_use]
pub fn valid_doses(doses: i32) -> bool {
    (1..=MAX_DOSES).contains(&doses)
}

/// Whether an age band is well-formed: each bound in `0 ..= MAX_AGE`
/// and, when both are present, `min <= max`.
#[must_use]
pub fn valid_age_band(min_age: Option<i32>, max_age: Option<i32>) -> bool {
    let in_bounds = |age: i32| (0..=MAX_AGE).contains(&age);
    if min_age.is_some_and(|a| !in_bounds(a)) || max_age.is_some_and(|a| !in_bounds(a)) {
        return false;
    }
    match (min_age, max_age) {
        (Some(min), Some(max)) => min <= max,
        _ => true,
    }
}

/// Whole-year age on `on` for someone born `born`, or `None` when the
/// date is in the future of `on`. Never panics: leap-day birthdays
/// count from 1 March in a common year.
#[must_use]
pub fn age_on(born: NaiveDate, on: NaiveDate) -> Option<i32> {
    if born > on {
        return None;
    }
    let mut age = on.year() - born.year();
    let had_birthday = (on.month(), on.day()) >= (born.month(), born.day());
    if !had_birthday {
        age -= 1;
    }
    (age >= 0).then_some(age)
}

/// One rule's non-clinical predicates, borrowed from the stored row.
#[derive(Debug)]
pub struct Predicates<'a> {
    /// Inclusive minimum age, if age-banded.
    pub min_age: Option<i32>,
    /// Inclusive maximum age, if age-banded.
    pub max_age: Option<i32>,
    /// Department allow-list; empty ⇒ every department.
    pub departments: &'a [String],
    /// Job-title allow-list; empty ⇒ every job title.
    pub job_titles: &'a [String],
    /// First day the rule is active, if bounded.
    pub active_from: Option<NaiveDate>,
    /// Last day the rule is active, if bounded.
    pub active_until: Option<NaiveDate>,
}

/// Whether the rule is active on `today` (both bounds inclusive).
#[must_use]
pub fn active_on(predicates: &Predicates<'_>, today: NaiveDate) -> bool {
    predicates.active_from.is_none_or(|from| today >= from)
        && predicates.active_until.is_none_or(|until| today <= until)
}

/// Whether an employee is eligible under the rule on `today`:
/// the rule is active, the age band matches (an **unknown age fails a
/// banded rule** — unknown is not a match), and the department /
/// job-title lists (case-insensitive; empty = all) contain the
/// employee's values.
#[must_use]
pub fn eligible(
    predicates: &Predicates<'_>,
    age: Option<i32>,
    department: &str,
    job_title: &str,
    today: NaiveDate,
) -> bool {
    if !active_on(predicates, today) {
        return false;
    }
    if predicates.min_age.is_some() || predicates.max_age.is_some() {
        let Some(age) = age else { return false };
        if predicates.min_age.is_some_and(|min| age < min)
            || predicates.max_age.is_some_and(|max| age > max)
        {
            return false;
        }
    }
    let contains = |list: &[String], value: &str| {
        list.is_empty() || list.iter().any(|entry| entry.eq_ignore_ascii_case(value))
    };
    contains(predicates.departments, department) && contains(predicates.job_titles, job_title)
}

/// What an eligible employee should currently see for one rule, given
/// their acknowledgement state.
#[derive(Debug, PartialEq, Eq)]
pub enum PromptState {
    /// No acknowledgement yet — show the prompt.
    Prompt,
    /// A multi-dose course was acknowledged `booked`/`done` and the one
    /// optional reminder has not been sent — show it (once).
    Reminder,
    /// Acknowledged (or already reminded) — show nothing.
    Quiet,
}

/// Derive the [`PromptState`]: no acknowledgement ⇒ `Prompt`;
/// `booked`/`done` on a multi-dose course with no reminder yet ⇒
/// `Reminder` (exactly once); anything else — `declined`, `dismissed`,
/// single-dose, or already reminded — ⇒ `Quiet`. Declining never
/// re-prompts (WPM-R25: no recorded consequence, no nagging).
#[must_use]
pub fn prompt_state(doses: i32, response: Option<&str>, reminded: bool) -> PromptState {
    match response {
        None => PromptState::Prompt,
        Some("booked" | "done") if doses > 1 && !reminded => PromptState::Reminder,
        Some(_) => PromptState::Quiet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn open(departments: &[String]) -> Predicates<'_> {
        Predicates {
            min_age: None,
            max_age: None,
            departments,
            job_titles: &[],
            active_from: None,
            active_until: None,
        }
    }

    #[test]
    fn responses_are_a_closed_workflow_vocabulary() {
        assert_eq!(RESPONSES, &["booked", "done", "declined", "dismissed"]);
    }

    /// The kind vocabulary is closed (WPM-R26): a new prompting flavour
    /// is a spec change, not a stray string.
    #[test]
    fn kinds_are_a_closed_vocabulary() {
        assert_eq!(ENTITLEMENT_KINDS, &["health", "benefit"]);
    }

    #[test]
    fn doses_and_age_band_bounds() {
        assert!(valid_doses(1) && valid_doses(MAX_DOSES));
        assert!(!valid_doses(0) && !valid_doses(MAX_DOSES + 1));
        assert!(valid_age_band(Some(65), None));
        assert!(valid_age_band(Some(50), Some(79)));
        assert!(!valid_age_band(Some(79), Some(50)), "min above max");
        assert!(!valid_age_band(Some(-1), None) && !valid_age_band(None, Some(200)));
    }

    /// Whole-year age arithmetic, including the day-before-birthday
    /// boundary and a 29 February birthday in a common year — and it
    /// never panics on a future date.
    #[test]
    fn age_arithmetic_cannot_panic() {
        let born = date(1961, 7, 24);
        assert_eq!(age_on(born, date(2026, 7, 24)), Some(65), "birthday today");
        assert_eq!(age_on(born, date(2026, 7, 23)), Some(64), "day before");
        let leapling = date(1960, 2, 29);
        assert_eq!(age_on(leapling, date(2026, 2, 28)), Some(65));
        assert_eq!(age_on(leapling, date(2026, 3, 1)), Some(66));
        assert_eq!(age_on(date(2030, 1, 1), date(2026, 1, 1)), None, "future birth");
    }

    /// The NHS shingles shape: 65+ matches at exactly 65, not at 64,
    /// and an **unknown age fails the banded rule** (unknown ≠ match).
    #[test]
    fn age_band_matches_and_unknown_age_fails_banded_rules() {
        let banded = Predicates { min_age: Some(65), ..open(&[]) };
        let today = date(2026, 7, 24);
        assert!(eligible(&banded, Some(65), "engineering", "Engineer", today));
        assert!(!eligible(&banded, Some(64), "engineering", "Engineer", today));
        assert!(!eligible(&banded, None, "engineering", "Engineer", today), "unknown age");
        // An un-banded rule ignores the unknown age entirely.
        assert!(eligible(&open(&[]), None, "engineering", "Engineer", today));
    }

    /// Department / job-title lists: empty = everyone; matching is
    /// case-insensitive; a non-listed department fails.
    #[test]
    fn department_and_title_lists_scope_the_cohort() {
        let wards = vec!["Ward 7".to_string(), "ICU".to_string()];
        let scoped = open(&wards);
        let today = date(2026, 7, 24);
        assert!(eligible(&scoped, None, "ward 7", "Nurse", today), "case-insensitive");
        assert!(!eligible(&scoped, None, "finance", "Nurse", today));
        let titled = Predicates { job_titles: &wards, ..open(&[]) };
        assert!(!eligible(&titled, None, "finance", "Accountant", today));
    }

    /// The active window bounds prompting (both ends inclusive) —
    /// cohorts change year to year, so rules are dated configuration.
    #[test]
    fn active_window_is_inclusive() {
        let seasonal = Predicates {
            active_from: Some(date(2026, 9, 1)),
            active_until: Some(date(2027, 3, 31)),
            ..open(&[])
        };
        assert!(!eligible(&seasonal, None, "x", "y", date(2026, 8, 31)));
        assert!(eligible(&seasonal, None, "x", "y", date(2026, 9, 1)));
        assert!(eligible(&seasonal, None, "x", "y", date(2027, 3, 31)));
        assert!(!eligible(&seasonal, None, "x", "y", date(2027, 4, 1)));
    }

    /// The prompt machine: unacknowledged prompts; `booked`/`done` on a
    /// multi-dose course earns exactly one reminder; declining or
    /// dismissing is final (no nagging).
    #[test]
    fn prompt_state_machine() {
        assert_eq!(prompt_state(1, None, false), PromptState::Prompt);
        assert_eq!(prompt_state(2, Some("booked"), false), PromptState::Reminder);
        assert_eq!(prompt_state(2, Some("done"), false), PromptState::Reminder);
        assert_eq!(prompt_state(2, Some("booked"), true), PromptState::Quiet, "one reminder only");
        assert_eq!(prompt_state(1, Some("booked"), false), PromptState::Quiet, "single dose");
        assert_eq!(prompt_state(2, Some("declined"), false), PromptState::Quiet, "declining is final");
        assert_eq!(prompt_state(2, Some("dismissed"), false), PromptState::Quiet);
    }
}
