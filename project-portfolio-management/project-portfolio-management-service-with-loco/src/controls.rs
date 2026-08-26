//! Pure rules for the **Controlling process** — set a standard,
//! measure, compare, act. DB-free and exhaustively unit-tested (entity
//! spec §5.9.8 / FR-38, FR-39).
//!
//! The Controlling phase (§1.5) was a name with no mechanism under it.
//! A control is the loop that gives it one, and the three **timings**
//! are load-bearing rather than taxonomy: they decide what a *failing*
//! control is allowed to do.
//!
//! - **Feedforward** acts before the work and may **block**.
//! - **Concurrent** acts during and may warn or escalate, but must
//!   never silently undo the operator's action (the invariant the
//!   automation engine already holds).
//! - **Feedback** acts after and may only **record** — the work it
//!   judges is finished, and a control that rewrites history is not a
//!   control.
//!
//! Two honesty rules carried from the rest of this crate:
//! `Unmeasured` is a **third verdict**, never a quiet pass; and a
//! failing reading with no action and no explicit acceptance is
//! **unanswered**, because "fix problems" is the fourth step of the
//! process and a control that only measures is half-built.

use serde::{Deserialize, Serialize};

/// When a control acts, which fixes what it may do on failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    /// Before the work: may block.
    Feedforward,
    /// During the work: may warn and escalate, never silently undo.
    Concurrent,
    /// After the work: may only record.
    Feedback,
}

/// Every timing, in process order, so a coverage report can show each
/// one even at zero.
pub const TIMINGS: [Timing; 3] = [Timing::Feedforward, Timing::Concurrent, Timing::Feedback];

/// What a failing control of this timing is permitted to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
    /// Refuse the write, naming the control.
    Block,
    /// Warn and escalate; the operator's action stands.
    Warn,
    /// Record only.
    Record,
}

/// How a reading is compared with its standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparator {
    /// Value must be at least the target.
    AtLeast,
    /// Value must be at most the target.
    AtMost,
    /// Value must be within `tolerance` of the target, either side.
    Within,
    /// Value must equal the target exactly.
    Equals,
}

/// The outcome of comparing one reading with its standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Met the standard.
    Pass,
    /// Did not meet the standard.
    Fail,
    /// No value to compare. **Never a pass.**
    Unmeasured,
}

/// The standard a control holds work to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standard {
    /// The metric's name, which must be one the service produces.
    pub metric: String,
    /// The target, in the metric's own units (minor units for money,
    /// basis points for ratios — no float, as everywhere else).
    pub target_value: i64,
    /// How the value is compared with the target.
    pub comparator: Comparator,
    /// Required by [`Comparator::Within`], ignored otherwise.
    pub tolerance: Option<i64>,
}

/// What a failing control of `timing` may do.
#[must_use]
pub const fn permitted_response(timing: Timing) -> Response {
    match timing {
        Timing::Feedforward => Response::Block,
        Timing::Concurrent => Response::Warn,
        Timing::Feedback => Response::Record,
    }
}

/// Whether a control may refuse a write. Only feedforward may: acting
/// before the fact is the entire purpose of that timing, and letting a
/// feedback control block would let a judgement about finished work
/// reject new work.
#[must_use]
pub const fn may_block(timing: Timing) -> bool {
    matches!(permitted_response(timing), Response::Block)
}

/// Why a control cannot be registered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invalid {
    /// The metric named is not one this service produces. Refused at
    /// **write**, so a control can never sit permanently unmeasurable —
    /// a check nobody can evaluate reads exactly like one that passes.
    UnknownMetric(String),
    /// `Within` needs a tolerance.
    MissingTolerance,
    /// A tolerance cannot be negative.
    NegativeTolerance,
    /// A control needs a name.
    BlankName,
}

/// Validate a control against the metrics this service actually
/// produces.
///
/// # Errors
/// Returns every problem found, not merely the first, so one round trip
/// tells the operator everything wrong with the control.
pub fn validate(
    name: &str,
    standard: &Standard,
    known_metrics: &[&str],
) -> Result<(), Vec<Invalid>> {
    let mut problems = Vec::new();
    if name.trim().is_empty() {
        problems.push(Invalid::BlankName);
    }
    if !known_metrics.contains(&standard.metric.as_str()) {
        problems.push(Invalid::UnknownMetric(standard.metric.clone()));
    }
    match (standard.comparator, standard.tolerance) {
        (Comparator::Within, None) => problems.push(Invalid::MissingTolerance),
        (_, Some(t)) if t < 0 => problems.push(Invalid::NegativeTolerance),
        _ => {}
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

/// Compare one reading with its standard.
///
/// `None` is [`Verdict::Unmeasured`] — a third verdict, never a pass.
/// The gap is `value - target` for the ordered comparators and the
/// distance outside the band for [`Comparator::Within`]; it is `None`
/// when there is nothing to compare.
#[must_use]
pub fn compare(standard: &Standard, value: Option<i64>) -> (Verdict, Option<i64>) {
    let Some(value) = value else {
        return (Verdict::Unmeasured, None);
    };
    let gap = value.checked_sub(standard.target_value);
    match standard.comparator {
        Comparator::AtLeast => {
            let verdict = if value >= standard.target_value {
                Verdict::Pass
            } else {
                Verdict::Fail
            };
            (verdict, gap)
        }
        Comparator::AtMost => {
            let verdict = if value <= standard.target_value {
                Verdict::Pass
            } else {
                Verdict::Fail
            };
            (verdict, gap)
        }
        Comparator::Equals => {
            let verdict = if value == standard.target_value {
                Verdict::Pass
            } else {
                Verdict::Fail
            };
            (verdict, gap)
        }
        Comparator::Within => {
            // A `Within` control with no tolerance cannot be registered
            // (`validate`), so a missing one here is a stored record
            // that predates the rule: report it unmeasured rather than
            // inventing a band.
            let Some(tolerance) = standard.tolerance else {
                return (Verdict::Unmeasured, None);
            };
            let Some(distance) = gap.map(i64::abs) else {
                return (Verdict::Unmeasured, None);
            };
            if distance <= tolerance {
                (Verdict::Pass, Some(0))
            } else {
                (Verdict::Fail, distance.checked_sub(tolerance))
            }
        }
    }
}

/// One reading as stored, reduced to what coverage needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadingFact {
    /// Days since the reading was taken.
    pub age_days: i64,
    /// Its verdict.
    pub verdict: Verdict,
    /// Whether a failing reading has an action or an explicit
    /// acceptance recorded against it.
    pub answered: bool,
}

/// One control as stored, reduced to what coverage needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlFact {
    /// When it acts.
    pub timing: Timing,
    /// How often a reading is expected, in days. `None` means no
    /// cadence is declared, so a reading can never be *overdue*.
    pub cadence_days: Option<i64>,
    /// Its readings, newest first.
    pub readings: Vec<ReadingFact>,
    /// Disabled controls are counted but never reported as overdue.
    pub enabled: bool,
}

/// What is **not** being controlled — the question the register exists
/// to answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Controls registered.
    pub total: usize,
    /// Enabled controls that have **never** produced a reading. The
    /// most important number here: a control that never fired is
    /// indistinguishable from one that always passes.
    pub never_read: usize,
    /// Enabled controls whose newest reading is older than their
    /// declared cadence.
    pub overdue: usize,
    /// Failing readings with neither an action nor an explicit
    /// acceptance.
    pub unanswered_failures: usize,
    /// Readings that had no value to compare.
    pub unmeasured: usize,
    /// Count per timing, **every timing present even at zero** — an
    /// empty cell is a finding, not a row to omit.
    pub by_timing: Vec<(Timing, usize)>,
    /// Pass rate in basis points over readings that were actually
    /// measured, or `None` when none were. Unmeasured readings are
    /// excluded from both halves rather than counted as either.
    pub pass_rate_basis_points: Option<i64>,
}

/// Roll controls up into the coverage report.
#[must_use]
pub fn coverage(controls: &[ControlFact]) -> Coverage {
    let mut never_read = 0;
    let mut overdue = 0;
    let mut unanswered_failures = 0;
    let mut unmeasured = 0;
    let mut passed = 0_i64;
    let mut measured = 0_i64;

    let mut by_timing: Vec<(Timing, usize)> = TIMINGS.iter().map(|timing| (*timing, 0)).collect();

    for control in controls {
        if let Some(entry) = by_timing.iter_mut().find(|(t, _)| *t == control.timing) {
            entry.1 += 1;
        }

        if control.enabled && control.readings.is_empty() {
            never_read += 1;
        }

        if control.enabled
            && let (Some(cadence), Some(newest)) = (control.cadence_days, control.readings.first())
            && newest.age_days > cadence
        {
            overdue += 1;
        }

        for reading in &control.readings {
            match reading.verdict {
                Verdict::Pass => {
                    passed += 1;
                    measured += 1;
                }
                Verdict::Fail => {
                    measured += 1;
                    if !reading.answered {
                        unanswered_failures += 1;
                    }
                }
                Verdict::Unmeasured => unmeasured += 1,
            }
        }
    }

    Coverage {
        total: controls.len(),
        never_read,
        overdue,
        unanswered_failures,
        unmeasured,
        by_timing,
        pass_rate_basis_points: if measured == 0 {
            None
        } else {
            passed.checked_mul(10_000).map(|scaled| scaled / measured)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRICS: &[&str] = &["flow_efficiency", "dipp", "adoption_rate"];

    fn standard(comparator: Comparator, target: i64, tolerance: Option<i64>) -> Standard {
        Standard {
            metric: "flow_efficiency".to_string(),
            target_value: target,
            comparator,
            tolerance,
        }
    }

    /// The timing decides the permitted response. This is the rule the
    /// whole model rests on.
    #[test]
    fn timing_fixes_what_a_failure_may_do() {
        assert_eq!(permitted_response(Timing::Feedforward), Response::Block);
        assert_eq!(permitted_response(Timing::Concurrent), Response::Warn);
        assert_eq!(permitted_response(Timing::Feedback), Response::Record);
        assert!(may_block(Timing::Feedforward));
        assert!(!may_block(Timing::Concurrent));
        assert!(!may_block(Timing::Feedback));
    }

    /// A control naming a metric the service does not produce is
    /// refused at write — otherwise it reads `Unmeasured` forever,
    /// which is indistinguishable from passing.
    #[test]
    fn unknown_metric_is_refused_at_write() {
        let mut s = standard(Comparator::AtLeast, 1_500, None);
        s.metric = "invented".to_string();
        let problems = validate("Flow floor", &s, METRICS).unwrap_err();
        assert_eq!(
            problems,
            vec![Invalid::UnknownMetric("invented".to_string())]
        );
    }

    /// Every problem is reported at once, not just the first.
    #[test]
    fn validation_reports_every_problem() {
        let mut s = standard(Comparator::Within, 100, None);
        s.metric = "invented".to_string();
        let problems = validate("  ", &s, METRICS).unwrap_err();
        assert!(problems.contains(&Invalid::BlankName));
        assert!(problems.contains(&Invalid::UnknownMetric("invented".to_string())));
        assert!(problems.contains(&Invalid::MissingTolerance));
    }

    /// A valid control validates.
    #[test]
    fn valid_control_passes() {
        assert!(
            validate(
                "Flow floor",
                &standard(Comparator::AtLeast, 1_500, None),
                METRICS
            )
            .is_ok()
        );
        assert!(validate("Band", &standard(Comparator::Within, 100, Some(5)), METRICS).is_ok());
    }

    /// The four comparators compare, and the gap is reported with the
    /// verdict so a reader can check it.
    #[test]
    fn comparators_compare() {
        assert_eq!(
            compare(&standard(Comparator::AtLeast, 1_500, None), Some(2_000)),
            (Verdict::Pass, Some(500))
        );
        assert_eq!(
            compare(&standard(Comparator::AtLeast, 1_500, None), Some(1_000)),
            (Verdict::Fail, Some(-500))
        );
        assert_eq!(
            compare(&standard(Comparator::AtMost, 100, None), Some(80)),
            (Verdict::Pass, Some(-20))
        );
        assert_eq!(
            compare(&standard(Comparator::Equals, 7, None), Some(7)),
            (Verdict::Pass, Some(0))
        );
        assert_eq!(
            compare(&standard(Comparator::Within, 100, Some(5)), Some(103)),
            (Verdict::Pass, Some(0))
        );
        assert_eq!(
            compare(&standard(Comparator::Within, 100, Some(5)), Some(112)),
            (Verdict::Fail, Some(7))
        );
    }

    /// **No value is `Unmeasured`, never a pass** — the regression test
    /// against a control that looks green because nobody measured it.
    #[test]
    fn absent_value_is_unmeasured_not_pass() {
        for comparator in [
            Comparator::AtLeast,
            Comparator::AtMost,
            Comparator::Equals,
            Comparator::Within,
        ] {
            assert_eq!(
                compare(&standard(comparator, 100, Some(5)), None),
                (Verdict::Unmeasured, None)
            );
        }
    }

    /// Untrusted input must not panic (security invariant 2).
    #[test]
    fn extreme_values_never_panic() {
        let s = standard(Comparator::Within, i64::MIN, Some(i64::MAX));
        let _ = compare(&s, Some(i64::MAX));
        let s = standard(Comparator::AtLeast, i64::MIN, None);
        let _ = compare(&s, Some(i64::MAX));
    }

    fn control(
        timing: Timing,
        cadence: Option<i64>,
        readings: &[(i64, Verdict, bool)],
    ) -> ControlFact {
        ControlFact {
            timing,
            cadence_days: cadence,
            readings: readings
                .iter()
                .map(|(age_days, verdict, answered)| ReadingFact {
                    age_days: *age_days,
                    verdict: *verdict,
                    answered: *answered,
                })
                .collect(),
            enabled: true,
        }
    }

    /// Coverage reports what is **not** controlled: a control that has
    /// never fired, one whose cadence has lapsed, and a failure nobody
    /// answered.
    #[test]
    fn coverage_reports_the_gaps() {
        let c = coverage(&[
            control(Timing::Feedforward, Some(7), &[]), // never read
            control(Timing::Concurrent, Some(7), &[(30, Verdict::Pass, true)]), // overdue
            control(Timing::Feedback, Some(30), &[(1, Verdict::Fail, false)]), // unanswered
            control(Timing::Feedback, None, &[(999, Verdict::Pass, true)]), // no cadence: never overdue
        ]);
        assert_eq!(c.total, 4);
        assert_eq!(c.never_read, 1);
        assert_eq!(c.overdue, 1);
        assert_eq!(c.unanswered_failures, 1);
    }

    /// Every timing appears even at zero — an empty cell is a finding,
    /// not a row to omit.
    #[test]
    fn every_timing_is_reported_even_at_zero() {
        let c = coverage(&[control(Timing::Feedback, None, &[])]);
        assert_eq!(
            c.by_timing,
            vec![
                (Timing::Feedforward, 0),
                (Timing::Concurrent, 0),
                (Timing::Feedback, 1),
            ]
        );
    }

    /// Unmeasured readings are excluded from the pass rate rather than
    /// counted as either half; all-unmeasured reports `None`, not 0%.
    #[test]
    fn unmeasured_readings_leave_the_pass_rate_alone() {
        let c = coverage(&[control(
            Timing::Feedback,
            None,
            &[
                (1, Verdict::Pass, true),
                (2, Verdict::Fail, true),
                (3, Verdict::Unmeasured, true),
            ],
        )]);
        assert_eq!(c.unmeasured, 1);
        assert_eq!(c.pass_rate_basis_points, Some(5_000)); // 1 of 2 measured

        let none = coverage(&[control(
            Timing::Feedback,
            None,
            &[(1, Verdict::Unmeasured, true)],
        )]);
        assert_eq!(none.pass_rate_basis_points, None);
    }

    /// A disabled control is counted, never reported as overdue — it is
    /// switched off, not neglected.
    #[test]
    fn disabled_controls_are_not_overdue() {
        let mut c = control(Timing::Concurrent, Some(1), &[(999, Verdict::Pass, true)]);
        c.enabled = false;
        let report = coverage(&[c]);
        assert_eq!(report.total, 1);
        assert_eq!(report.overdue, 0);
        assert_eq!(report.never_read, 0);
    }
}
