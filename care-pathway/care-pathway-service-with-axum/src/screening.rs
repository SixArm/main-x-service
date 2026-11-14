//! Screening **reach and timing** over one pathway's cohort (spec §13
//! T-15c). Pure: the controller loads the rows and renders; this module
//! counts and decides what may be shown.
//!
//! The rule it exists to keep is the coverage rule of time-based analysis,
//! restated: a low positive rate over a fifth of the cohort is not a low
//! positive rate. So an unscreened instance is in the denominator, every
//! figure comes with its denominator, and nothing is shown that a small
//! cell would let a reader subtract back to one person.
//!
//! **What is counted.** Each instance's *first* screen on the named
//! instrument, measured from its clock start. Later screens are not part of
//! "reach": a repeat score is a trajectory, which is T-15d's question.
//! **Not an assessment of anyone**: nothing here is per person.

use chrono::NaiveDate;

use crate::instruments::Instrument;
use crate::suppression;

/// One instance's inputs.
#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// The day the clock started: the anchor the time to screen is
    /// measured from.
    pub anchor: NaiveDate,
    /// The day and score of its earliest screen on the instrument, if any.
    pub first_screen: Option<(NaiveDate, f64)>,
}

/// The counts, before suppression is decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counts {
    /// Instances in the cohort: the denominator of reach.
    pub enrolled: usize,
    /// Instances whose first screen falls inside the window.
    pub screened: usize,
    /// `enrolled - screened`. Includes a screen outside the window.
    pub unscreened: usize,
    /// Screened instances whose first score is at or above the
    /// instrument's threshold. `None` when the instrument has no threshold.
    pub positive: Option<usize>,
    /// Screened instances below the threshold. `None` as above.
    pub negative: Option<usize>,
    /// Days from anchor to first screen, ascending, for the screened.
    pub days_to_first_screen: Vec<i64>,
}

/// Count one cohort. `within_days` bounds how late a first screen may be
/// and still count as reach; `None` counts any screen.
#[must_use]
pub fn count(inputs: &[Input], instrument: &Instrument, within_days: Option<i64>) -> Counts {
    let mut screened = 0usize;
    let mut positive = 0usize;
    let mut negative = 0usize;
    let mut days = Vec::new();
    for input in inputs {
        let Some((on, score)) = input.first_screen else {
            continue;
        };
        // A score dated before the clock started is treated as day zero:
        // it was taken, and a negative wait would only be noise.
        let waited = (on - input.anchor).num_days().max(0);
        if within_days.is_some_and(|limit| waited > limit) {
            continue;
        }
        screened += 1;
        days.push(waited);
        if let Some(threshold) = instrument.screen_at {
            if score >= threshold {
                positive += 1;
            } else {
                negative += 1;
            }
        }
    }
    days.sort_unstable();
    let thresholded = instrument.screen_at.is_some();
    Counts {
        enrolled: inputs.len(),
        screened,
        unscreened: inputs.len() - screened,
        positive: thresholded.then_some(positive),
        negative: thresholded.then_some(negative),
        days_to_first_screen: days,
    }
}

/// Whether the figures must be withheld, **all together**.
///
/// Reach and positive share are suppressed together when *either* is
/// small. Hiding only the small cell would leave it recoverable by
/// subtraction from the denominator and the other cells, so every cell
/// goes when any one is small: the cohort, the screened, the unscreened,
/// and (where the instrument has a threshold) the positive and negative.
#[must_use]
pub fn withheld(counts: &Counts, min_cell_count: usize) -> bool {
    let small = |n: usize| n > 0 && n < min_cell_count;
    small(counts.enrolled)
        || small(counts.screened)
        || small(counts.unscreened)
        || counts.positive.is_some_and(small)
        || counts.negative.is_some_and(small)
}

/// [`withheld`] against the deployment's configured floor.
#[must_use]
pub fn withheld_for_deployment(counts: &Counts) -> bool {
    withheld(counts, suppression::min_cell_count())
}

/// A share as a fraction, or `None` over an empty denominator.
#[must_use]
#[allow(clippy::cast_precision_loss)] // counts far below 2^52
pub fn share(part: usize, whole: usize) -> Option<f64> {
    (whole > 0).then(|| part as f64 / whole as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, d).unwrap()
    }

    fn input(anchor: u32, screen: Option<(u32, f64)>) -> Input {
        Input {
            anchor: day(anchor),
            first_screen: screen.map(|(d, s)| (day(d), s)),
        }
    }

    #[test]
    fn an_unscreened_instance_is_in_the_denominator() {
        let phq9 = instruments::find("phq9").unwrap();
        let c = count(
            &[input(1, Some((3, 12.0))), input(1, None), input(1, None)],
            phq9,
            None,
        );
        assert_eq!((c.enrolled, c.screened, c.unscreened), (3, 1, 2));
        assert_eq!(share(c.screened, c.enrolled), Some(1.0 / 3.0));
    }

    #[test]
    fn positive_is_at_or_above_the_threshold_of_the_first_screen() {
        let phq9 = instruments::find("phq9").unwrap(); // threshold 10
        let c = count(
            &[
                input(1, Some((2, 10.0))),
                input(1, Some((2, 9.0))),
                input(1, Some((2, 20.0))),
            ],
            phq9,
            None,
        );
        assert_eq!((c.positive, c.negative), (Some(2), Some(1)));
    }

    #[test]
    fn an_instrument_without_a_threshold_has_no_positive_share() {
        let ies = instruments::find("ies_r").unwrap();
        let c = count(&[input(1, Some((2, 50.0)))], ies, None);
        assert_eq!((c.positive, c.negative), (None, None));
        assert_eq!(c.screened, 1);
    }

    #[test]
    fn a_late_first_screen_is_outside_the_window_and_counts_as_unscreened() {
        let phq9 = instruments::find("phq9").unwrap();
        let c = count(
            &[input(1, Some((4, 12.0))), input(1, Some((20, 12.0)))],
            phq9,
            Some(7),
        );
        assert_eq!((c.screened, c.unscreened), (1, 1));
        assert_eq!(c.days_to_first_screen, vec![3]);
    }

    #[test]
    fn a_score_dated_before_the_clock_is_day_zero() {
        let phq9 = instruments::find("phq9").unwrap();
        let c = count(&[input(10, Some((5, 3.0)))], phq9, None);
        assert_eq!(c.days_to_first_screen, vec![0]);
    }

    #[test]
    fn one_small_cell_withholds_everything() {
        let phq9 = instruments::find("phq9").unwrap();
        // 20 enrolled, 15 screened (3 positive: small), 5 unscreened.
        let mut inputs = Vec::new();
        for i in 0..15 {
            inputs.push(input(1, Some((2, if i < 3 { 15.0 } else { 2.0 }))));
        }
        for _ in 0..5 {
            inputs.push(input(1, None));
        }
        let c = count(&inputs, phq9, None);
        assert_eq!(c.positive, Some(3));
        assert!(
            withheld(&c, 10),
            "a small positive cell withholds reach too"
        );
        assert!(
            !withheld(&c, 3),
            "and nothing is withheld when none is small"
        );
    }

    #[test]
    fn an_empty_cohort_discloses_nothing_to_hide() {
        let phq9 = instruments::find("phq9").unwrap();
        let c = count(&[], phq9, None);
        assert!(!withheld(&c, 10));
        assert_eq!(share(0, 0), None);
    }
}
