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
