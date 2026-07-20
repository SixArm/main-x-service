//! Pure rules for the care-pathway instance layer — the enrolment
//! status lifecycle, urgency + care-team vocabularies, and the review
//! hygiene helper. No I/O.

/// Instance statuses.
pub const INSTANCE_STATUSES: &[&str] = &["active", "on_hold", "completed", "discontinued"];

/// Urgency levels (routine → emergency).
pub const URGENCY_LEVELS: &[&str] = &["routine", "urgent", "emergency"];

/// Care-team roles.
pub const TEAM_ROLES: &[&str] =
    &["lead_clinician", "gp", "specialist", "nurse", "mental_health", "coordinator", "other"];

/// Instance event kinds (recorded, not inferred).
pub const EVENT_KINDS: &[&str] = &["note", "review", "escalation", "de_escalation", "referral"];

/// Whether a status is terminal (no further transitions).
#[must_use]
pub fn is_terminal(status: &str) -> bool {
    matches!(status, "completed" | "discontinued")
}

/// The enrolment lifecycle: `active` ↔ `on_hold`, and either may go to
/// a terminal `completed` / `discontinued`. Terminal states do not
/// transition.
///
/// # Errors
///
/// A human-readable refusal.
pub fn instance_transition(current: &str, to: &str) -> Result<(), String> {
    if !INSTANCE_STATUSES.contains(&to) {
        return Err(format!("unknown status `{to}` (statuses: {INSTANCE_STATUSES:?})"));
    }
    if is_terminal(current) {
        return Err(format!("`{current}` is terminal and does not transition"));
    }
    let ok = matches!(
        (current, to),
        ("active", "on_hold" | "completed" | "discontinued")
            | ("on_hold", "active" | "completed" | "discontinued")
    );
    if ok {
        Ok(())
    } else {
        Err(format!("illegal transition `{current}` → `{to}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_holds_and_closes() {
        assert!(instance_transition("active", "on_hold").is_ok());
        assert!(instance_transition("on_hold", "active").is_ok());
        assert!(instance_transition("active", "completed").is_ok());
        assert!(instance_transition("on_hold", "discontinued").is_ok());
        assert!(instance_transition("active", "active").is_err(), "no self-loop");
        assert!(instance_transition("completed", "active").is_err(), "terminal");
        assert!(instance_transition("active", "sideways").is_err(), "unknown");
    }

    #[test]
    fn terminal_states() {
        assert!(is_terminal("completed") && is_terminal("discontinued"));
        assert!(!is_terminal("active") && !is_terminal("on_hold"));
    }
}
