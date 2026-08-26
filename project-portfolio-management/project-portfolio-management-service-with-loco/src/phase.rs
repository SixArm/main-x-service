//! Pure rules for the **sequential project phase** — Initiating →
//! Planning → Executing → Controlling → Closing. DB-free and
//! exhaustively unit-tested (entity spec §5.9.4 / FR-30).
//!
//! The phase says where *management of the plan* has got to. It is one
//! of three ordered vocabularies in this entity and they are
//! deliberately uncoupled (§1.5.1) — see [`crate::lifecycle`] for the
//! demand funnel and [`crate::governance`] for the gate stage.
//!
//! Four rules, each with a reason that is not obvious:
//!
//! - **Advancement is one step at a time.** A skip is refused. If a
//!   plan really is executing before it was planned, that is a fact
//!   worth recording as such, not one to hide by permitting a jump.
//! - **Backward moves are allowed and must carry a reason.**
//!   Re-planning is normal; only a *silent* backward move is refused.
//! - **Every phase is reported even at zero**, so an empty phase is a
//!   finding rather than a row that vanished.
//! - **Phase never gates an operational write.** Tasks may be created
//!   in Initiating and issues raised in Closing. Refusing writes on
//!   that basis would teach operators to misreport the phase, which
//!   costs more than it buys.

use project_portfolio_management_matcher::PlanPhase;
use serde::{Deserialize, Serialize};

/// Why a phase move is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// More than one step forward. Carries the phase that was skipped.
    SkippedPhase {
        /// The first phase that would have been jumped over.
        skipped: String,
    },
    /// Backward without a stated reason.
    SilentRegression,
    /// The move goes nowhere.
    NoChange,
}

/// Whether a move from `from` to `to` is permitted.
///
/// `from` is `None` for a plan that has never had a phase, in which
/// case only the first phase may be entered — the same
/// one-step-at-a-time rule, counted from before the beginning.
///
/// # Errors
/// Returns the [`Refusal`] with enough detail for a `422` that names
/// what was wrong rather than merely rejecting.
pub fn check_move(
    from: Option<PlanPhase>,
    to: PlanPhase,
    reason: Option<&str>,
) -> Result<(), Refusal> {
    let Some(from) = from else {
        // No phase yet: entering anything past the first is a skip.
        return if to == PlanPhase::Initiating {
            Ok(())
        } else {
            Err(Refusal::SkippedPhase {
                skipped: PlanPhase::Initiating.token().to_string(),
            })
        };
    };

    let (a, b) = (from.ordinal(), to.ordinal());
    if a == b {
        return Err(Refusal::NoChange);
    }
    if b > a {
        if b - a > 1 {
            // Name the first phase jumped over, not merely "too far".
            let skipped = PlanPhase::ALL[a + 1];
            return Err(Refusal::SkippedPhase {
                skipped: skipped.token().to_string(),
            });
        }
        return Ok(());
    }
    // Backward: permitted, but never silently. A plan returning from
    // Executing to Planning is re-planning, which is normal and worth
    // recording; an unexplained return is not.
    if reason.is_some_and(|r| !r.trim().is_empty()) {
        Ok(())
    } else {
        Err(Refusal::SilentRegression)
    }
}

/// The phase a plan may advance to next, or `None` at the end.
#[must_use]
pub fn next_phase(from: Option<PlanPhase>) -> Option<PlanPhase> {
    match from {
        None => Some(PlanPhase::Initiating),
        Some(current) => PlanPhase::ALL.get(current.ordinal() + 1).copied(),
    }
}

/// One recorded transition, reduced to what the duration rollup needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionFact {
    /// The phase entered.
    pub to: PlanPhase,
    /// Milliseconds since the epoch when it was entered.
    pub at_ms: i64,
}

/// Time spent in one phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseDuration {
    /// The phase.
    pub phase: String,
    /// Milliseconds spent in it across every visit.
    pub ms: i64,
    /// How many times it was entered. More than one means the plan came
    /// back — which the durations alone would hide.
    pub visits: usize,
    /// Whether the plan is in this phase now.
    pub current: bool,
}

/// Total time in each phase, **every phase present even at zero**.
///
/// `transitions` need not be sorted; `as_of_ms` closes the final open
/// interval. A phase entered more than once accumulates across visits
/// and reports the visit count, because a plan that returned to
/// Planning twice is a different story from one that sat there once for
/// the same total.
#[must_use]
pub fn durations(transitions: &[TransitionFact], as_of_ms: i64) -> Vec<PhaseDuration> {
    let mut ordered: Vec<TransitionFact> = transitions.to_vec();
    ordered.sort_by_key(|t| t.at_ms);

    let mut rows: Vec<PhaseDuration> = PlanPhase::ALL
        .iter()
        .map(|phase| PhaseDuration {
            phase: phase.token().to_string(),
            ms: 0,
            visits: 0,
            current: false,
        })
        .collect();

    for (index, entry) in ordered.iter().enumerate() {
        // Clamp to zero: clock skew must not produce negative time.
        let end = ordered
            .get(index + 1)
            .map_or(as_of_ms, |next| next.at_ms)
            .max(entry.at_ms);
        let span = end.saturating_sub(entry.at_ms);
        let slot = entry.to.ordinal();
        rows[slot].ms = rows[slot].ms.saturating_add(span);
        rows[slot].visits += 1;
    }

    if let Some(last) = ordered.last() {
        rows[last.to.ordinal()].current = true;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One step forward is the only permitted advance.
    #[test]
    fn one_step_forward_only() {
        assert_eq!(
            check_move(Some(PlanPhase::Initiating), PlanPhase::Planning, None),
            Ok(())
        );
        assert_eq!(
            check_move(Some(PlanPhase::Initiating), PlanPhase::Executing, None),
            Err(Refusal::SkippedPhase {
                skipped: "planning".to_string()
            })
        );
    }

    /// A refusal names the phase that was skipped, so the `422` says
    /// what was wrong rather than only that something was.
    #[test]
    fn a_skip_names_what_was_skipped() {
        let err = check_move(Some(PlanPhase::Initiating), PlanPhase::Closing, None);
        assert_eq!(
            err,
            Err(Refusal::SkippedPhase {
                skipped: "planning".to_string()
            })
        );
    }

    /// A plan with no phase may only enter the first one.
    #[test]
    fn a_plan_with_no_phase_starts_at_the_beginning() {
        assert_eq!(check_move(None, PlanPhase::Initiating, None), Ok(()));
        assert_eq!(
            check_move(None, PlanPhase::Executing, None),
            Err(Refusal::SkippedPhase {
                skipped: "initiating".to_string()
            })
        );
    }

    /// Backward moves are allowed **with** a reason and refused without
    /// one. Re-planning is normal; hiding it is not.
    #[test]
    fn backward_needs_a_reason() {
        assert_eq!(
            check_move(Some(PlanPhase::Executing), PlanPhase::Planning, None),
            Err(Refusal::SilentRegression)
        );
        assert_eq!(
            check_move(Some(PlanPhase::Executing), PlanPhase::Planning, Some("  ")),
            Err(Refusal::SilentRegression)
        );
        assert_eq!(
            check_move(
                Some(PlanPhase::Executing),
                PlanPhase::Planning,
                Some("scope change")
            ),
            Ok(())
        );
        // A backward jump of several phases is still just a regression:
        // the one-step rule constrains advancement, not retreat.
        assert_eq!(
            check_move(
                Some(PlanPhase::Closing),
                PlanPhase::Initiating,
                Some("rechartered")
            ),
            Ok(())
        );
    }

    /// Moving nowhere is refused rather than written as a no-op
    /// transition, which would inflate the visit count.
    #[test]
    fn no_change_is_refused() {
        assert_eq!(
            check_move(Some(PlanPhase::Planning), PlanPhase::Planning, None),
            Err(Refusal::NoChange)
        );
    }

    /// The next phase, and the end of the road.
    #[test]
    fn next_phase_walks_then_stops() {
        assert_eq!(next_phase(None), Some(PlanPhase::Initiating));
        assert_eq!(
            next_phase(Some(PlanPhase::Initiating)),
            Some(PlanPhase::Planning)
        );
        assert_eq!(next_phase(Some(PlanPhase::Closing)), None);
    }

    fn t(phase: PlanPhase, at_ms: i64) -> TransitionFact {
        TransitionFact { to: phase, at_ms }
    }

    /// Durations partition the elapsed time, and **every phase appears
    /// even at zero** — an empty phase is a finding, not a missing row.
    #[test]
    fn durations_report_every_phase() {
        let rows = durations(
            &[t(PlanPhase::Initiating, 0), t(PlanPhase::Planning, 100)],
            300,
        );
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].ms, 100);
        assert_eq!(rows[1].ms, 200);
        assert_eq!(rows[2].ms, 0);
        assert!(rows[1].current);
        assert!(!rows[0].current);
        // The partition is exact: no time is lost or double-counted.
        assert_eq!(rows.iter().map(|r| r.ms).sum::<i64>(), 300);
    }

    /// A plan that came back reports the visit count, which the total
    /// alone would hide: two visits of 50 and one of 100 are different
    /// stories.
    #[test]
    fn revisits_are_counted_not_merged_away() {
        let rows = durations(
            &[
                t(PlanPhase::Planning, 0),
                t(PlanPhase::Executing, 50),
                t(PlanPhase::Planning, 100),
            ],
            150,
        );
        assert_eq!(rows[1].visits, 2);
        assert_eq!(rows[1].ms, 100);
        assert_eq!(rows[2].visits, 1);
    }

    /// Unsorted input and clock skew must not produce negative time or
    /// panic (security invariant 2).
    #[test]
    fn unsorted_and_skewed_input_is_total() {
        let rows = durations(
            &[t(PlanPhase::Planning, 100), t(PlanPhase::Initiating, 0)],
            50,
        );
        assert!(rows.iter().all(|r| r.ms >= 0));
        // as_of before the last transition: the open interval is zero,
        // never negative.
        let skewed = durations(&[t(PlanPhase::Executing, i64::MAX)], i64::MIN);
        assert!(skewed.iter().all(|r| r.ms >= 0));
        let empty = durations(&[], 0);
        assert_eq!(empty.len(), 5);
        assert!(empty.iter().all(|r| r.ms == 0 && !r.current));
    }
}
