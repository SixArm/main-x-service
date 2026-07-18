//! Closed token vocabularies for the string-typed domain columns
//! (CRM-D2 keeps the schema plain strings so a vocabulary can grow by
//! data migration, not DDL).

/// Contact statuses.
pub const CONTACT_STATUSES: &[&str] = &["active", "inactive"];

/// Preferred contact channels.
pub const CHANNELS: &[&str] = &["email", "phone", "post", "none"];

/// Marketing-consent states (CRM-R6). `never` = no consent event yet.
pub const CONSENT_STATES: &[&str] = &["granted", "withdrawn", "never"];

/// Consent event actions.
pub const CONSENT_ACTIONS: &[&str] = &["granted", "withdrawn"];

/// Account relationship tiers.
pub const ACCOUNT_TIERS: &[&str] = &["prospect", "customer", "partner", "former"];

/// Activity kinds.
pub const ACTIVITY_KINDS: &[&str] = &["call", "email", "meeting", "note", "task"];

/// Objects an activity can attach to.
pub const ACTIVITY_SUBJECTS: &[&str] = &["contact", "account", "lead", "deal", "ticket"];

/// Lead sources.
pub const LEAD_SOURCES: &[&str] = &["web", "referral", "event", "import", "campaign"];

/// Lead statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const LEAD_STATUSES: &[&str] =
    &["new", "contacted", "qualified", "converted", "disqualified"];

/// Campaign statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const CAMPAIGN_STATUSES: &[&str] =
    &["draft", "scheduled", "running", "completed", "cancelled"];

/// Nurture enrolment statuses.
pub const ENROLLMENT_STATUSES: &[&str] = &["active", "completed", "exited"];

/// Ticket statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const TICKET_STATUSES: &[&str] = &["open", "pending", "resolved", "closed"];

/// Ticket priorities (each needs an SLA policy row).
pub const PRIORITIES: &[&str] = &["low", "normal", "high", "urgent"];

/// Ticket channels.
pub const TICKET_CHANNELS: &[&str] = &["email", "phone", "web", "chat"];

/// Article statuses (lifecycle in [`crate::rules::lifecycle`]).
pub const ARTICLE_STATUSES: &[&str] = &["draft", "published", "archived"];

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
        assert!(is_token(LEAD_STATUSES, "qualified"));
        assert!(!is_token(LEAD_STATUSES, "Qualified"));
        assert!(!is_token(LEAD_STATUSES, "won"));
        assert!(is_token(PRIORITIES, "urgent"));
        assert!(!is_token(PRIORITIES, ""));
    }
}
