//! Pure rules for **collaborative review** and **assignee
//! management** — the review invitation state machine, the consensus
//! aggregation over submitted verdicts, and the per-assignee workload
//! rollup. DB-free and exhaustively unit-tested (the `governance.rs`
//! posture).
//!
//! Two honesty rules shape this module:
//!
//! - **Consensus never invents agreement.** A tie is reported as a tie
//!   (`majority: None`), and the mean score is `None` until at least
//!   one reviewer has actually submitted a score.
//! - **Outstanding work stays visible.** `outstanding` counts the
//!   invitations still owed a verdict, so a "complete" review is one
//!   where nobody is still being waited on.

use std::collections::BTreeMap;

/// What a review invitation may be attached to.
pub const REVIEW_SUBJECT_KINDS: &[&str] = &["idea", "proposal", "plan"];

/// Review invitation statuses. `invited` / `accepted` are live; the
/// rest are terminal.
pub const REVIEW_STATUSES: &[&str] = &[
    "invited",
    "accepted",
    "declined",
    "submitted",
    "expired",
    "withdrawn",
];

/// Whether the reviewer is inside the organisation or an outside
/// expert. External reviewers are a disclosure decision, so the scope
/// is stored explicitly rather than guessed from the reference.
pub const REVIEWER_SCOPES: &[&str] = &["internal", "external"];

/// The verdict a submitted review may carry.
pub const RECOMMENDATIONS: &[&str] = &["advance", "hold", "reject"];

/// The review statuses that still owe a verdict.
pub const LIVE_REVIEW_STATUSES: &[&str] = &["invited", "accepted"];

/// Whether `value` is a member of the closed set `set`.
#[must_use]
pub fn is_token(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

/// Whether a review score is in range. Scores are `0..=100` so they
/// compose with the Smart Score's expert-review component
/// ([`crate::prioritisation`]) without rescaling.
#[must_use]
pub const fn valid_review_score(score: i32) -> bool {
    score >= 0 && score <= 100
}

/// The legal review transitions:
///
/// ```text
/// invited  ──accept──▶ accepted ──submit──▶ submitted (terminal)
///    │                    │
///    ├──decline──▶ declined (terminal)
///    └──expire/withdraw──▶ expired / withdrawn (terminal)
/// ```
///
/// A verdict may only be submitted by a reviewer who accepted, so an
/// unanswered invitation can never become evidence.
///
/// # Errors
///
/// A refusal naming the current status when the move is illegal.
pub fn review_transition(from: &str, to: &str) -> Result<(), String> {
    if !is_token(REVIEW_STATUSES, from) {
        return Err(format!("unknown review status `{from}`"));
    }
    if !is_token(REVIEW_STATUSES, to) {
        return Err(format!("unknown review status `{to}`"));
    }
    let allowed: &[&str] = match from {
        "invited" => &["accepted", "declined", "expired", "withdrawn"],
        "accepted" => &["submitted", "expired", "withdrawn"],
        _ => &[],
    };
    if allowed.contains(&to) {
        Ok(())
    } else if allowed.is_empty() {
        Err(format!("a `{from}` review is final and cannot change"))
    } else {
        Err(format!(
            "a `{from}` review may only move to one of {allowed:?}, not `{to}`"
        ))
    }
}

/// One submitted verdict, as the consensus sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// The reviewer's score `0..=100`, when they gave one.
    pub score: Option<i32>,
    /// One of [`RECOMMENDATIONS`].
    pub recommendation: String,
    /// One of [`REVIEWER_SCOPES`].
    pub scope: String,
}

/// The aggregate view of one subject's review round.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Consensus {
    /// Invitations issued and not withdrawn.
    pub invited: usize,
    /// Verdicts actually submitted.
    pub submitted: usize,
    /// Invitations still owed a verdict (`invited` + `accepted`).
    pub outstanding: usize,
    /// Reviewers who declined.
    pub declined: usize,
    /// Mean submitted score, or `None` when nobody scored.
    pub mean_score: Option<f64>,
    /// Submitted verdicts by recommendation.
    pub recommendations: BTreeMap<String, usize>,
    /// The strict-majority recommendation, or `None` on a tie / no
    /// verdicts. A plurality is **not** promoted to a majority.
    pub majority: Option<String>,
    /// How many verdicts came from outside experts.
    pub external_submitted: usize,
    /// Whether every invitation has been answered.
    pub complete: bool,
}

/// Aggregate a review round. `outstanding` and `declined` come from
/// the live/declined invitation counts the caller already has, so an
/// unanswered invitation is never silently dropped from the picture.
#[must_use]
pub fn consensus(verdicts: &[Verdict], outstanding: usize, declined: usize) -> Consensus {
    let submitted = verdicts.len();
    let scores: Vec<i32> = verdicts.iter().filter_map(|v| v.score).collect();
    let mean_score = if scores.is_empty() {
        None
    } else {
        let total: i64 = scores.iter().map(|s| i64::from(*s)).sum();
        // Precision loss here is cosmetic (a mean of 0..=100 over a
        // bounded reviewer count), and the value is rounded to 1dp.
        #[allow(clippy::cast_precision_loss)]
        Some(((total as f64 / scores.len() as f64) * 10.0).round() / 10.0)
    };
    let mut recommendations: BTreeMap<String, usize> = BTreeMap::new();
    for v in verdicts {
        *recommendations.entry(v.recommendation.clone()).or_insert(0) += 1;
    }
    let majority = recommendations
        .iter()
        .find(|(_, count)| **count * 2 > submitted)
        .map(|(rec, _)| rec.clone());
    Consensus {
        invited: submitted + outstanding + declined,
        submitted,
        outstanding,
        declined,
        mean_score,
        recommendations,
        majority,
        external_submitted: verdicts.iter().filter(|v| v.scope == "external").count(),
        complete: outstanding == 0,
    }
}

/// One assignee's open workload, as the board sees it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssigneeLoad {
    /// The `EntityRef` the tasks are assigned to.
    pub assignee_ref: String,
    /// Open (not `done`) tasks.
    pub open: usize,
    /// Open tasks sitting in `blocked`.
    pub blocked: usize,
    /// Open tasks in `in_progress` — the work-in-progress count.
    pub in_progress: usize,
    /// Story points across the open tasks that carry an estimate.
    pub open_points: i64,
    /// Open tasks with no estimate (excluded from `open_points`).
    pub unestimated: usize,
}

/// One task, as the workload rollup sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLoadFact {
    /// The assignee, or `None` for unassigned work.
    pub assignee_ref: Option<String>,
    /// The Kanban status.
    pub status: String,
    /// Story points, when estimated.
    pub points: Option<i32>,
}

/// Roll open tasks up per assignee, busiest first (ties broken by
/// reference for a stable order). Unassigned work is reported under
/// the reserved `unassigned` key rather than dropped — an unassigned
/// backlog is exactly what "manage assignees" needs to surface.
#[must_use]
pub fn workload(tasks: &[TaskLoadFact]) -> Vec<AssigneeLoad> {
    let mut by_ref: BTreeMap<String, AssigneeLoad> = BTreeMap::new();
    for task in tasks.iter().filter(|t| t.status != "done") {
        let key = task
            .assignee_ref
            .clone()
            .unwrap_or_else(|| "unassigned".to_string());
        let load = by_ref.entry(key.clone()).or_insert_with(|| AssigneeLoad {
            assignee_ref: key,
            open: 0,
            blocked: 0,
            in_progress: 0,
            open_points: 0,
            unestimated: 0,
        });
        load.open += 1;
        match task.status.as_str() {
            "blocked" => load.blocked += 1,
            "in_progress" => load.in_progress += 1,
            _ => {}
        }
        match task.points {
            Some(points) => load.open_points += i64::from(points),
            None => load.unestimated += 1,
        }
    }
    let mut rows: Vec<AssigneeLoad> = by_ref.into_values().collect();
    rows.sort_by(|a, b| {
        b.open
            .cmp(&a.open)
            .then_with(|| a.assignee_ref.cmp(&b.assignee_ref))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict(score: Option<i32>, advice: &str, reviewer_scope: &str) -> Verdict {
        Verdict {
            score,
            recommendation: advice.to_string(),
            scope: reviewer_scope.to_string(),
        }
    }

    #[test]
    fn review_transitions_follow_the_state_machine() {
        assert!(review_transition("invited", "accepted").is_ok());
        assert!(review_transition("invited", "declined").is_ok());
        assert!(review_transition("accepted", "submitted").is_ok());
        assert!(review_transition("invited", "expired").is_ok());
    }

    #[test]
    fn an_unaccepted_invitation_cannot_submit_a_verdict() {
        let err = review_transition("invited", "submitted").expect_err("must refuse");
        assert!(err.contains("invited"), "{err}");
    }

    #[test]
    fn terminal_review_statuses_are_final() {
        for terminal in ["submitted", "declined", "expired", "withdrawn"] {
            let err = review_transition(terminal, "accepted").expect_err("must refuse");
            assert!(err.contains("final"), "{terminal}: {err}");
        }
    }

    #[test]
    fn unknown_review_statuses_are_refused_both_ways() {
        assert!(review_transition("banana", "accepted").is_err());
        assert!(review_transition("invited", "banana").is_err());
    }

    #[test]
    fn review_scores_are_bounded_to_0_100() {
        assert!(valid_review_score(0));
        assert!(valid_review_score(100));
        assert!(!valid_review_score(-1));
        assert!(!valid_review_score(101));
    }

    #[test]
    fn consensus_means_only_the_scores_that_exist() {
        let c = consensus(
            &[
                verdict(Some(80), "advance", "internal"),
                verdict(None, "advance", "external"),
                verdict(Some(60), "hold", "internal"),
            ],
            0,
            0,
        );
        assert_eq!(c.mean_score, Some(70.0), "unscored verdicts do not dilute");
        assert_eq!(c.submitted, 3);
        assert_eq!(c.external_submitted, 1);
    }

    #[test]
    fn consensus_reports_no_mean_when_nobody_scored() {
        let c = consensus(&[verdict(None, "hold", "internal")], 0, 0);
        assert_eq!(c.mean_score, None);
    }

    #[test]
    fn a_strict_majority_is_required_a_plurality_is_not_enough() {
        // 2 advance / 1 hold / 1 reject — a plurality, not a majority.
        let plurality = consensus(
            &[
                verdict(None, "advance", "internal"),
                verdict(None, "advance", "internal"),
                verdict(None, "hold", "internal"),
                verdict(None, "reject", "external"),
            ],
            0,
            0,
        );
        assert_eq!(plurality.majority, None, "2 of 4 is not a majority");

        let majority = consensus(
            &[
                verdict(None, "advance", "internal"),
                verdict(None, "advance", "internal"),
                verdict(None, "reject", "external"),
            ],
            0,
            0,
        );
        assert_eq!(majority.majority.as_deref(), Some("advance"));
    }

    #[test]
    fn a_tie_has_no_majority() {
        let c = consensus(
            &[
                verdict(None, "advance", "internal"),
                verdict(None, "reject", "external"),
            ],
            0,
            0,
        );
        assert_eq!(c.majority, None);
    }

    #[test]
    fn outstanding_invitations_keep_the_round_incomplete() {
        let c = consensus(&[verdict(Some(90), "advance", "internal")], 2, 1);
        assert_eq!(c.invited, 4, "1 submitted + 2 outstanding + 1 declined");
        assert_eq!(c.outstanding, 2);
        assert!(!c.complete);
        assert!(consensus(&[], 0, 0).complete, "an empty round is complete");
    }

    fn task(assignee: Option<&str>, status: &str, points: Option<i32>) -> TaskLoadFact {
        TaskLoadFact {
            assignee_ref: assignee.map(std::string::ToString::to_string),
            status: status.to_string(),
            points,
        }
    }

    #[test]
    fn workload_counts_open_work_per_assignee_busiest_first() {
        let rows = workload(&[
            task(Some("person:a"), "todo", Some(3)),
            task(Some("person:a"), "in_progress", Some(5)),
            task(Some("person:a"), "done", Some(8)),
            task(Some("person:b"), "blocked", None),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].assignee_ref, "person:a");
        assert_eq!(rows[0].open, 2, "done work is not open work");
        assert_eq!(rows[0].open_points, 8);
        assert_eq!(rows[0].in_progress, 1);
        assert_eq!(rows[1].blocked, 1);
        assert_eq!(rows[1].unestimated, 1);
        assert_eq!(rows[1].open_points, 0, "unestimated adds no points");
    }

    #[test]
    fn unassigned_work_is_surfaced_not_dropped() {
        let rows = workload(&[task(None, "todo", None), task(None, "todo", Some(2))]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].assignee_ref, "unassigned");
        assert_eq!(rows[0].open, 2);
    }

    #[test]
    fn workload_of_nothing_is_empty_not_a_zero_row() {
        assert!(workload(&[]).is_empty());
        assert!(workload(&[task(Some("person:a"), "done", Some(1))]).is_empty());
    }
}
