//! Lifecycle state machines (WPM-D3): one transition table per
//! pipeline, all funnelled through [`check`] so every controller gives
//! the same `422`-shaped answer ("cannot move X from `a` to `b`").
//!
//! Tables are `(from, to)` pairs — small, explicit, and exhaustively
//! unit-tested below. Terminal states simply have no outgoing pairs.

/// The legal employee status transitions (WPM-R7).
/// `onboarding → active` additionally requires the onboarding gate
/// ([`crate::rules::workforce`] has no say here — the controller checks
/// item completion; this table is the shape).
pub const EMPLOYEE: &[(&str, &str)] = &[
    ("onboarding", "active"),
    ("active", "on_leave"),
    ("on_leave", "active"),
    ("active", "offboarding"),
    ("on_leave", "offboarding"),
    ("offboarding", "terminated"),
    ("offboarding", "retired"),
    ("onboarding", "terminated"), // failed pre-start (e.g. right-to-work)
];

/// The legal requisition transitions (WPM-R1).
pub const REQUISITION: &[(&str, &str)] = &[
    ("draft", "open"),
    ("open", "interviewing"),
    ("interviewing", "offer"),
    ("offer", "filled"),
    ("offer", "interviewing"), // offer declined, back to the panel
    ("draft", "cancelled"),
    ("open", "cancelled"),
    ("interviewing", "cancelled"),
    ("offer", "cancelled"),
];

/// The legal application stage transitions (WPM-R2). `rejected` and
/// `withdrawn` are reachable from every non-terminal stage.
pub const APPLICATION: &[(&str, &str)] = &[
    ("received", "screened"),
    ("screened", "interviewing"),
    ("interviewing", "offer"),
    ("offer", "hired"),
    ("received", "rejected"),
    ("screened", "rejected"),
    ("interviewing", "rejected"),
    ("offer", "rejected"),
    ("received", "withdrawn"),
    ("screened", "withdrawn"),
    ("interviewing", "withdrawn"),
    ("offer", "withdrawn"),
];

/// The legal leave request transitions (WPM-R5).
pub const LEAVE: &[(&str, &str)] = &[
    ("requested", "approved"),
    ("requested", "rejected"),
    ("requested", "cancelled"),
    ("approved", "cancelled"), // pre-start cancellation restores balance
];

/// The legal review transitions (WPM-R10).
pub const REVIEW: &[(&str, &str)] = &[
    ("draft", "submitted"),
    ("submitted", "calibrated"),
    ("submitted", "draft"), // returned to the author by calibration
    ("calibrated", "shared"),
];

/// The legal payroll run transitions (WPM-R13). `calculated → draft`
/// is the re-open for re-calculation; `approved` is immutable except
/// the `paid` stamp.
pub const PAYROLL: &[(&str, &str)] = &[
    ("draft", "calculated"),
    ("calculated", "draft"),
    ("calculated", "approved"),
    ("approved", "paid"),
];

/// Whether `table` permits moving `from → to`.
#[must_use]
pub fn permits(table: &[(&str, &str)], from: &str, to: &str) -> bool {
    table.iter().any(|(f, t)| *f == from && *t == to)
}

/// Check a transition, yielding the family `422` message on refusal:
/// the message names the record kind and the **current** state.
///
/// # Errors
///
/// A human-readable refusal when the transition is not in the table.
pub fn check(kind: &str, table: &[(&str, &str)], from: &str, to: &str) -> Result<(), String> {
    if permits(table, from, to) {
        Ok(())
    } else {
        Err(format!("cannot move {kind} from {from:?} to {to:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tokens;

    /// Every `(from, to)` pair in every table uses tokens from the
    /// matching vocabulary — a typo in a table is a test failure.
    #[test]
    fn tables_use_vocabulary_tokens_only() {
        type Case = (
            &'static [(&'static str, &'static str)],
            &'static [&'static str],
        );
        let cases: &[Case] = &[
            (EMPLOYEE, tokens::EMPLOYEE_STATUSES),
            (REQUISITION, tokens::REQUISITION_STATUSES),
            (APPLICATION, tokens::APPLICATION_STAGES),
            (LEAVE, tokens::LEAVE_STATUSES),
            (REVIEW, tokens::REVIEW_STATUSES),
            (PAYROLL, tokens::PAYROLL_STATUSES),
        ];
        for (table, vocab) in cases {
            for (from, to) in *table {
                assert!(tokens::is_token(vocab, from), "unknown token {from}");
                assert!(tokens::is_token(vocab, to), "unknown token {to}");
            }
        }
    }

    /// The employee lifecycle: the happy path is legal, terminal states
    /// are terminal, and skips are refused.
    #[test]
    fn employee_matrix() {
        for (from, to) in [
            ("onboarding", "active"),
            ("active", "on_leave"),
            ("on_leave", "active"),
            ("active", "offboarding"),
            ("offboarding", "terminated"),
            ("offboarding", "retired"),
        ] {
            assert!(permits(EMPLOYEE, from, to), "{from}->{to} must be legal");
        }
        for (from, to) in [
            ("onboarding", "on_leave"),
            ("active", "terminated"), // must pass through offboarding
            ("terminated", "active"),
            ("retired", "active"),
            ("active", "active"),
        ] {
            assert!(!permits(EMPLOYEE, from, to), "{from}->{to} must be illegal");
        }
    }

    /// Requisitions: cancel from any non-terminal state; `filled` and
    /// `cancelled` are terminal; a declined offer returns to
    /// interviewing.
    #[test]
    fn requisition_matrix() {
        assert!(permits(REQUISITION, "draft", "open"));
        assert!(permits(REQUISITION, "offer", "interviewing"));
        assert!(permits(REQUISITION, "offer", "filled"));
        assert!(!permits(REQUISITION, "filled", "open"));
        assert!(!permits(REQUISITION, "cancelled", "open"));
        assert!(!permits(REQUISITION, "draft", "filled"));
    }

    /// Applications: reject/withdraw from every live stage; `hired`,
    /// `rejected`, `withdrawn` are terminal; no stage skipping.
    #[test]
    fn application_matrix() {
        for from in ["received", "screened", "interviewing", "offer"] {
            assert!(permits(APPLICATION, from, "rejected"));
            assert!(permits(APPLICATION, from, "withdrawn"));
        }
        assert!(permits(APPLICATION, "offer", "hired"));
        assert!(!permits(APPLICATION, "received", "offer"));
        assert!(!permits(APPLICATION, "hired", "rejected"));
        assert!(!permits(APPLICATION, "rejected", "screened"));
    }

    /// Leave: decisions only from `requested`; an approved request can
    /// still be cancelled; decisions are terminal otherwise.
    #[test]
    fn leave_matrix() {
        assert!(permits(LEAVE, "requested", "approved"));
        assert!(permits(LEAVE, "requested", "rejected"));
        assert!(permits(LEAVE, "approved", "cancelled"));
        assert!(!permits(LEAVE, "rejected", "approved"));
        assert!(!permits(LEAVE, "cancelled", "requested"));
        assert!(!permits(LEAVE, "approved", "rejected"));
    }

    /// Reviews: draft → submitted → calibrated → shared, with the
    /// calibration return-to-draft; `shared` is terminal.
    #[test]
    fn review_matrix() {
        assert!(permits(REVIEW, "draft", "submitted"));
        assert!(permits(REVIEW, "submitted", "draft"));
        assert!(permits(REVIEW, "calibrated", "shared"));
        assert!(!permits(REVIEW, "draft", "shared"));
        assert!(!permits(REVIEW, "shared", "draft"));
    }

    /// Payroll: draft ⇄ calculated, approve only from calculated, pay
    /// only from approved; `paid` is terminal; approved never returns
    /// to draft (immutability, WPM-R13).
    #[test]
    fn payroll_matrix() {
        assert!(permits(PAYROLL, "draft", "calculated"));
        assert!(permits(PAYROLL, "calculated", "draft"));
        assert!(permits(PAYROLL, "calculated", "approved"));
        assert!(permits(PAYROLL, "approved", "paid"));
        assert!(!permits(PAYROLL, "draft", "approved"));
        assert!(!permits(PAYROLL, "approved", "draft"));
        assert!(!permits(PAYROLL, "approved", "calculated"));
        assert!(!permits(PAYROLL, "paid", "draft"));
    }

    /// The `check` wrapper names the kind and the current state.
    #[test]
    fn check_message_names_kind_and_state() {
        let err = check("payroll run", PAYROLL, "approved", "draft").unwrap_err();
        assert!(err.contains("payroll run"));
        assert!(err.contains("approved"));
        assert!(check("payroll run", PAYROLL, "draft", "calculated").is_ok());
    }
}
