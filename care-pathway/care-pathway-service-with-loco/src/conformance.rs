//! Conformance to the enrolled template (spec `13-tasks.md` T-14i):
//! per instance, the steps copied at enrolment
//! (`instance_steps.position`) against their completion order
//! (`done_on`) — skipped steps, adjacent declared pairs completed out
//! of order, steps completed after closure, and `escalation` events —
//! plus a cohort share of fully-conformant instances.
//!
//! **Against the template only, never a discovered model**
//! (`agents/share/overview.md`'s own triage of BNSSG's conformance
//! checking: "Adapt — against the template the instance was enrolled
//! on, never against a discovered model"). This module never looks at
//! what other instances on the same pathway did; it only compares one
//! instance's own completion timestamps against its own declared step
//! order. And **no penalty for extra events**: an `escalation` count
//! is reported alongside the ratio, never subtracted from it — a
//! journey may genuinely need more attention than its template
//! foresaw, and that is not the same fact as completing steps out of
//! order.

use serde::Serialize;

/// One declared step, as loaded from `instance_steps` — `done` is not
/// carried separately because this crate's step-completion handler
/// (`controllers::instances::complete_step`) always sets `done` and
/// `done_on` together; a step's completion is fully described by
/// whether `done_on_ms` is present.
#[derive(Debug, Clone, Copy)]
pub struct StepRecord {
    /// `instance_steps.position` — the template's declared order.
    pub position: i32,
    /// Epoch milliseconds of `done_on` (day-resolution), or `None` if
    /// not yet done.
    pub done_on_ms: Option<i64>,
}

/// A pair is comparable only when both its steps are done; an
/// undone endpoint makes the pair's order undefined, which is a
/// different fact from an *inverted* one and must never be counted as
/// such (the acceptance text's own "a skipped step is reported as
/// skipped, not as an inversion").
pub const VERDICT_IN_ORDER: &str = "in_order";
/// Both steps are done, but the later-declared one was completed
/// first.
pub const VERDICT_INVERTED: &str = "inverted";
/// At least one of the pair's two steps has no `done_on` yet.
pub const VERDICT_SKIPPED: &str = "skipped";

/// Fewer than two declared steps — there is no adjacent pair to score,
/// so the ratio is `None` rather than a division by zero standing in
/// for "no data".
pub const NO_PAIRS_REASON: &str = "fewer than two declared steps";

/// One adjacent declared pair's verdict.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PairVerdict {
    /// The earlier-declared step's position.
    pub from_position: i32,
    /// The later-declared step's position.
    pub to_position: i32,
    /// One of [`VERDICT_IN_ORDER`] / [`VERDICT_INVERTED`] / [`VERDICT_SKIPPED`].
    pub verdict: &'static str,
}

/// One instance's full conformance report.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Conformance {
    /// Total adjacent declared pairs (`steps.len().saturating_sub(1)`)
    /// — the ratio's denominator, fixed by the template regardless of
    /// completion state.
    pub declared_pairs: usize,
    /// Pairs verdicted [`VERDICT_IN_ORDER`] — the ratio's numerator.
    pub pairs_in_order: usize,
    /// `pairs_in_order / declared_pairs`, or `None` when
    /// `declared_pairs == 0` (see `reason`).
    pub ratio: Option<f64>,
    /// Set exactly when `ratio` is `None` — [`NO_PAIRS_REASON`] today,
    /// a closed vocabulary of one.
    pub reason: Option<&'static str>,
    /// Every adjacent pair's own verdict, in declared order.
    pub pairs: Vec<PairVerdict>,
    /// Declared positions with no `done_on` yet.
    pub skipped_positions: Vec<i32>,
    /// Declared positions completed strictly after the instance's own
    /// `closed_on` — always empty on an instance that never closed.
    pub completed_after_closure: Vec<i32>,
    /// Count of this instance's `escalation`-kind events. Carried
    /// alongside the ratio, never folded into it — see this module's
    /// own "no penalty for extra events" doc.
    pub escalation_events: usize,
}

/// Score one instance's conformance to its own declared step order.
///
/// `steps` need not arrive in position order; this sorts them first.
/// `closed_on_ms` is `None` for an instance that has never closed, in
/// which case `completed_after_closure` is always empty.
#[must_use]
pub fn conformance(
    mut steps: Vec<StepRecord>,
    closed_on_ms: Option<i64>,
    escalation_events: usize,
) -> Conformance {
    steps.sort_by_key(|s| s.position);

    let mut pairs = Vec::with_capacity(steps.len().saturating_sub(1));
    let mut pairs_in_order = 0usize;
    for w in steps.windows(2) {
        let (a, b) = (w[0], w[1]);
        let verdict = match (a.done_on_ms, b.done_on_ms) {
            (Some(x), Some(y)) if x <= y => {
                pairs_in_order += 1;
                VERDICT_IN_ORDER
            }
            (Some(_), Some(_)) => VERDICT_INVERTED,
            _ => VERDICT_SKIPPED,
        };
        pairs.push(PairVerdict {
            from_position: a.position,
            to_position: b.position,
            verdict,
        });
    }
    let declared_pairs = pairs.len();
    let (ratio, reason) = if declared_pairs == 0 {
        (None, Some(NO_PAIRS_REASON))
    } else {
        #[allow(clippy::cast_precision_loss)]
        // display ratio, cohorts are far below f64's exact-integer range
        let value = pairs_in_order as f64 / declared_pairs as f64;
        (Some(value), None)
    };

    let skipped_positions: Vec<i32> = steps
        .iter()
        .filter(|s| s.done_on_ms.is_none())
        .map(|s| s.position)
        .collect();

    let completed_after_closure: Vec<i32> = closed_on_ms.map_or_else(Vec::new, |closed| {
        steps
            .iter()
            .filter(|s| s.done_on_ms.is_some_and(|d| d > closed))
            .map(|s| s.position)
            .collect()
    });

    Conformance {
        declared_pairs,
        pairs_in_order,
        ratio,
        reason,
        pairs,
        skipped_positions,
        completed_after_closure,
        escalation_events,
    }
}

/// A cohort's fully-conformant share.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CohortConformance {
    /// Total instances in the cohort (matches the caller's own
    /// `instances` count).
    pub instances: usize,
    /// Instances with a defined ratio (`declared_pairs > 0`) — the
    /// share's denominator; an instance with fewer than two declared
    /// steps has nothing to be conformant *about*, so it is excluded
    /// rather than silently counted as either conformant or not.
    pub with_ratio: usize,
    /// Instances whose ratio is exactly `1.0`.
    pub fully_conformant: usize,
    /// `fully_conformant / with_ratio`, or `None` when `with_ratio == 0`.
    pub share: Option<f64>,
}

/// Roll up a cohort's per-instance ratios (one entry per instance,
/// `None` where [`conformance`] itself returned `None`).
#[must_use]
pub fn cohort_conformance(ratios: &[Option<f64>]) -> CohortConformance {
    let instances = ratios.len();
    let with_ratio = ratios.iter().filter(|r| r.is_some()).count();
    let fully_conformant = ratios
        .iter()
        .filter(|r| r.is_some_and(|v| (v - 1.0).abs() < f64::EPSILON))
        .count();
    let share = if with_ratio == 0 {
        None
    } else {
        #[allow(clippy::cast_precision_loss)]
        // display ratio, cohorts are far below f64's exact-integer range
        let value = fully_conformant as f64 / with_ratio as f64;
        Some(value)
    };
    CohortConformance {
        instances,
        with_ratio,
        fully_conformant,
        share,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(position: i32, done_on_ms: Option<i64>) -> StepRecord {
        StepRecord {
            position,
            done_on_ms,
        }
    }

    const DAY_MS: i64 = 86_400_000;

    /// Acceptance: completing steps in template order scores 1.0 with
    /// zero inversions.
    #[test]
    fn template_order_scores_one_with_zero_inversions() {
        let steps = vec![
            step(0, Some(0)),
            step(1, Some(DAY_MS)),
            step(2, Some(2 * DAY_MS)),
        ];
        let result = conformance(steps, None, 0);
        assert_eq!(result.declared_pairs, 2);
        assert_eq!(result.pairs_in_order, 2);
        assert_eq!(result.ratio, Some(1.0));
        assert_eq!(result.reason, None);
        assert!(
            result.pairs.iter().all(|p| p.verdict == VERDICT_IN_ORDER),
            "{:?}",
            result.pairs
        );
    }

    /// Acceptance: reverse order scores 0.
    #[test]
    fn reverse_order_scores_zero() {
        let steps = vec![
            step(0, Some(2 * DAY_MS)),
            step(1, Some(DAY_MS)),
            step(2, Some(0)),
        ];
        let result = conformance(steps, None, 0);
        assert_eq!(result.declared_pairs, 2);
        assert_eq!(result.pairs_in_order, 0);
        assert_eq!(result.ratio, Some(0.0));
        assert!(
            result.pairs.iter().all(|p| p.verdict == VERDICT_INVERTED),
            "{:?}",
            result.pairs
        );
    }

    /// Acceptance: a skipped step is reported as skipped, not as an
    /// inversion.
    #[test]
    fn a_skipped_step_is_skipped_not_inverted() {
        let steps = vec![step(0, Some(0)), step(1, None), step(2, Some(DAY_MS))];
        let result = conformance(steps, None, 0);
        assert_eq!(result.declared_pairs, 2);
        assert_eq!(result.pairs_in_order, 0);
        assert_eq!(result.skipped_positions, vec![1]);
        for pair in &result.pairs {
            assert_eq!(pair.verdict, VERDICT_SKIPPED, "{pair:?}");
        }
    }

    /// Acceptance: an instance with one declared step reports `null`
    /// (no pairs) with the reason.
    #[test]
    fn one_declared_step_reports_no_pairs() {
        let result = conformance(vec![step(0, Some(0))], None, 0);
        assert_eq!(result.declared_pairs, 0);
        assert_eq!(result.pairs_in_order, 0);
        assert_eq!(result.ratio, None);
        assert_eq!(result.reason, Some(NO_PAIRS_REASON));
        assert!(result.pairs.is_empty());
    }

    /// Zero declared steps is the same "no pairs" case, not a panic.
    #[test]
    fn zero_declared_steps_reports_no_pairs() {
        let result = conformance(Vec::new(), None, 0);
        assert_eq!(result.ratio, None);
        assert_eq!(result.reason, Some(NO_PAIRS_REASON));
    }

    /// Input order does not matter; declared position order does.
    #[test]
    fn unsorted_input_is_sorted_by_position_first() {
        let steps = vec![
            step(2, Some(2 * DAY_MS)),
            step(0, Some(0)),
            step(1, Some(DAY_MS)),
        ];
        let result = conformance(steps, None, 0);
        assert_eq!(result.ratio, Some(1.0));
        assert_eq!(
            result.pairs[0],
            PairVerdict {
                from_position: 0,
                to_position: 1,
                verdict: VERDICT_IN_ORDER
            }
        );
    }

    /// A tie (same day) is in order, not an inversion -- "no penalty"
    /// extends to same-day completion.
    #[test]
    fn a_same_day_tie_is_in_order() {
        let steps = vec![step(0, Some(DAY_MS)), step(1, Some(DAY_MS))];
        let result = conformance(steps, None, 0);
        assert_eq!(result.ratio, Some(1.0));
    }

    /// A step completed after the instance's own closure is flagged,
    /// independent of the ratio.
    #[test]
    fn a_step_completed_after_closure_is_flagged() {
        let steps = vec![step(0, Some(0)), step(1, Some(3 * DAY_MS))];
        let result = conformance(steps, Some(DAY_MS), 0);
        assert_eq!(result.completed_after_closure, vec![1]);
        // The ratio is unaffected -- this is a separate finding.
        assert_eq!(result.ratio, Some(1.0));
    }

    /// A step done on the closure day itself is not "after" it --
    /// day resolution cannot distinguish sub-day ordering.
    #[test]
    fn a_step_done_on_the_closure_day_is_not_after_it() {
        let steps = vec![step(0, Some(DAY_MS))];
        let result = conformance(steps, Some(DAY_MS), 0);
        assert!(result.completed_after_closure.is_empty());
    }

    /// No closure recorded at all -- never flagged, regardless of
    /// when steps were completed.
    #[test]
    fn no_closure_means_nothing_is_flagged_as_after_it() {
        let steps = vec![step(0, Some(100 * DAY_MS))];
        let result = conformance(steps, None, 0);
        assert!(result.completed_after_closure.is_empty());
    }

    /// Escalation events are carried, never subtracted from the ratio
    /// -- "no penalty for extra events".
    #[test]
    fn escalation_events_are_reported_not_penalized() {
        let steps = vec![step(0, Some(0)), step(1, Some(DAY_MS))];
        let result = conformance(steps, None, 3);
        assert_eq!(result.escalation_events, 3);
        assert_eq!(result.ratio, Some(1.0));
    }

    // -- cohort rollup ----------------------------------------------

    #[test]
    fn cohort_share_excludes_no_pair_instances_from_the_denominator() {
        let ratios = vec![Some(1.0), Some(1.0), Some(0.5), None];
        let result = cohort_conformance(&ratios);
        assert_eq!(result.instances, 4);
        assert_eq!(result.with_ratio, 3);
        assert_eq!(result.fully_conformant, 2);
        assert_eq!(result.share, Some(2.0 / 3.0));
    }

    #[test]
    fn cohort_share_is_none_when_no_instance_has_a_ratio() {
        let result = cohort_conformance(&[None, None]);
        assert_eq!(result.with_ratio, 0);
        assert_eq!(result.share, None);
    }

    #[test]
    fn an_empty_cohort_reports_zero_with_no_share() {
        let result = cohort_conformance(&[]);
        assert_eq!(result.instances, 0);
        assert_eq!(result.share, None);
    }
}
