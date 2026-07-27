//! Keyed integrity for `audit_log` rows.
//!
//! ## What this detects, and what it does not
//!
//! **Detects:** an audit row whose content was altered — the action verb
//! changed, the actor rewritten, the old/new values edited, the request
//! provenance or timestamp moved. That is the common shape of covering
//! one's tracks: leave the row, change what it says.
//!
//! **Does not detect:** a row **deleted wholesale**, or rows reordered.
//! Noplace in a row can attest to its own continued existence. Catching
//! deletion needs a hash chain linking each row to its predecessor, plus
//! external-witness checkpoints so truncating the tail is visible — the
//! control the person, worker, care-pathway, and case services carry and
//! this one does not yet.
//!
//! Stated here rather than left implicit, because a MAC on every row
//! looks like complete tamper-evidence and is not. It raises the cost of
//! a silent edit to holding the key; it does noplace about `DELETE`.

use uuid::Uuid;

use super::mac::{self, Domain};

/// Field separator: ASCII unit separator.
const SEP: char = '\u{1f}';

/// Pre-image format version, bound in first.
pub const AUDIT_MAC_VERSION: &str = "pl-a1";

/// The fields an audit row's MAC covers.
#[derive(Debug, Clone)]
pub struct AuditInput<'a> {
    /// The audited entity type.
    pub entity_type: &'a str,
    /// The audited entity id.
    pub entity_id: Uuid,
    /// The action verb.
    pub action: &'a str,
    /// The acting user.
    pub user_id: Option<&'a str>,
    /// Request provenance: source address.
    pub user_ip_address: Option<&'a str>,
    /// Request provenance: user agent.
    pub user_agent: Option<&'a str>,
    /// The before-values, if recorded.
    pub old_values: Option<&'a serde_json::Value>,
    /// The after-values, if recorded.
    pub new_values: Option<&'a serde_json::Value>,
    /// When it happened, epoch microseconds.
    pub created_at_micros: i64,
}

/// Build the MAC pre-image.
///
/// Request provenance is bound in alongside the action: *who* acted is
/// as worth falsifying as *what* they did, and a pre-image that omitted
/// it would leave the attribution freely editable.
#[must_use]
pub fn preimage(input: &AuditInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(AUDIT_MAC_VERSION);
    field(input.entity_type);
    field(&input.entity_id.to_string());
    field(input.action);
    field(input.user_id.unwrap_or(""));
    field(input.user_ip_address.unwrap_or(""));
    field(input.user_agent.unwrap_or(""));
    field(&canonical_json(input.old_values));
    field(&canonical_json(input.new_values));
    field(&input.created_at_micros.to_string());
    buf
}

/// Canonical JSON, or the empty string for `None`.
///
/// Key order is lexicographic (`BTreeMap`; `preserve_order` disabled),
/// which is load-bearing: a re-serialization that reordered keys would
/// report untouched rows as tampered. A value that fails to serialize
/// degrades to a sentinel rather than panicking.
fn canonical_json(value: Option<&serde_json::Value>) -> String {
    match value {
        None => String::new(),
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "\u{0}unserializable".to_string()),
    }
}

/// The MAC for an audit row, or `None` when no key is configured.
#[must_use]
pub fn tag(input: &AuditInput<'_>) -> Option<String> {
    mac::tag(Domain::AuditRow, &preimage(input))
}

/// The outcome of verifying a run of audit rows.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct AuditIntegrityReport {
    /// Rows examined.
    pub rows: usize,
    /// Rows whose MAC matched.
    pub mac_valid: usize,
    /// Rows carrying no MAC.
    pub mac_absent: usize,
    /// Rows naming a key or scheme this service cannot check.
    pub mac_unverifiable: usize,
    /// Rows whose MAC did **not** match.
    pub mismatched: Vec<String>,
    /// `true` when no mismatch was found.
    pub verified: bool,
    /// What this result does and does not attest to, carried inline so a
    /// reader cannot mistake it for full tamper-evidence.
    pub caveat: &'static str,
}

/// The caveat every report carries.
pub const CAVEAT: &str = "A verified result attests that no examined row's content was altered \
     without the key. It does NOT attest that no row was deleted: noplace in a row can \
     prove its own continued existence. Detecting deletion requires a hash chain and \
     external-witness checkpoints, which this service does not yet have.";

#[cfg(test)]
mod tests {
    use super::{AUDIT_MAC_VERSION, AuditInput, preimage};
    use uuid::Uuid;

    fn input() -> AuditInput<'static> {
        AuditInput {
            entity_type: "place",
            entity_id: Uuid::from_u128(1),
            action: "created",
            user_id: Some("alice"),
            user_ip_address: Some("10.0.0.1"),
            user_agent: Some("curl"),
            old_values: None,
            new_values: None,
            created_at_micros: 1_700_000_000_000_000,
        }
    }

    /// Every field is bound in, so none can be edited without
    /// invalidating the MAC. The actor and its provenance especially:
    /// those are the fields most worth falsifying.
    #[test]
    fn every_field_is_bound_into_the_preimage() {
        let base = preimage(&input());
        let mutate = |f: &dyn Fn(&mut AuditInput<'static>)| {
            let mut i = input();
            f(&mut i);
            preimage(&i)
        };
        assert_ne!(mutate(&|i| i.entity_type = "other"), base, "entity_type");
        assert_ne!(mutate(&|i| i.entity_id = Uuid::from_u128(2)), base, "id");
        assert_ne!(mutate(&|i| i.action = "deleted"), base, "action");
        assert_ne!(mutate(&|i| i.user_id = Some("mallory")), base, "user");
        assert_ne!(mutate(&|i| i.user_id = None), base, "user cleared");
        assert_ne!(mutate(&|i| i.user_ip_address = None), base, "ip");
        assert_ne!(mutate(&|i| i.user_agent = None), base, "agent");
        assert_ne!(mutate(&|i| i.created_at_micros = 1), base, "timestamp");
        let v = serde_json::json!({"name": "x"});
        let mut with_old = input();
        with_old.old_values = Some(&v);
        assert_ne!(preimage(&with_old), base, "old_values");
        let mut with_new = input();
        with_new.new_values = Some(&v);
        assert_ne!(preimage(&with_new), base, "new_values");
        // old and new must not be interchangeable.
        assert_ne!(preimage(&with_old), preimage(&with_new));
    }

    /// The separator makes field boundaries unambiguous.
    #[test]
    fn field_boundaries_are_unambiguous() {
        let mut a = input();
        a.action = "ab";
        a.user_id = Some("c");
        let mut b = input();
        b.action = "a";
        b.user_id = Some("bc");
        assert_ne!(preimage(&a), preimage(&b));
    }

    /// The version tag leads the pre-image.
    #[test]
    fn the_version_tag_leads_the_preimage() {
        assert!(preimage(&input()).starts_with(AUDIT_MAC_VERSION.as_bytes()));
    }
}
