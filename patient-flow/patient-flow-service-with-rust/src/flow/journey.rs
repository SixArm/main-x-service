//! Journey logic: `Red2Green` day rules, the DTOC clock, and length of
//! stay — pure functions, no clock of their own (spec
//! `patient-journey.md`).

use chrono::{DateTime, FixedOffset, NaiveDate};

use super::tokens;

/// Validate a `Red2Green` day entry: a known classification, ≤ 2 known
/// delay reasons, and reasons only on a red day (a green day carries
/// none by definition).
///
/// # Errors
///
/// A human-readable list of problems (empty ⇒ valid is expressed as
/// `Ok(())`).
pub fn validate_red_green(
    classification: &str,
    delay_reasons: &[String],
) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();
    if !tokens::is_token(tokens::RED_GREEN, classification) {
        problems.push(format!(
            "classification must be red or green, got {classification:?}"
        ));
    }
    if delay_reasons.len() > 2 {
        problems.push(format!(
            "at most 2 delay reasons per red day, got {}",
            delay_reasons.len()
        ));
    }
    for reason in delay_reasons {
        if !tokens::is_token(tokens::DELAY_REASONS, reason) {
            problems.push(format!("unknown delay reason {reason:?}"));
        }
    }
    if classification == "green" && !delay_reasons.is_empty() {
        problems.push("a green day carries no delay reasons".to_string());
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

/// Whether a stay is a **delayed transfer of care** at `now`: it is
/// discharge-ready, not discharged, and the grace period has passed —
/// the grace being **midnight (UTC) of the ready day** (spec default;
/// `PATIENT_FLOW_DTOC_GRACE=midnight`), i.e. a patient still in a bed
/// on any *later day* than the day they became ready counts.
#[must_use]
pub fn is_dtoc(
    discharge_ready_at: Option<DateTime<FixedOffset>>,
    discharged_at: Option<DateTime<FixedOffset>>,
    now: DateTime<FixedOffset>,
) -> bool {
    match (discharge_ready_at, discharged_at) {
        (Some(ready), None) => now.date_naive() > ready.date_naive(),
        _ => false,
    }
}

/// Whole days of a DTOC delay at `now` (0 when not delayed).
#[must_use]
pub fn dtoc_days(
    discharge_ready_at: Option<DateTime<FixedOffset>>,
    discharged_at: Option<DateTime<FixedOffset>>,
    now: DateTime<FixedOffset>,
) -> i64 {
    if is_dtoc(discharge_ready_at, discharged_at, now) {
        discharge_ready_at.map_or(0, |ready| {
            (now.date_naive() - ready.date_naive()).num_days()
        })
    } else {
        0
    }
}

/// Length of stay in whole days at `now` (or at discharge, once
/// discharged). Day-of-admission counts as day 0.
#[must_use]
pub fn length_of_stay_days(
    admitted_at: DateTime<FixedOffset>,
    discharged_at: Option<DateTime<FixedOffset>>,
    now: DateTime<FixedOffset>,
) -> i64 {
    let end = discharged_at.unwrap_or(now).date_naive();
    (end - admitted_at.date_naive()).num_days().max(0)
}

/// Whether an EDD is overdue at `today` (an EDD in the past is
/// surfaced separately, never silently rolled forward — spec
/// `capacity.md`).
#[must_use]
pub fn edd_overdue(edd: Option<NaiveDate>, today: NaiveDate) -> bool {
    edd.is_some_and(|d| d < today)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(y, m, d, h, 0, 0)
            .unwrap()
    }

    /// Red with ≤2 known reasons is valid; green with none is valid.
    #[test]
    fn red_green_valid_cases() {
        assert!(validate_red_green("red", &["awaiting_transport".to_string()]).is_ok());
        assert!(
            validate_red_green(
                "red",
                &[
                    "awaiting_transport".to_string(),
                    "awaiting_pharmacy".to_string()
                ]
            )
            .is_ok()
        );
        assert!(validate_red_green("red", &[]).is_ok()); // reason may be added later same-day
        assert!(validate_red_green("green", &[]).is_ok());
    }

    /// Unknown classification, >2 reasons, unknown reasons, and
    /// reasons-on-green are each rejected with a named problem.
    #[test]
    fn red_green_invalid_cases() {
        assert!(validate_red_green("amber", &[]).is_err());
        let three: Vec<String> = ["awaiting_transport", "awaiting_pharmacy", "other"]
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(validate_red_green("red", &three).is_err());
        assert!(validate_red_green("red", &["waiting_on_vibes".to_string()]).is_err());
        assert!(validate_red_green("green", &["other".to_string()]).is_err());
    }

    /// DTOC starts the day after discharge-ready: same-day is not
    /// DTOC; the next morning is; discharge stops the clock.
    #[test]
    fn dtoc_clock() {
        let ready = Some(at(2026, 7, 16, 14));
        assert!(!is_dtoc(ready, None, at(2026, 7, 16, 23)));
        assert!(is_dtoc(ready, None, at(2026, 7, 17, 0)));
        assert!(is_dtoc(ready, None, at(2026, 7, 19, 9)));
        assert_eq!(dtoc_days(ready, None, at(2026, 7, 19, 9)), 3);
        assert!(!is_dtoc(
            ready,
            Some(at(2026, 7, 17, 10)),
            at(2026, 7, 19, 9)
        ));
        assert!(!is_dtoc(None, None, at(2026, 7, 19, 9)));
        assert_eq!(dtoc_days(ready, None, at(2026, 7, 16, 23)), 0);
    }

    /// LOS counts whole days from admission to now/discharge, never
    /// negative.
    #[test]
    fn los_days() {
        let admitted = at(2026, 7, 10, 15);
        assert_eq!(length_of_stay_days(admitted, None, at(2026, 7, 10, 23)), 0);
        assert_eq!(length_of_stay_days(admitted, None, at(2026, 7, 17, 1)), 7);
        assert_eq!(
            length_of_stay_days(admitted, Some(at(2026, 7, 12, 9)), at(2026, 7, 17, 1)),
            2
        );
        assert_eq!(length_of_stay_days(admitted, None, at(2026, 7, 9, 1)), 0);
    }

    /// EDD overdue only when strictly before today.
    #[test]
    fn edd_overdue_is_strict() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 17).unwrap();
        assert!(edd_overdue(NaiveDate::from_ymd_opt(2026, 7, 16), today));
        assert!(!edd_overdue(NaiveDate::from_ymd_opt(2026, 7, 17), today));
        assert!(!edd_overdue(NaiveDate::from_ymd_opt(2026, 7, 18), today));
        assert!(!edd_overdue(None, today));
    }
}

// ---------------------------------------------------------------------
// The stitched-journey timeline contract
// ---------------------------------------------------------------------

/// Milliseconds in one day.
pub const DAY_MS: i64 = 86_400_000;

/// A stay's timeline, in the four numbers a stitched journey needs
/// (`care-pathway-service`'s `src/journey.rs` contract).
///
/// # Why a green day is value-adding time
///
/// Time-based analysis asks what share of an episode was *the work*.
/// `Red2Green` already answers exactly that question in the NHS's own
/// vocabulary: a **green** day moves the patient toward discharge, a
/// **red** day does not. So the value-adding time of a stay is its green
/// days, and no new judgement had to be invented to say so.
///
/// # Unclassified days count as non-value-adding
///
/// A stay spanning ten days with three classified reports the green
/// share of those three, not of the ten. That is deliberate and matches
/// the consuming service's own denominator rule: elapsed calendar time
/// is the denominator, and unrecorded time counts against you — because
/// the alternative rewards recording less. The figure is therefore a
/// **floor**, and [`StayTimeline::coverage_ratio`] says how much of the
/// stay it rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StayTimeline {
    /// Admission, epoch milliseconds.
    pub clock_start_ms: i64,
    /// Discharge, or `as_of` while the patient is still in.
    pub clock_stop_ms: i64,
    /// Elapsed span.
    pub lead_time_ms: i64,
    /// Green days, as milliseconds.
    pub value_time_ms: i64,
    /// Days classified red or green.
    pub classified_days: i64,
    /// Green days among them.
    pub green_days: i64,
    /// Whole days the stay spans (at least one — an admission and
    /// discharge on the same day is a day of care, not zero).
    pub span_days: i64,
}

impl StayTimeline {
    /// The share of the stay that carries a `Red2Green` classification.
    ///
    /// Reported so a consumer can tell a genuinely red stay from an
    /// unclassified one: both show little value-adding time, and only
    /// this distinguishes them.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // display ratio over a bounded count
    pub fn coverage_ratio(&self) -> Option<f64> {
        (self.span_days > 0)
            .then(|| (self.classified_days as f64 / self.span_days as f64).clamp(0.0, 1.0))
    }

    /// `unclassified` | `partial` | `classified`, so a caller cannot
    /// render "nobody filled this in" as "nothing valuable happened".
    #[must_use]
    pub fn confidence(&self) -> &'static str {
        match self.coverage_ratio() {
            None => "unclassified",
            Some(c) if c < 0.20 => "unclassified",
            Some(c) if c < 0.80 => "partial",
            Some(_) => "classified",
        }
    }
}

/// Derive a stay's timeline from its admission, its discharge (or
/// `now`), and its `Red2Green` classifications.
///
/// Pure — `now` is a parameter — so the whole matrix is unit-testable:
/// an open stay, a same-day discharge, an unclassified stay, and a
/// classification outside the stay's own span.
#[must_use]
pub fn stay_timeline(
    admitted_at: DateTime<FixedOffset>,
    discharged_at: Option<DateTime<FixedOffset>>,
    now: DateTime<FixedOffset>,
    classifications: &[(NaiveDate, String)],
) -> StayTimeline {
    let end = discharged_at.unwrap_or(now);
    let clock_start_ms = admitted_at.timestamp_millis();
    // A clock that ends before it starts is a data error, not a
    // negative duration.
    let clock_stop_ms = end.timestamp_millis().max(clock_start_ms);
    let lead_time_ms = clock_stop_ms - clock_start_ms;

    // A stay that opens and closes on one day is one day of care, not
    // zero — the day count is inclusive.
    let span_days = length_of_stay_days(admitted_at, discharged_at, now) + 1;

    let (start_day, end_day) = (admitted_at.date_naive(), end.date_naive());
    let mut classified_days = 0i64;
    let mut green_days = 0i64;
    let mut seen: std::collections::BTreeSet<NaiveDate> = std::collections::BTreeSet::new();
    for (day, classification) in classifications {
        // A classification outside the stay's own span is ignored
        // rather than counted: it would push coverage above 1 and
        // credit the stay with a day it did not have.
        if *day < start_day || *day > end_day {
            continue;
        }
        // One row per day, whatever the data says. A duplicated day
        // must not count twice.
        if !seen.insert(*day) {
            continue;
        }
        classified_days += 1;
        if classification == "green" {
            green_days += 1;
        }
    }

    StayTimeline {
        clock_start_ms,
        clock_stop_ms,
        lead_time_ms,
        // Capped at the elapsed span: more green days than the stay
        // lasted would put the consuming ratio above 1.
        value_time_ms: (green_days * DAY_MS).min(lead_time_ms.max(0)),
        classified_days,
        green_days,
        span_days,
    }
}

#[cfg(test)]
mod timeline_tests {
    use super::*;

    fn at(day: u32, hour: u32) -> DateTime<FixedOffset> {
        format!("2026-03-{day:02}T{hour:02}:00:00+00:00")
            .parse()
            .expect("timestamp")
    }

    fn d(day: u32) -> NaiveDate {
        format!("2026-03-{day:02}").parse().expect("date")
    }

    fn green(day: u32) -> (NaiveDate, String) {
        (d(day), "green".to_string())
    }

    fn red(day: u32) -> (NaiveDate, String) {
        (d(day), "red".to_string())
    }

    #[test]
    fn green_days_are_the_value_adding_time() {
        // Admitted the 1st, discharged the 5th: a five-day span with
        // two green days.
        let t = stay_timeline(
            at(1, 9),
            Some(at(5, 15)),
            at(9, 0),
            &[green(1), red(2), red(3), green(4), red(5)],
        );
        assert_eq!(t.span_days, 5, "inclusive of both ends");
        assert_eq!(t.classified_days, 5);
        assert_eq!(t.green_days, 2);
        assert_eq!(t.value_time_ms, 2 * DAY_MS);
        assert_eq!(t.confidence(), "classified");
        assert_eq!(t.coverage_ratio(), Some(1.0));
    }

    #[test]
    fn an_unclassified_stay_says_so_rather_than_reading_as_all_waste() {
        // Nobody filled in the board. That looks identical to a stay of
        // pure red days unless the confidence says otherwise — and the
        // two call for completely different responses.
        let t = stay_timeline(at(1, 9), Some(at(11, 9)), at(12, 0), &[]);
        assert_eq!(t.value_time_ms, 0);
        assert_eq!(t.classified_days, 0);
        assert_eq!(t.coverage_ratio(), Some(0.0));
        assert_eq!(t.confidence(), "unclassified");

        // A genuinely red stay reports the same zero but is *classified*.
        let all_red: Vec<(NaiveDate, String)> = (1..=11).map(red).collect();
        let red_stay = stay_timeline(at(1, 9), Some(at(11, 9)), at(12, 0), &all_red);
        assert_eq!(red_stay.value_time_ms, 0);
        assert_eq!(red_stay.confidence(), "classified");
    }

    #[test]
    fn a_partly_classified_stay_reports_a_floor() {
        // Three of eleven days classified. The green share is of the
        // three, and the unclassified eight count as non-value-adding —
        // the same denominator rule the consuming service applies.
        let t = stay_timeline(
            at(1, 9),
            Some(at(11, 9)),
            at(12, 0),
            &[green(1), green(2), red(3)],
        );
        assert_eq!(t.value_time_ms, 2 * DAY_MS);
        assert_eq!(t.confidence(), "partial");
        assert!(t.coverage_ratio().unwrap_or(1.0) < 0.5);
    }

    #[test]
    fn an_open_stay_runs_to_now() {
        let t = stay_timeline(at(1, 0), None, at(4, 0), &[green(1), green(2)]);
        assert_eq!(t.lead_time_ms, 3 * DAY_MS);
        assert_eq!(t.clock_stop_ms, at(4, 0).timestamp_millis());
        assert_eq!(t.value_time_ms, 2 * DAY_MS);
    }

    #[test]
    fn a_same_day_stay_is_one_day_not_zero() {
        // Admitted and discharged the same day is a day of care. A zero
        // span would make every ratio undefined.
        let t = stay_timeline(at(3, 8), Some(at(3, 20)), at(4, 0), &[green(3)]);
        assert_eq!(t.span_days, 1);
        assert_eq!(t.coverage_ratio(), Some(1.0));
        // Value time is capped at the elapsed span: twelve hours, not a
        // whole green day, or the consuming ratio would exceed 1.
        assert_eq!(t.value_time_ms, t.lead_time_ms);
        assert!(t.value_time_ms < DAY_MS);
    }

    #[test]
    fn a_classification_outside_the_stay_is_ignored() {
        // It would push coverage above 1 and credit the stay with a day
        // it did not have.
        let t = stay_timeline(
            at(5, 9),
            Some(at(6, 9)),
            at(9, 0),
            &[green(1), green(5), green(20)],
        );
        assert_eq!(t.classified_days, 1, "only the in-span day counts");
        assert_eq!(t.green_days, 1);
        assert!(t.coverage_ratio().unwrap_or(9.0) <= 1.0);
    }

    #[test]
    fn a_duplicated_day_counts_once() {
        let t = stay_timeline(
            at(1, 0),
            Some(at(3, 0)),
            at(4, 0),
            &[green(1), green(1), red(1)],
        );
        assert_eq!(t.classified_days, 1);
        assert_eq!(t.green_days, 1);
    }

    #[test]
    fn a_reversed_clock_is_zero_not_negative() {
        // Bad data must not produce a negative duration that flows into
        // a stitched journey's arithmetic.
        let t = stay_timeline(at(9, 0), Some(at(1, 0)), at(9, 0), &[]);
        assert_eq!(t.lead_time_ms, 0);
        assert_eq!(t.value_time_ms, 0);
        assert!(t.clock_stop_ms >= t.clock_start_ms);
    }
}
