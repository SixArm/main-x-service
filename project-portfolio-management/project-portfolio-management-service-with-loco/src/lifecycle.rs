//! Pure rules for **bird's-eye visibility** — the challenge lifecycle
//! funnel (idea → proposal → delivery → benefits) and the per-plan
//! readiness check for the next phase gate. DB-free and exhaustively
//! unit-tested.
//!
//! Two honesty rules:
//!
//! - **Readiness is a checklist, not a verdict.** Every check reports
//!   its own outcome and detail; `ready` is simply "no check failed".
//!   A plan is never declared ready because the blockers were not
//!   looked up — an unknown is a check that is *not* ok.
//! - **Stalled is a measurement, not a mood.** A phase is stalled when
//!   items have sat in it longer than the caller's threshold, counted
//!   from real timestamps.

use crate::governance::{GATES, next_gate};

/// The lifecycle phases, in order. These are the funnel stages a
/// challenge passes through, not the plan's `kind` label.
pub const PHASES: &[&str] = &[
    "idea",
    "proposal",
    "in_delivery",
    "gated_complete",
    "benefits",
    "closed",
];

/// One phase of the funnel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PhaseRow {
    /// The phase name, from [`PHASES`].
    pub phase: &'static str,
    /// Items live in the phase right now.
    pub live: usize,
    /// Of those, how many have sat there beyond the stall threshold.
    pub stalled: usize,
}

/// The funnel's inputs: one entry per live item, with how many days it
/// has been in its current phase (`None` when unknown — such an item
/// counts as live but never as stalled, because we cannot show our
/// working).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseItem {
    /// The phase the item is in.
    pub phase: String,
    /// Days in that phase.
    pub days_in_phase: Option<i64>,
}

/// Roll items up into the ordered funnel. Every phase appears, even at
/// zero, so a bird's-eye view never hides an empty stage. Items in an
/// unknown phase are ignored by the rollup and returned as the second
/// element, so they are visible rather than silently absorbed.
#[must_use]
pub fn funnel(items: &[PhaseItem], stall_days: i64) -> (Vec<PhaseRow>, usize) {
    let mut rows: Vec<PhaseRow> = PHASES
        .iter()
        .map(|phase| PhaseRow {
            phase,
            live: 0,
            stalled: 0,
        })
        .collect();
    let mut unknown = 0;
    for item in items {
        let Some(row) = rows.iter_mut().find(|r| r.phase == item.phase) else {
            unknown += 1;
            continue;
        };
        row.live += 1;
        if item.days_in_phase.is_some_and(|days| days > stall_days) {
            row.stalled += 1;
        }
    }
    (rows, unknown)
}

/// One readiness check.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Check {
    /// Machine name of the check.
    pub name: &'static str,
    /// Whether it passes.
    pub ok: bool,
    /// What was actually counted.
    pub detail: String,
}

/// The evidence the readiness check consumes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadinessFacts {
    /// The plan's current stage (last approved gate), if any.
    pub stage: Option<String>,
    /// Open risks at or above the escalation exposure.
    pub severe_open_risks: usize,
    /// Review invitations still owed a verdict.
    pub outstanding_reviews: usize,
    /// Scheduled actions past their deadline and still pending.
    pub overdue_actions: usize,
    /// Tasks sitting in `blocked`.
    pub blocked_tasks: usize,
    /// Tasks not yet `done`.
    pub open_tasks: usize,
}

/// A plan's position in the lifecycle and its readiness to move on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Readiness {
    /// The last gate passed, or `null` before the first.
    pub stage: Option<String>,
    /// The gate this plan is working toward, or `null` when the gated
    /// journey is complete.
    pub next_gate: Option<&'static str>,
    /// Whether every check passes.
    pub ready: bool,
    /// The checks, in fixed order.
    pub checks: Vec<Check>,
    /// The failing checks' names — the blockers, at a glance.
    pub blockers: Vec<&'static str>,
}

/// Derive a plan's next-phase readiness. The checks are fixed and
/// always all reported, so an empty `blockers` list means every check
/// was actually run and passed.
#[must_use]
pub fn readiness(facts: &ReadinessFacts) -> Readiness {
    let stage = facts.stage.clone();
    let next = next_gate(stage.as_deref());
    let checks = vec![
        Check {
            name: "gate_journey_open",
            ok: next.is_some(),
            detail: match next {
                Some(gate) => format!("next gate is {gate}"),
                None => "gated journey already complete".to_string(),
            },
        },
        Check {
            name: "no_severe_open_risks",
            ok: facts.severe_open_risks == 0,
            detail: format!("{} severe open risk(s)", facts.severe_open_risks),
        },
        Check {
            name: "reviews_answered",
            ok: facts.outstanding_reviews == 0,
            detail: format!("{} review(s) outstanding", facts.outstanding_reviews),
        },
        Check {
            name: "no_overdue_actions",
            ok: facts.overdue_actions == 0,
            detail: format!("{} scheduled action(s) overdue", facts.overdue_actions),
        },
        Check {
            name: "nothing_blocked",
            ok: facts.blocked_tasks == 0,
            detail: format!(
                "{} blocked of {} open task(s)",
                facts.blocked_tasks, facts.open_tasks
            ),
        },
    ];
    let blockers: Vec<&'static str> = checks.iter().filter(|c| !c.ok).map(|c| c.name).collect();
    Readiness {
        stage,
        next_gate: next,
        ready: blockers.is_empty(),
        checks,
        blockers,
    }
}

/// The phase a plan sits in, from the facts the funnel has: a stage of
/// the final gate means the gated journey is done. Unknown stages fall
/// back to `in_delivery` — the plan exists and is being worked, which
/// is the honest minimum claim.
#[must_use]
pub fn phase_of_plan(stage: Option<&str>, active: bool) -> &'static str {
    if !active {
        return "closed";
    }
    match stage {
        // An unknown stage is not evidence of progress: it must not be
        // read as a completed gate journey.
        Some(stage) if !GATES.contains(&stage) => "in_delivery",
        Some(stage) if stage == GATES[GATES.len() - 1] => "benefits",
        Some(stage) if next_gate(Some(stage)).is_none() => "gated_complete",
        _ => "in_delivery",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(phase: &str, days: Option<i64>) -> PhaseItem {
        PhaseItem {
            phase: phase.to_string(),
            days_in_phase: days,
        }
    }

    #[test]
    fn every_phase_is_reported_even_when_empty() {
        let (rows, unknown) = funnel(&[], 30);
        assert_eq!(rows.len(), PHASES.len());
        assert!(rows.iter().all(|r| r.live == 0 && r.stalled == 0));
        assert_eq!(unknown, 0);
    }

    #[test]
    fn phases_stay_in_lifecycle_order() {
        let (rows, _) = funnel(&[], 30);
        let names: Vec<&str> = rows.iter().map(|r| r.phase).collect();
        assert_eq!(names, PHASES);
    }

    #[test]
    fn stalled_counts_only_items_past_the_threshold() {
        let (rows, _) = funnel(
            &[
                item("idea", Some(31)),
                item("idea", Some(30)),
                item("idea", None),
                item("proposal", Some(90)),
            ],
            30,
        );
        let ideas = rows.iter().find(|r| r.phase == "idea").expect("idea row");
        assert_eq!(ideas.live, 3);
        assert_eq!(
            ideas.stalled, 1,
            "30 days is not yet past a 30-day threshold"
        );
        let proposals = rows
            .iter()
            .find(|r| r.phase == "proposal")
            .expect("proposal row");
        assert_eq!(proposals.stalled, 1);
    }

    #[test]
    fn an_item_of_unknown_age_is_live_but_never_stalled() {
        let (rows, _) = funnel(&[item("idea", None)], 1);
        let ideas = rows.iter().find(|r| r.phase == "idea").expect("idea row");
        assert_eq!((ideas.live, ideas.stalled), (1, 0));
    }

    #[test]
    fn items_in_an_unknown_phase_are_surfaced_not_absorbed() {
        let (rows, unknown) = funnel(&[item("limbo", Some(5)), item("idea", Some(1))], 30);
        assert_eq!(unknown, 1);
        assert_eq!(rows.iter().map(|r| r.live).sum::<usize>(), 1);
    }

    #[test]
    fn a_clean_plan_is_ready_and_names_its_next_gate() {
        let r = readiness(&ReadinessFacts::default());
        assert!(r.ready);
        assert!(r.blockers.is_empty());
        assert_eq!(r.next_gate, Some(GATES[0]), "no stage ⇒ the first gate");
        assert_eq!(r.checks.len(), 5, "every check is always reported");
    }

    #[test]
    fn each_blocker_is_named_and_fails_readiness() {
        let facts = ReadinessFacts {
            severe_open_risks: 2,
            outstanding_reviews: 1,
            overdue_actions: 3,
            blocked_tasks: 4,
            open_tasks: 9,
            ..ReadinessFacts::default()
        };
        let r = readiness(&facts);
        assert!(!r.ready);
        assert_eq!(
            r.blockers,
            vec![
                "no_severe_open_risks",
                "reviews_answered",
                "no_overdue_actions",
                "nothing_blocked"
            ]
        );
        assert!(
            r.checks
                .iter()
                .any(|c| c.name == "nothing_blocked" && c.detail.contains("4 blocked of 9")),
            "checks show their working: {:?}",
            r.checks
        );
    }

    #[test]
    fn a_completed_gate_journey_is_not_ready_for_a_further_gate() {
        let facts = ReadinessFacts {
            stage: Some(GATES[GATES.len() - 1].to_string()),
            ..ReadinessFacts::default()
        };
        let r = readiness(&facts);
        assert_eq!(r.next_gate, None);
        assert!(!r.ready);
        assert_eq!(r.blockers, vec!["gate_journey_open"]);
    }

    #[test]
    fn readiness_reports_the_gate_after_the_current_stage() {
        let facts = ReadinessFacts {
            stage: Some(GATES[0].to_string()),
            ..ReadinessFacts::default()
        };
        assert_eq!(readiness(&facts).next_gate, Some(GATES[1]));
    }

    #[test]
    fn plan_phases_follow_stage_and_liveness() {
        assert_eq!(phase_of_plan(None, true), "in_delivery");
        assert_eq!(phase_of_plan(Some(GATES[2]), true), "in_delivery");
        assert_eq!(
            phase_of_plan(Some(GATES[GATES.len() - 1]), true),
            "benefits"
        );
        assert_eq!(phase_of_plan(Some(GATES[0]), false), "closed");
        assert_eq!(
            phase_of_plan(Some("not-a-gate"), true),
            "in_delivery",
            "an unknown stage claims the minimum, not the maximum"
        );
    }
}
