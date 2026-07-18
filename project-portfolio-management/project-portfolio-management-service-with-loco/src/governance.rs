//! Pure governance rules (PPM Phase A): the proposal pipeline state
//! machine, the ordered phase gates, risk scoring, and budget
//! arithmetic — DB-free, exhaustively unit-tested (the patient-flow
//! `flow/` posture applied here).

/// Proposal pipeline statuses (PPM-1), in lifecycle order.
pub const PROPOSAL_STATUSES: &[&str] =
    &["draft", "submitted", "in_review", "approved", "rejected", "promoted"];

/// Phase gates (PPM-3), strictly ordered: an approved review at gate
/// *n+1* is only legal when the item's stage is gate *n*.
pub const GATES: &[&str] = &[
    "g0_concept",
    "g1_feasibility",
    "g2_definition",
    "g3_delivery",
    "g4_launch",
    "g5_benefits",
];

/// Gate-review decisions. Only the approving decisions advance stage.
pub const DECISIONS: &[&str] = &["approved", "approved_with_conditions", "hold", "rejected"];

/// Risk statuses (PPM-12).
pub const RISK_STATUSES: &[&str] = &["open", "mitigating", "closed", "materialised"];

/// Budget-line categories (PPM-10).
pub const BUDGET_CATEGORIES: &[&str] = &["capex", "opex"];

/// Whether `value` is in the closed set `set`.
#[must_use]
pub fn is_token(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

/// The proposal pipeline actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalAction {
    /// draft → submitted.
    Submit,
    /// submitted → `in_review`.
    Review,
    /// `in_review` → approved.
    Approve,
    /// `in_review` → rejected.
    Reject,
    /// approved → promoted (mints the work item).
    Promote,
}

impl ProposalAction {
    /// The action's audit token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Submit => "proposal_submitted",
            Self::Review => "proposal_in_review",
            Self::Approve => "proposal_approved",
            Self::Reject => "proposal_rejected",
            Self::Promote => "proposal_promoted",
        }
    }
}

/// Apply a pipeline action to the current status. First-match, total:
/// every legal `(status, action)` pair yields the next status; every
/// other pair is an error naming both.
///
/// # Errors
///
/// A human-readable refusal when the action is not legal from
/// `status`.
pub fn proposal_transition(status: &str, action: ProposalAction) -> Result<&'static str, String> {
    match (status, action) {
        ("draft", ProposalAction::Submit) => Ok("submitted"),
        ("submitted", ProposalAction::Review) => Ok("in_review"),
        ("in_review", ProposalAction::Approve) => Ok("approved"),
        ("in_review", ProposalAction::Reject) => Ok("rejected"),
        ("approved", ProposalAction::Promote) => Ok("promoted"),
        _ => Err(format!("cannot {action:?} a proposal in status {status:?}")),
    }
}

/// Whether a proposal may still be edited (only before submission).
#[must_use]
pub fn proposal_editable(status: &str) -> bool {
    status == "draft"
}

/// The next gate an item at `stage` may be reviewed at: the first
/// gate when no stage is set, else the gate after the current one
/// (`None` once `g5_benefits` is passed — the journey is complete).
#[must_use]
pub fn next_gate(stage: Option<&str>) -> Option<&'static str> {
    match stage {
        None => Some(GATES[0]),
        Some(current) => {
            let idx = GATES.iter().position(|g| *g == current)?;
            GATES.get(idx + 1).copied()
        }
    }
}

/// Apply a gate review: the gate must be the item's [`next_gate`]
/// (gates cannot be skipped or repeated), and only an approving
/// decision advances the stage — `hold` / `rejected` record the
/// review and leave the stage unchanged.
///
/// Returns the item's new stage.
///
/// # Errors
///
/// A refusal naming the expected gate when `gate` is out of order,
/// unknown, or the journey is already complete.
pub fn apply_gate_review(
    stage: Option<&str>,
    gate: &str,
    decision: &str,
) -> Result<Option<String>, String> {
    if !is_token(GATES, gate) {
        return Err(format!("unknown gate {gate:?}"));
    }
    if !is_token(DECISIONS, decision) {
        return Err(format!("unknown decision {decision:?}"));
    }
    let expected = next_gate(stage)
        .ok_or_else(|| "the gate journey is complete (g5_benefits passed)".to_string())?;
    if gate != expected {
        return Err(format!(
            "out-of-order gate: expected {expected:?}, got {gate:?}"
        ));
    }
    if matches!(decision, "approved" | "approved_with_conditions") {
        Ok(Some(gate.to_string()))
    } else {
        Ok(stage.map(ToString::to_string))
    }
}

/// Risk exposure = probability × impact (each 1–5 ⇒ 1–25).
///
/// # Errors
///
/// When either input is outside 1–5.
pub fn risk_exposure(probability: i32, impact: i32) -> Result<i32, String> {
    if !(1..=5).contains(&probability) {
        return Err(format!("probability must be 1–5, got {probability}"));
    }
    if !(1..=5).contains(&impact) {
        return Err(format!("impact must be 1–5, got {impact}"));
    }
    Ok(probability * impact)
}

/// Whether an ISO-4217-shaped currency code (three ASCII uppercase
/// letters) — format-checked, not a registry lookup.
#[must_use]
pub fn valid_currency(code: &str) -> bool {
    code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase())
}

/// Accumulate a recorded actual onto a budget line, refusing `i64`
/// overflow (never-panic invariant — attacker-controlled input).
///
/// # Errors
///
/// When the addition would overflow.
pub fn accumulate_actual(current_minor: i64, delta_minor: i64) -> Result<i64, String> {
    current_minor
        .checked_add(delta_minor)
        .ok_or_else(|| "actual amount overflows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The happy path walks the whole pipeline; every off-path pair
    /// is refused (exhaustive over statuses × actions).
    #[test]
    fn proposal_pipeline_is_total() {
        let legal = [
            ("draft", ProposalAction::Submit, "submitted"),
            ("submitted", ProposalAction::Review, "in_review"),
            ("in_review", ProposalAction::Approve, "approved"),
            ("in_review", ProposalAction::Reject, "rejected"),
            ("approved", ProposalAction::Promote, "promoted"),
        ];
        for (from, action, to) in legal {
            assert_eq!(proposal_transition(from, action), Ok(to));
        }
        let actions = [
            ProposalAction::Submit,
            ProposalAction::Review,
            ProposalAction::Approve,
            ProposalAction::Reject,
            ProposalAction::Promote,
        ];
        for status in PROPOSAL_STATUSES {
            for action in actions {
                let is_legal = legal.iter().any(|(f, a, _)| f == status && *a == action);
                assert_eq!(
                    proposal_transition(status, action).is_ok(),
                    is_legal,
                    "{status} × {action:?}"
                );
            }
        }
        assert!(proposal_editable("draft"));
        assert!(!proposal_editable("submitted"));
        assert!(!proposal_editable("promoted"));
    }

    /// Gates advance strictly in order; approving decisions move the
    /// stage, hold/rejected do not; skipping and repeating refuse;
    /// the journey ends after g5.
    #[test]
    fn gate_journey_is_strictly_ordered() {
        assert_eq!(next_gate(None), Some("g0_concept"));
        assert_eq!(next_gate(Some("g0_concept")), Some("g1_feasibility"));
        assert_eq!(next_gate(Some("g5_benefits")), None);

        // Walk the whole journey with approvals.
        let mut stage: Option<String> = None;
        for gate in GATES {
            stage = apply_gate_review(stage.as_deref(), gate, "approved").unwrap();
            assert_eq!(stage.as_deref(), Some(*gate));
        }
        assert!(
            apply_gate_review(stage.as_deref(), "g5_benefits", "approved").is_err(),
            "journey complete"
        );

        // Hold records but does not advance; the same gate stays next.
        let held = apply_gate_review(None, "g0_concept", "hold").unwrap();
        assert_eq!(held, None);
        // Skipping refuses and names the expected gate.
        let err = apply_gate_review(None, "g2_definition", "approved").unwrap_err();
        assert!(err.contains("g0_concept"), "{err}");
        // Repeating a passed gate refuses.
        assert!(apply_gate_review(Some("g1_feasibility"), "g1_feasibility", "approved").is_err());
        // Unknown tokens refuse.
        assert!(apply_gate_review(None, "g9_wat", "approved").is_err());
        assert!(apply_gate_review(None, "g0_concept", "maybe").is_err());
    }

    /// Exposure is p×i within bounds; out-of-range refuses.
    #[test]
    fn risk_exposure_bounds() {
        assert_eq!(risk_exposure(1, 1), Ok(1));
        assert_eq!(risk_exposure(5, 5), Ok(25));
        assert_eq!(risk_exposure(3, 4), Ok(12));
        assert!(risk_exposure(0, 3).is_err());
        assert!(risk_exposure(3, 6).is_err());
        assert!(risk_exposure(-1, 1).is_err());
    }

    /// Currency shape + overflow-safe actuals.
    #[test]
    fn money_rules() {
        assert!(valid_currency("GBP"));
        assert!(valid_currency("EUR"));
        assert!(!valid_currency("gbp"));
        assert!(!valid_currency("GBPX"));
        assert!(!valid_currency("G1P"));
        assert_eq!(accumulate_actual(100, 50), Ok(150));
        assert_eq!(accumulate_actual(100, -150), Ok(-50));
        assert!(accumulate_actual(i64::MAX, 1).is_err());
    }
}
