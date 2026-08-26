//! Pure rules for the **OKR engine** — objectives, key results,
//! check-ins, and the scores derived from them (entity spec §5.9.2 /
//! FR-27). DB-free and exhaustively unit-tested.
//!
//! # Nothing here is stored
//!
//! Progress, objective score and plan score are computed on read. A
//! stored score can drift from the evidence it claims to summarise, and
//! nothing downstream can tell — the rule Smart Score already follows.
//!
//! # Four rules, each guarding a specific way of lying
//!
//! - **A key result without a metric is not a key result.** An
//!   objective with no measurable key result scores `null`, sorts last,
//!   and is never `0` — zero means *measured and failing*.
//! - **The baseline never moves.** `start_value` is captured once;
//!   progress measured from a moving baseline is not progress.
//! - **Confidence is never blended into the score.** A self-reported
//!   number and a measured one are different kinds of evidence, and
//!   averaging them makes the measured half unfalsifiable.
//! - **One currency.** A currency-valued key result is never compared
//!   across currencies; this crate converts nowhere.

use serde::{Deserialize, Serialize};

/// Basis-point scale: `10_000` is 100%.
pub const BASIS_POINTS: i64 = 10_000;

/// What a key result measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    /// A plain count or quantity.
    Number,
    /// A percentage, stored in basis points like every other ratio.
    Percent,
    /// Money in minor units of the key result's currency.
    Currency,
    /// Done or not: `0` or non-zero.
    Boolean,
}

/// Which way "better" runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Higher is better.
    Increase,
    /// Lower is better.
    Decrease,
    /// Stay within a tolerance band of the target.
    Maintain,
}

/// Why progress could not be computed. Reported beside the `None`, so a
/// reader never has to guess whether the figure is missing, undefined,
/// or genuinely zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unmeasurable {
    /// Start and target are equal, so there is no distance to travel.
    NoRange,
    /// `Maintain` was declared without a tolerance band.
    NoTolerance,
    /// The arithmetic would overflow.
    Overflow,
}

/// One key result, reduced to what scoring needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyResultFact {
    /// What it measures.
    pub metric: Metric,
    /// Which way better runs.
    pub direction: Direction,
    /// The baseline, captured at creation and never recomputed.
    pub start_value: i64,
    /// The target.
    pub target_value: i64,
    /// The latest observed value.
    pub current_value: i64,
    /// Band for [`Direction::Maintain`].
    pub tolerance: Option<i64>,
    /// ISO 4217 code for [`Metric::Currency`].
    pub currency: Option<String>,
}

/// Progress on one key result, in basis points, clamped to `0..=10_000`.
///
/// `Increase` runs start → target; `Decrease` inverts, so falling
/// toward a lower target is progress; `Maintain` scores full marks
/// inside its band and falls off outside it.
///
/// # Errors
/// Returns the [`Unmeasurable`] reason rather than a sentinel: a key
/// result whose start equals its target has no distance to travel, and
/// reporting `0` or `10_000` for that would both be inventions.
pub fn progress(kr: &KeyResultFact) -> Result<i64, Unmeasurable> {
    if kr.direction == Direction::Maintain {
        let Some(tolerance) = kr.tolerance else {
            return Err(Unmeasurable::NoTolerance);
        };
        let distance = kr
            .current_value
            .checked_sub(kr.target_value)
            .map(i64::abs)
            .ok_or(Unmeasurable::Overflow)?;
        return Ok(if distance <= tolerance {
            BASIS_POINTS
        } else {
            0
        });
    }

    let span = kr
        .target_value
        .checked_sub(kr.start_value)
        .ok_or(Unmeasurable::Overflow)?;
    if span == 0 {
        return Err(Unmeasurable::NoRange);
    }
    let travelled = kr
        .current_value
        .checked_sub(kr.start_value)
        .ok_or(Unmeasurable::Overflow)?;

    // The sign of `span` already encodes the direction of travel, so
    // `Decrease` needs no special case: a target below the baseline
    // gives a negative span, and moving down gives a negative
    // numerator. Their ratio is positive, which is what progress means.
    let ratio = travelled
        .checked_mul(BASIS_POINTS)
        .ok_or(Unmeasurable::Overflow)?
        / span;
    Ok(ratio.clamp(0, BASIS_POINTS))
}

/// One objective's score, in basis points: the mean of its key results'
/// progress.
///
/// `None` when it has **no measurable key result** — reported as
/// `unmeasured` and sorted last, never as `0`.
#[must_use]
pub fn objective_score(key_results: &[KeyResultFact]) -> Option<i64> {
    let measured: Vec<i64> = key_results
        .iter()
        .filter_map(|kr| progress(kr).ok())
        .collect();
    if measured.is_empty() {
        return None;
    }
    let total: i64 = measured.iter().sum();
    i64::try_from(measured.len())
        .ok()
        .filter(|count| *count > 0)
        .map(|count| total / count)
}

/// One objective's contribution to a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignedObjective {
    /// The objective's score, or `None` when unmeasured.
    pub score: Option<i64>,
    /// The alignment weight from `objective_links`.
    pub weight: i64,
}

/// A plan's score: its aligned objectives' scores, **weighted by the
/// existing alignment weight**.
///
/// Unmeasured objectives are **excluded from both halves** rather than
/// counted as zero — an objective nobody could measure must not drag a
/// plan down, and must not silently lift it either.
///
/// `None` when no aligned objective is measurable, or when every weight
/// is zero or negative.
#[must_use]
pub fn plan_score(objectives: &[AlignedObjective]) -> Option<i64> {
    let mut weighted = 0_i64;
    let mut total_weight = 0_i64;
    for objective in objectives {
        let (Some(score), true) = (objective.score, objective.weight > 0) else {
            continue;
        };
        weighted = weighted.checked_add(score.checked_mul(objective.weight)?)?;
        total_weight = total_weight.checked_add(objective.weight)?;
    }
    if total_weight == 0 {
        return None;
    }
    Some(weighted / total_weight)
}

/// Whether two currency-valued key results may be compared or summed.
///
/// This crate converts nowhere, so a mixed-currency roll-up is reported
/// as *no evidence* rather than silently added up — the restriction ROI
/// already carries.
#[must_use]
pub fn same_currency(key_results: &[KeyResultFact]) -> bool {
    let mut seen: Option<&str> = None;
    for kr in key_results
        .iter()
        .filter(|kr| kr.metric == Metric::Currency)
    {
        let currency = kr.currency.as_deref().unwrap_or_default();
        match seen {
            None => seen = Some(currency),
            Some(first) if first == currency => {}
            Some(_) => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kr(direction: Direction, start: i64, target: i64, current: i64) -> KeyResultFact {
        KeyResultFact {
            metric: Metric::Number,
            direction,
            start_value: start,
            target_value: target,
            current_value: current,
            tolerance: None,
            currency: None,
        }
    }

    /// Progress runs from the baseline to the target, and is clamped —
    /// overshooting a target is 100%, not 150%.
    #[test]
    fn progress_runs_from_baseline_to_target() {
        assert_eq!(progress(&kr(Direction::Increase, 0, 100, 0)), Ok(0));
        assert_eq!(progress(&kr(Direction::Increase, 0, 100, 50)), Ok(5_000));
        assert_eq!(progress(&kr(Direction::Increase, 0, 100, 100)), Ok(10_000));
        assert_eq!(
            progress(&kr(Direction::Increase, 0, 100, 150)),
            Ok(10_000),
            "overshoot is 100%, not 150%"
        );
        // Below the baseline is zero, not negative.
        assert_eq!(progress(&kr(Direction::Increase, 50, 100, 0)), Ok(0));
    }

    /// A `Decrease` key result makes progress by falling. Reducing
    /// defects from 100 to 25 against a target of 0 is 75% done.
    #[test]
    fn decrease_progresses_downward() {
        assert_eq!(progress(&kr(Direction::Decrease, 100, 0, 25)), Ok(7_500));
        assert_eq!(progress(&kr(Direction::Decrease, 100, 0, 100)), Ok(0));
        assert_eq!(progress(&kr(Direction::Decrease, 100, 0, 0)), Ok(10_000));
        // Going the wrong way is zero, never negative.
        assert_eq!(progress(&kr(Direction::Decrease, 100, 0, 200)), Ok(0));
    }

    /// `Maintain` scores full marks inside its band and nothing outside,
    /// and is unmeasurable without one — a band is what "maintain"
    /// means.
    #[test]
    fn maintain_needs_a_band() {
        let mut k = kr(Direction::Maintain, 0, 100, 103);
        assert_eq!(progress(&k), Err(Unmeasurable::NoTolerance));
        k.tolerance = Some(5);
        assert_eq!(progress(&k), Ok(10_000));
        k.current_value = 112;
        assert_eq!(progress(&k), Ok(0));
    }

    /// No distance to travel is **unmeasurable**, not 0% and not 100%:
    /// both would be inventions.
    #[test]
    fn a_zero_range_is_unmeasurable() {
        assert_eq!(
            progress(&kr(Direction::Increase, 100, 100, 100)),
            Err(Unmeasurable::NoRange)
        );
    }

    /// Untrusted values must not panic or overflow.
    #[test]
    fn extreme_values_are_total() {
        assert!(progress(&kr(Direction::Increase, i64::MIN, i64::MAX, 0)).is_err());
        assert!(progress(&kr(Direction::Increase, 0, 1, i64::MAX)).is_err());
        let mut k = kr(Direction::Maintain, 0, i64::MIN, i64::MAX);
        k.tolerance = Some(i64::MAX);
        assert!(progress(&k).is_err());
    }

    /// An objective with **no measurable key result** scores `None`,
    /// never `0` — the regression test against an objective that looks
    /// measured and failing when nobody measured it.
    #[test]
    fn an_unmeasurable_objective_is_none_not_zero() {
        assert_eq!(objective_score(&[]), None);
        // One key result, and it has no range to travel.
        assert_eq!(
            objective_score(&[kr(Direction::Increase, 5, 5, 5)]),
            None,
            "unmeasurable must not become 0%"
        );
    }

    /// An objective's score is the mean of the key results that *could*
    /// be measured; the unmeasurable ones are excluded rather than
    /// counted as zero.
    #[test]
    fn objective_score_excludes_the_unmeasurable() {
        let score = objective_score(&[
            kr(Direction::Increase, 0, 100, 100), // 100%
            kr(Direction::Increase, 0, 100, 0),   // 0%
        ]);
        assert_eq!(score, Some(5_000));

        let with_unmeasurable = objective_score(&[
            kr(Direction::Increase, 0, 100, 100), // 100%
            kr(Direction::Increase, 5, 5, 5),     // unmeasurable
        ]);
        assert_eq!(
            with_unmeasurable,
            Some(10_000),
            "the unmeasurable key result must not halve the score"
        );
    }

    /// A plan's score uses the **existing alignment weight**, and
    /// excludes unmeasured objectives from both halves.
    #[test]
    fn plan_score_is_weighted_by_alignment() {
        let score = plan_score(&[
            AlignedObjective {
                score: Some(10_000),
                weight: 3,
            },
            AlignedObjective {
                score: Some(0),
                weight: 1,
            },
        ]);
        assert_eq!(score, Some(7_500), "weighted, not a plain mean");

        let with_unmeasured = plan_score(&[
            AlignedObjective {
                score: Some(10_000),
                weight: 3,
            },
            AlignedObjective {
                score: None,
                weight: 100,
            },
        ]);
        assert_eq!(
            with_unmeasured,
            Some(10_000),
            "an unmeasured objective must not drag the plan down, however heavy"
        );

        assert_eq!(plan_score(&[]), None);
        assert_eq!(
            plan_score(&[AlignedObjective {
                score: Some(5_000),
                weight: 0
            }]),
            None,
            "a zero total weight is undefined, not zero"
        );
    }

    /// Mixed currencies are refused rather than compared, because this
    /// crate converts nowhere.
    #[test]
    fn mixed_currencies_are_not_comparable() {
        let money = |code: &str| KeyResultFact {
            metric: Metric::Currency,
            currency: Some(code.to_string()),
            ..kr(Direction::Increase, 0, 100, 50)
        };
        assert!(same_currency(&[money("GBP"), money("GBP")]));
        assert!(!same_currency(&[money("GBP"), money("EUR")]));
        // Non-currency key results never block a comparison.
        assert!(same_currency(&[kr(Direction::Increase, 0, 1, 1)]));
    }
}
