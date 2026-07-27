//! Keyed integrity for `auth_events` rows.
//!
//! ## Why this trail in particular
//!
//! `auth_events` is the record of **who logged in and who was granted
//! which authorization attributes**. The `attributes_assigned` event is
//! the ABAC grant trail: it is the evidence that someone gave an account
//! `access=admin`. An attacker who escalated privilege and could then
//! edit that row would erase the only account of how they got it.
//!
//! ## What this detects, and what it does not
//!
//! **Detects:** a row whose content was altered — the event verb changed,
//! the subject rewritten, the detail edited, the timestamp moved.
//!
//! **Does not detect:** a row **deleted wholesale**, or rows reordered.
//! Nothing in a row can attest to its own continued existence. Catching
//! deletion needs a hash chain plus external-witness checkpoints, which
//! this service does not have.
//!
//! ## The email field
//!
//! The address is bound in, and it is personal data that GDPR erasure
//! scrubs (SEC-A7). Erasure therefore invalidates the MAC on the scrubbed
//! rows by design — the row genuinely no longer says what it said. Those
//! rows report as mismatched, which is the correct reading: an erasure is
//! a deliberate, recorded modification, not a silent one.

use uuid::Uuid;

use super::mac::{self, Domain};

/// Field separator: ASCII unit separator.
const SEP: char = '\u{1f}';

/// Pre-image format version, bound in first.
pub const AUDIT_MAC_VERSION: &str = "au-a1";

/// The fields an auth-event row's MAC covers.
#[derive(Debug, Clone)]
pub struct AuditInput<'a> {
    /// The event verb.
    pub event: &'a str,
    /// The subject address, where one applies.
    pub email: Option<&'a str>,
    /// The subject user, where one applies.
    pub user_pid: Option<Uuid>,
    /// Free-text detail.
    pub detail: Option<&'a str>,
    /// When it happened, epoch microseconds.
    pub created_at_micros: i64,
}

/// Build the MAC pre-image.
#[must_use]
pub fn preimage(input: &AuditInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(AUDIT_MAC_VERSION);
    field(input.event);
    field(input.email.unwrap_or(""));
    field(&input.user_pid.map_or_else(String::new, |p| p.to_string()));
    field(input.detail.unwrap_or(""));
    field(&input.created_at_micros.to_string());
    buf
}

/// The MAC for an auth-event row, or `None` when no key is configured.
#[must_use]
pub fn tag(input: &AuditInput<'_>) -> Option<String> {
    mac::tag(Domain::AuditRow, &preimage(input))
}

#[cfg(test)]
mod tests {
    use super::{AUDIT_MAC_VERSION, AuditInput, preimage};
    use uuid::Uuid;

    fn input() -> AuditInput<'static> {
        AuditInput {
            event: "login",
            email: Some("alice@example.com"),
            user_pid: Some(Uuid::from_u128(1)),
            detail: None,
            created_at_micros: 1_700_000_000_000_000,
        }
    }

    /// Every field is bound in. The event verb and subject especially:
    /// rewriting `attributes_assigned` into `login`, or repointing it at
    /// another account, is exactly how a privilege escalation would be
    /// covered up.
    #[test]
    fn every_field_is_bound_into_the_preimage() {
        let base = preimage(&input());
        let mutate = |f: &dyn Fn(&mut AuditInput<'static>)| {
            let mut i = input();
            f(&mut i);
            preimage(&i)
        };
        assert_ne!(mutate(&|i| i.event = "logout"), base, "event");
        assert_ne!(mutate(&|i| i.email = Some("m@x.com")), base, "email");
        assert_ne!(mutate(&|i| i.email = None), base, "email cleared");
        assert_ne!(
            mutate(&|i| i.user_pid = Some(Uuid::from_u128(2))),
            base,
            "pid"
        );
        assert_ne!(mutate(&|i| i.user_pid = None), base, "pid cleared");
        assert_ne!(mutate(&|i| i.detail = Some("x")), base, "detail");
        assert_ne!(mutate(&|i| i.created_at_micros = 1), base, "timestamp");
    }

    /// The separator makes field boundaries unambiguous.
    #[test]
    fn field_boundaries_are_unambiguous() {
        let mut a = input();
        a.event = "ab";
        a.email = Some("c");
        let mut b = input();
        b.event = "a";
        b.email = Some("bc");
        assert_ne!(preimage(&a), preimage(&b));
    }

    /// The version tag leads the pre-image.
    #[test]
    fn the_version_tag_leads_the_preimage() {
        assert!(preimage(&input()).starts_with(AUDIT_MAC_VERSION.as_bytes()));
    }
}
