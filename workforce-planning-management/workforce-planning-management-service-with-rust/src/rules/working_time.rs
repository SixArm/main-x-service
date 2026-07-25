//! Pure rules for working-time guardrails (WPM-R27): the 48-hour
//! average over the 17-week reference window and the 11-hour daily
//! rest gap between shift assignments. Advisory only — these derive
//! flags, they gate nothing (WPM-D19). No I/O, no clock; arithmetic
//! never panics.

use chrono::{DateTime, Utc};

/// The UK Working Time Regulations reference window, in weeks.
pub const REFERENCE_WEEKS: i64 = 17;

/// The 48-hour average weekly ceiling, in minutes.
pub const MAX_AVG_WEEKLY_MINUTES: i64 = 48 * 60;

/// The 11-hour daily rest floor between shifts, in minutes.
pub const MIN_DAILY_REST_MINUTES: i64 = 11 * 60;

/// Average weekly minutes over `weeks`, or `None` when `weeks` is not
/// positive (a zero-width window has no average — `null`, never `0`).
#[must_use]
#[allow(clippy::cast_precision_loss)] // display average
pub fn weekly_average(total_minutes: i64, weeks: i64) -> Option<f64> {
    (weeks > 0).then(|| total_minutes as f64 / weeks as f64)
}

/// Whether the average weekly minutes exceed the 48-hour ceiling.
/// Integer comparison (`total > ceiling × weeks`) — no float edge.
#[must_use]
pub fn over_average(total_minutes: i64, weeks: i64) -> bool {
    weeks > 0 && total_minutes > MAX_AVG_WEEKLY_MINUTES.saturating_mul(weeks)
}

/// One rest-gap breach between two consecutive shift assignments.
#[derive(Debug, PartialEq, Eq)]
pub struct RestBreach {
    /// When the earlier shift ends.
    pub prev_end: DateTime<Utc>,
    /// When the next shift starts.
    pub next_start: DateTime<Utc>,
    /// The gap between them, in minutes (clamped at 0 for overlaps).
    pub gap_minutes: i64,
}

/// The rest-gap breaches in a set of shift `intervals`
/// (`(starts, ends)`; any order): sorted by start, each consecutive
/// pair with less than [`MIN_DAILY_REST_MINUTES`] between the earlier
/// end and the later start is a breach. Malformed intervals
/// (`end <= start`) are skipped rather than trusted.
#[must_use]
pub fn rest_breaches(intervals: &[(DateTime<Utc>, DateTime<Utc>)]) -> Vec<RestBreach> {
    let mut sorted: Vec<&(DateTime<Utc>, DateTime<Utc>)> =
        intervals.iter().filter(|(start, end)| end > start).collect();
    sorted.sort_by_key(|(start, _)| *start);
    sorted
        .windows(2)
        .filter_map(|pair| {
            let (_, prev_end) = *pair[0];
            let (next_start, _) = *pair[1];
            let gap_minutes = (next_start - prev_end).num_minutes().max(0);
            (gap_minutes < MIN_DAILY_REST_MINUTES).then_some(RestBreach {
                prev_end,
                next_start,
                gap_minutes,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        chrono::NaiveDate::from_ymd_opt(2026, 7, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_utc()
    }

    /// The average carries its terms: a zero-week window is `None`
    /// (null, never 0), and the value is minutes per week.
    #[test]
    fn weekly_average_terms() {
        assert_eq!(weekly_average(1700, 17), Some(100.0));
        assert_eq!(weekly_average(0, 17), Some(0.0));
        assert_eq!(weekly_average(1700, 0), None, "no window, no average");
    }

    /// The 48-hour flag is an integer comparison at the exact boundary:
    /// 48 h/week over 17 weeks is NOT over; one more minute is.
    #[test]
    fn over_average_boundary() {
        let ceiling = MAX_AVG_WEEKLY_MINUTES * REFERENCE_WEEKS;
        assert!(!over_average(ceiling, REFERENCE_WEEKS), "exactly 48 h is not over");
        assert!(over_average(ceiling + 1, REFERENCE_WEEKS));
        assert!(!over_average(i64::MAX, 0), "no window, no flag");
    }

    /// The rest-gap check: a 10-hour turnaround is a breach, an
    /// 11-hour one is not, unordered input is sorted first, an overlap
    /// clamps to 0, and a malformed interval is skipped.
    #[test]
    fn rest_breaches_matrix() {
        // 22:00 end (day 1) → 08:00 start (day 2) = 10 h ⇒ breach.
        let tight = vec![(at(1, 14), at(1, 22)), (at(2, 8), at(2, 16))];
        let breaches = rest_breaches(&tight);
        assert_eq!(breaches.len(), 1);
        assert_eq!(breaches[0].gap_minutes, 600);
        // 22:00 → 09:00 = 11 h exactly ⇒ no breach (floor is inclusive).
        let ok = vec![(at(1, 14), at(1, 22)), (at(2, 9), at(2, 17))];
        assert!(rest_breaches(&ok).is_empty());
        // Unordered input sorts before pairing.
        let unordered = vec![(at(2, 8), at(2, 16)), (at(1, 14), at(1, 22))];
        assert_eq!(rest_breaches(&unordered).len(), 1);
        // Overlapping shifts clamp the gap at 0.
        let overlap = vec![(at(1, 8), at(1, 18)), (at(1, 16), at(1, 23))];
        assert_eq!(rest_breaches(&overlap)[0].gap_minutes, 0);
        // A malformed (end <= start) interval is skipped, not trusted.
        let malformed = vec![(at(1, 22), at(1, 14)), (at(2, 8), at(2, 16))];
        assert!(rest_breaches(&malformed).is_empty());
    }
}
