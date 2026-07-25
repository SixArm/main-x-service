//! Pure rules for the anonymous wellbeing pulse (WPM-R28): the score
//! scale, the survey window, and the k-floored aggregation (WPM-D20).
//! The k-anonymity floor is a **constant**, not configuration — a
//! floor a deployment can quietly lower to 1 is no floor at all.

use chrono::NaiveDate;

/// The k-anonymity floor: a cell (department or overall) with fewer
/// responses than this is suppressed — statistics AND count withheld.
pub const K_ANONYMITY: usize = 5;

/// The closed score scale (1 = struggling … 5 = thriving).
pub const SCORE_MIN: i32 = 1;
/// Upper bound of the score scale.
pub const SCORE_MAX: i32 = 5;

/// Whether a submitted score is on the scale.
#[must_use]
pub fn valid_score(score: i32) -> bool {
    (SCORE_MIN..=SCORE_MAX).contains(&score)
}

/// Whether a survey window is open on `today` (bounds inclusive).
#[must_use]
pub fn survey_open(
    active_from: Option<NaiveDate>,
    active_until: Option<NaiveDate>,
    today: NaiveDate,
) -> bool {
    active_from.is_none_or(|from| today >= from) && active_until.is_none_or(|until| today <= until)
}

/// One aggregated cell (a department, or the overall block).
#[derive(Debug, PartialEq)]
pub enum Cell {
    /// Below the k-floor: nothing is disclosed — not even the count.
    Suppressed,
    /// At or above the floor: the distribution and the mean.
    Disclosed {
        /// Response count (≥ [`K_ANONYMITY`]).
        count: usize,
        /// Responses per score, indexed `[score-1]` (1–5).
        distribution: [usize; 5],
        /// Mean score.
        mean: f64,
    },
}

/// Aggregate one cell of scores under the k-floor.
#[must_use]
#[allow(clippy::cast_precision_loss)] // display mean
pub fn aggregate_cell(scores: &[i32]) -> Cell {
    if scores.len() < K_ANONYMITY {
        return Cell::Suppressed;
    }
    let mut distribution = [0usize; 5];
    let mut total: i64 = 0;
    for &score in scores {
        // Out-of-scale rows cannot exist (validated at write), but a
        // pure function does not trust its caller: clamp, don't panic.
        let idx = score.clamp(SCORE_MIN, SCORE_MAX) - 1;
        #[allow(clippy::cast_sign_loss)] // clamped to 0..=4
        {
            distribution[idx as usize] += 1;
        }
        total += i64::from(score.clamp(SCORE_MIN, SCORE_MAX));
    }
    Cell::Disclosed {
        count: scores.len(),
        distribution,
        mean: total as f64 / scores.len() as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn score_scale_is_one_to_five() {
        assert!(valid_score(1) && valid_score(5));
        assert!(!valid_score(0) && !valid_score(6));
    }

    #[test]
    fn survey_window_is_inclusive() {
        let from = Some(date(2026, 7, 1));
        let until = Some(date(2026, 7, 31));
        assert!(survey_open(from, until, date(2026, 7, 1)));
        assert!(survey_open(from, until, date(2026, 7, 31)));
        assert!(!survey_open(from, until, date(2026, 8, 1)));
        assert!(
            survey_open(None, None, date(2026, 1, 1)),
            "unbounded is open"
        );
    }

    /// The k-floor: 4 responses disclose nothing (not even the count);
    /// 5 disclose count, distribution, and mean.
    #[test]
    fn k_floor_suppresses_below_five() {
        assert_eq!(aggregate_cell(&[5, 5, 5, 5]), Cell::Suppressed, "4 < k");
        match aggregate_cell(&[1, 2, 3, 4, 5]) {
            Cell::Disclosed {
                count,
                distribution,
                mean,
            } => {
                assert_eq!(count, 5);
                assert_eq!(distribution, [1, 1, 1, 1, 1]);
                assert!((mean - 3.0).abs() < f64::EPSILON);
            }
            Cell::Suppressed => panic!("5 responses meet the floor"),
        }
        assert_eq!(aggregate_cell(&[]), Cell::Suppressed, "empty is suppressed");
    }

    /// Out-of-scale rows (which the write path refuses) are clamped,
    /// never a panic or an index overflow.
    #[test]
    fn aggregate_never_panics_on_bad_rows() {
        match aggregate_cell(&[0, 99, -7, 3, 3]) {
            Cell::Disclosed {
                count,
                distribution,
                ..
            } => {
                assert_eq!(count, 5);
                assert_eq!(distribution.iter().sum::<usize>(), 5);
            }
            Cell::Suppressed => panic!("floor met"),
        }
    }
}
