//! Keyed integrity for `audit_log` rows.
//!
//! ## What this detects, and what it does not
//!
//! **Detects:** an audit row whose content was altered — the action verb
//! changed, the actor rewritten, the edge endpoints edited, the request
//! provenance or timestamp moved.
//!
//! **Does not detect:** a row **deleted wholesale**, or rows reordered.
//! Nothing in a row can attest to its own continued existence. Catching
//! deletion needs a hash chain plus external-witness checkpoints, which
//! this service does not have.
//!
//! ## Why only the audit trail is MACed here
//!
//! The `edges` table is a **derived read-model**, rebuilt by consuming
//! each entity service's event stream. A MAC there would attest to a
//! projection rather than to a source of truth: if it disagreed, the
//! correct response is to rebuild, not to investigate tampering. The
//! authoritative edges live in each originating service's `entity_links`,
//! and that is where protecting them is meaningful.
//!
//! The audit trail is different — it records who asked this service for
//! what, and it is not rebuildable from anywhere.

use super::mac::{self, Domain};

/// Field separator: ASCII unit separator.
const SEP: char = '\u{1f}';

/// Pre-image format version, bound in first.
pub const AUDIT_MAC_VERSION: &str = "lg-a1";

/// The fields an audit row's MAC covers.
#[derive(Debug, Clone)]
pub struct AuditInput<'a> {
    /// The acting principal.
    pub actor: Option<&'a str>,
    /// The action verb.
    pub action: &'a str,
    /// The edge kind, where the action concerns an edge.
    pub edge_kind: Option<&'a str>,
    /// The edge's source reference.
    pub from_ref: Option<&'a str>,
    /// The edge's target reference.
    pub to_ref: Option<&'a str>,
    /// Request provenance: source address.
    pub user_ip: Option<&'a str>,
    /// Request provenance: user agent.
    pub user_agent: Option<&'a str>,
    /// When it happened, epoch microseconds.
    pub occurred_at_micros: i64,
}

/// Build the MAC pre-image.
///
/// The edge endpoints are bound in because they are the substance of what
/// was recorded: an audit row saying "linked A to B" is worthless if B can
/// be rewritten afterwards.
#[must_use]
pub fn preimage(input: &AuditInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(AUDIT_MAC_VERSION);
    field(input.actor.unwrap_or(""));
    field(input.action);
    field(input.edge_kind.unwrap_or(""));
    field(input.from_ref.unwrap_or(""));
    field(input.to_ref.unwrap_or(""));
    field(input.user_ip.unwrap_or(""));
    field(input.user_agent.unwrap_or(""));
    field(&input.occurred_at_micros.to_string());
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

/// The MAC for an audit row, or `None` when no key is configured.
#[must_use]
pub fn tag(input: &AuditInput<'_>) -> Option<String> {
    mac::tag(Domain::AuditRow, &preimage(input))
}

/// Borrow a stored row as an [`AuditInput`].
#[must_use]
pub fn input_for(row: &crate::models::_entities::audit_log::Model) -> AuditInput<'_> {
    AuditInput {
        actor: row.actor.as_deref(),
        action: row.action.as_str(),
        edge_kind: row.edge_kind.as_deref(),
        from_ref: row.from_ref.as_deref(),
        to_ref: row.to_ref.as_deref(),
        user_ip: row.user_ip.as_deref(),
        user_agent: row.user_agent.as_deref(),
        occurred_at_micros: row.occurred_at.timestamp_micros(),
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
    pub mismatched: Vec<String>,
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
pub fn verify(rows: &[crate::models::_entities::audit_log::Model]) -> AuditIntegrityReport {
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
            report.mismatched.push(row.id.to_string());
        }
    }
    report.verified = report.mismatched.is_empty();
    report
}

#[cfg(test)]
mod tests {
    use super::{AUDIT_MAC_VERSION, AuditInput, preimage};

    fn input() -> AuditInput<'static> {
        AuditInput {
            actor: Some("alice"),
            action: "linked",
            edge_kind: Some("same_identity"),
            from_ref: Some("person:1"),
            to_ref: Some("worker:2"),
            user_ip: Some("10.0.0.1"),
            user_agent: Some("curl"),
            occurred_at_micros: 1_700_000_000_000_000,
        }
    }

    /// Every field is bound in. The endpoints especially: an audit row
    /// saying "linked A to B" is worthless if B can be rewritten after
    /// the fact.
    #[test]
    fn every_field_is_bound_into_the_preimage() {
        let base = preimage(&input());
        let mutate = |f: &dyn Fn(&mut AuditInput<'static>)| {
            let mut i = input();
            f(&mut i);
            preimage(&i)
        };
        assert_ne!(mutate(&|i| i.actor = Some("mallory")), base, "actor");
        assert_ne!(mutate(&|i| i.actor = None), base, "actor cleared");
        assert_ne!(mutate(&|i| i.action = "unlinked"), base, "action");
        assert_ne!(mutate(&|i| i.edge_kind = Some("works_at")), base, "kind");
        assert_ne!(mutate(&|i| i.from_ref = Some("person:9")), base, "from");
        assert_ne!(mutate(&|i| i.to_ref = Some("worker:9")), base, "to");
        assert_ne!(mutate(&|i| i.user_ip = None), base, "ip");
        assert_ne!(mutate(&|i| i.user_agent = None), base, "agent");
        assert_ne!(mutate(&|i| i.occurred_at_micros = 1), base, "timestamp");
    }

    /// The endpoints cannot be swapped without changing the pre-image:
    /// "A linked to B" and "B linked to A" are different assertions.
    #[test]
    fn the_endpoints_are_not_interchangeable() {
        let mut swapped = input();
        swapped.from_ref = Some("worker:2");
        swapped.to_ref = Some("person:1");
        assert_ne!(preimage(&swapped), preimage(&input()));
    }

    /// The separator makes field boundaries unambiguous.
    #[test]
    fn field_boundaries_are_unambiguous() {
        let mut a = input();
        a.action = "ab";
        a.edge_kind = Some("c");
        let mut b = input();
        b.action = "a";
        b.edge_kind = Some("bc");
        assert_ne!(preimage(&a), preimage(&b));
    }

    /// The version tag leads the pre-image.
    #[test]
    fn the_version_tag_leads_the_preimage() {
        assert!(preimage(&input()).starts_with(AUDIT_MAC_VERSION.as_bytes()));
    }
}
