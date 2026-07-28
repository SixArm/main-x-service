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

/// Every digest for one audit row, computed from one pre-image.
///
/// A **named struct rather than a tuple**: with three values `.0`/`.1`/
/// `.2` is a latent bug, since putting the SHA-3 digest in the SHA-256
/// column type-checks and fails only at the next verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    /// FIPS 180-4 SHA-256. Written unconditionally.
    pub sha256: String,
    /// FIPS 202 SHA3-256. Written unconditionally.
    pub sha3: String,
    /// HMAC-SHA256, or `None` when no key is configured.
    pub mac: Option<String>,
}

/// All three digests from one call, so none can be stamped without the
/// others.
///
/// The two unkeyed digests are written **unconditionally**; only the MAC
/// depends on a key. That matters because the MAC is default-off: without
/// these, an audit row on a deployment that has not yet configured a key
/// would carry no integrity at all.
#[must_use]
pub fn digests(input: &AuditInput<'_>) -> Digests {
    use sha2::Digest as _;

    let bytes = preimage(input);
    Digests {
        sha256: to_hex(&sha2::Sha256::digest(&bytes)),
        // Fully qualified: `sha2::Digest` and `sha3::Digest` are distinct
        // traits with the same method name, so importing both is
        // ambiguous.
        sha3: to_hex(&<sha3::Sha3_256 as sha3::Digest>::digest(&bytes)),
        mac: mac::tag(Domain::AuditRow, &bytes),
    }
}

/// Lowercase hex.
fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// The MAC for an auth-event row, or `None` when no key is configured.
#[must_use]
pub fn tag(input: &AuditInput<'_>) -> Option<String> {
    mac::tag(Domain::AuditRow, &preimage(input))
}

/// Borrow a stored row as an [`AuditInput`].
#[must_use]
pub fn input_for(row: &crate::models::_entities::auth_events::Model) -> AuditInput<'_> {
    AuditInput {
        event: row.event.as_str(),
        email: row.email.as_deref(),
        user_pid: row.user_pid,
        detail: row.detail.as_deref(),
        created_at_micros: row.created_at.timestamp_micros(),
    }
}

/// The outcome of verifying a run of audit rows.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct AuditIntegrityReport {
    /// Rows examined.
    pub rows: usize,
    /// Rows whose SHA-256 was recomputed and matched.
    pub intact: usize,
    /// Rows carrying no SHA-256 digest — written before the column
    /// existed. Neither verified nor a break.
    pub unhashed: usize,
    /// Rows whose SHA-3 was recomputed and matched.
    pub sha3_intact: usize,
    /// Rows carrying no SHA-3 digest.
    pub sha3_unhashed: usize,
    /// Rows whose MAC was recomputed and matched.
    pub mac_valid: usize,
    /// Rows carrying no MAC — written before a key was configured.
    pub mac_absent: usize,
    /// Rows naming a key or scheme this service cannot check.
    pub mac_unverifiable: usize,
    /// Rows whose content did not match what was stored.
    pub mismatched: Vec<i32>,
    /// `true` when no mismatch was found.
    pub verified: bool,
    /// What this result does and does not attest to, carried in the
    /// response so a reader cannot mistake it for full tamper-evidence.
    pub caveat: &'static str,
}

/// The caveat every report carries.
const CAVEAT: &str = "A verified result attests that no examined row's content was altered \
     without the key. It does NOT attest that no row was deleted: nothing in a row can \
     prove its own continued existence. Detecting deletion requires a hash chain and \
     external-witness checkpoints, which this service does not have.";

/// Verify a run of audit rows by recomputing every stored digest.
///
/// **Every stored value is read**, not just the MAC. The unkeyed digests
/// are checked first because they are present even when no key is
/// configured — on a default deployment they are the only integrity these
/// rows have, and checking only the MAC would report such a deployment as
/// entirely unverified when it is not.
#[must_use]
pub fn verify(rows: &[crate::models::_entities::auth_events::Model]) -> AuditIntegrityReport {
    let mut report = AuditIntegrityReport {
        rows: rows.len(),
        intact: 0,
        unhashed: 0,
        sha3_intact: 0,
        sha3_unhashed: 0,
        mac_valid: 0,
        mac_absent: 0,
        mac_unverifiable: 0,
        mismatched: Vec::new(),
        verified: true,
        caveat: CAVEAT,
    };
    for row in rows {
        let input = input_for(row);
        let computed = digests(&input);
        let mut broken = false;

        match row.hash.as_deref() {
            None => report.unhashed += 1,
            Some(stored) => {
                if stored == computed.sha256 {
                    report.intact += 1;
                } else {
                    broken = true;
                }
            }
        }
        match row.hash_sha3.as_deref() {
            None => report.sha3_unhashed += 1,
            Some(stored) => {
                if stored == computed.sha3 {
                    report.sha3_intact += 1;
                } else {
                    broken = true;
                }
            }
        }
        match mac::verify(Domain::AuditRow, row.mac.as_deref(), &preimage(&input)) {
            mac::MacVerdict::Valid => report.mac_valid += 1,
            mac::MacVerdict::Absent => report.mac_absent += 1,
            mac::MacVerdict::UnknownKey(_)
            | mac::MacVerdict::UnknownScheme(_)
            | mac::MacVerdict::Malformed => report.mac_unverifiable += 1,
            mac::MacVerdict::Invalid => broken = true,
        }
        // One entry per row, not one per algorithm: a tampered row is a
        // single incident.
        if broken {
            report.mismatched.push(row.id);
        }
    }
    report.verified = report.mismatched.is_empty();
    report
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
