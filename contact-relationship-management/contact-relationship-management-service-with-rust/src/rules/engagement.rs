//! Pure rules for the stakeholder-engagement features: declared
//! stakeholder typing, recorded sentiment, and the
//! innovation-partnership lifecycle. Declared means declared — no
//! rule here infers a role, a grid position, or a sentiment.

/// Stakeholder roles a contact or account may be **declared** as.
pub const STAKEHOLDER_ROLES: &[&str] = &[
    "customer", "partner", "regulator", "sponsor", "community", "media", "member",
];

/// Recorded interaction sentiments.
pub const SENTIMENTS: &[&str] = &["positive", "neutral", "negative"];

/// Innovation-partnership kinds.
pub const PARTNERSHIP_KINDS: &[&str] =
    &["university", "startup", "vendor", "accelerator", "other"];

/// Partnership stages, in order.
pub const PARTNERSHIP_STAGES: &[&str] = &["scouting", "pilot", "scaled", "retired"];

/// Membership statuses.
pub const MEMBERSHIP_STATUSES: &[&str] = &["active", "lapsed"];

/// A power–interest score must sit on the declared 1–5 scale.
#[must_use]
pub fn valid_grid_score(score: i32) -> bool {
    (1..=5).contains(&score)
}

/// The partnership lifecycle: forward one step at a time
/// (scouting → pilot → scaled), and `retired` reachable from any
/// live stage. Everything else refuses with a reason.
///
/// # Errors
///
/// A human-readable refusal naming the legal moves.
pub fn partnership_transition(current: &str, to: &str) -> Result<(), String> {
    if !PARTNERSHIP_STAGES.contains(&to) {
        return Err(format!("unknown stage `{to}` (stages: {PARTNERSHIP_STAGES:?})"));
    }
    if current == "retired" {
        return Err("a retired partnership does not transition".to_string());
    }
    if to == "retired" {
        return Ok(());
    }
    let current_idx = PARTNERSHIP_STAGES.iter().position(|s| *s == current);
    let to_idx = PARTNERSHIP_STAGES.iter().position(|s| *s == to);
    match (current_idx, to_idx) {
        (Some(c), Some(t)) if t == c + 1 => Ok(()),
        _ => Err(format!(
            "illegal transition `{current}` → `{to}` (forward one step, or retire)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_scores_are_one_to_five() {
        assert!(valid_grid_score(1));
        assert!(valid_grid_score(5));
        assert!(!valid_grid_score(0));
        assert!(!valid_grid_score(6));
    }

    #[test]
    fn partnership_lifecycle_is_forward_or_retire() {
        assert!(partnership_transition("scouting", "pilot").is_ok());
        assert!(partnership_transition("pilot", "scaled").is_ok());
        assert!(partnership_transition("scouting", "retired").is_ok());
        assert!(partnership_transition("scouting", "scaled").is_err(), "no skipping");
        assert!(partnership_transition("pilot", "scouting").is_err(), "no going back");
        assert!(partnership_transition("retired", "pilot").is_err(), "retired is terminal");
        assert!(partnership_transition("pilot", "sideways").is_err(), "unknown refused");
    }
}
