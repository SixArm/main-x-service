//! Closed token vocabularies for the string-typed domain columns.
//!
//! Each `const` slice is the closed set the validators accept; the
//! database stores the token verbatim (HCM-D2 keeps the schema plain
//! strings so a vocabulary can grow by data migration, not DDL).

/// Employee statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const EMPLOYEE_STATUSES: &[&str] = &[
    "onboarding",
    "active",
    "on_leave",
    "offboarding",
    "terminated",
    "retired",
];

/// Employment types.
pub const EMPLOYMENT_TYPES: &[&str] = &["permanent", "fixed_term", "contractor", "intern"];

/// Requisition statuses.
pub const REQUISITION_STATUSES: &[&str] =
    &["draft", "open", "interviewing", "offer", "filled", "cancelled"];

/// Candidate sources.
pub const CANDIDATE_SOURCES: &[&str] = &["web", "referral", "event", "import", "agency", "other"];

/// Application stages.
pub const APPLICATION_STAGES: &[&str] = &[
    "received",
    "screened",
    "interviewing",
    "offer",
    "hired",
    "rejected",
    "withdrawn",
];

/// Interview outcomes.
pub const INTERVIEW_OUTCOMES: &[&str] = &["pending", "advance", "reject"];

/// Onboarding item statuses.
pub const ONBOARDING_STATUSES: &[&str] = &["pending", "complete", "waived"];

/// Time entry kinds. Overtime is **derived** (HCM-R4); the `overtime`
/// kind exists for explicitly-agreed extra time.
pub const TIME_KINDS: &[&str] = &["regular", "overtime", "on_call"];

/// Time entry statuses.
pub const TIME_STATUSES: &[&str] = &["recorded", "approved"];

/// Leave kinds. Annual leave enforces the balance; sick may go
/// negative but is flagged (HCM-R5).
pub const LEAVE_KINDS: &[&str] = &["annual", "sick", "parental", "unpaid", "other"];

/// Leave request statuses.
pub const LEAVE_STATUSES: &[&str] = &["requested", "approved", "rejected", "cancelled"];

/// Benefit plan kinds.
pub const BENEFIT_KINDS: &[&str] = &["health", "pension", "dental", "life", "wellness", "other"];

/// Review cycle statuses.
pub const CYCLE_STATUSES: &[&str] = &["open", "closed"];

/// Review statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const REVIEW_STATUSES: &[&str] = &["draft", "submitted", "calibrated", "shared"];

/// Goal statuses.
pub const GOAL_STATUSES: &[&str] = &["open", "met", "missed"];

/// Training enrollment statuses.
pub const TRAINING_STATUSES: &[&str] = &["enrolled", "in_progress", "completed", "failed"];

/// Succession readiness ratings.
pub const READINESS: &[&str] = &["ready_now", "ready_1y", "ready_2y"];

/// Payroll run statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const PAYROLL_STATUSES: &[&str] = &["draft", "calculated", "approved", "paid"];

/// Whether `value` is a member of the closed set `set`.
#[must_use]
pub fn is_token(set: &[&str], value: &str) -> bool {
    set.contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_is_exact_and_case_sensitive() {
        assert!(is_token(EMPLOYEE_STATUSES, "onboarding"));
        assert!(!is_token(EMPLOYEE_STATUSES, "Onboarding"));
        assert!(!is_token(EMPLOYEE_STATUSES, "hired"));
        assert!(is_token(LEAVE_KINDS, "sick"));
        assert!(!is_token(LEAVE_KINDS, ""));
    }
}
