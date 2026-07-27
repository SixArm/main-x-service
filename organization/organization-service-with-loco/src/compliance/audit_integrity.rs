//! Keyed integrity for `audit_logs` rows.
//!
//! Each audit row carries an HMAC over a pre-image built from the fields
//! that say *what happened, to what, by whom, and when*. Editing any of
//! them without the key invalidates the MAC.
//!
//! ## What this detects, and what it does not
//!
//! **Detects:** an audit row whose content was altered — the action verb
//! changed, the actor rewritten, the snapshot edited, the timestamp
//! moved. That is the common shape of covering one's tracks: leave the
//! row, change what it says.
//!
//! **Does not detect:** a row **deleted wholesale**, or rows reordered.
//! Nothing in a row can attest to its own continued existence. Catching
//! deletion needs a hash chain linking each row to its predecessor, plus
//! external-witness checkpoints so truncating the tail is visible — the
//! control the person, worker, care-pathway, and case services carry and
//! this one does not yet.
//!
//! That limit is stated here rather than left implicit, because a MAC on
//! every row looks like complete tamper-evidence and is not. It raises
//! the cost of a silent edit to holding the key; it does nothing about
//! `DELETE`.
//!
//! ## Why `created_at` is set explicitly
//!
//! The timestamp is part of the pre-image, so it has to be known before
//! the insert. Letting the database default it would mean writing the row
//! and then stamping the MAC in a second statement — a window in which
//! the row exists unprotected, and a second failure mode if the stamp
//! never lands.

use uuid::Uuid;

use super::mac::{self, Domain};

/// Field separator in the pre-image: ASCII unit separator, which cannot
/// appear in the JSON or the identifiers being joined.
const SEP: char = '\u{1f}';

/// Pre-image format version. Bound in as the first field, so a later
/// change to the format cannot be mistaken for tampering on old rows.
pub const AUDIT_MAC_VERSION: &str = "o-a1";

/// The fields an audit row's MAC covers.
#[derive(Debug, Clone)]
pub struct AuditInput<'a> {
    /// The organization `pid` the entry concerns.
    pub entity_pid: Uuid,
    /// The action verb.
    pub action: &'a str,
    /// The actor that caused the change.
    pub actor: Option<&'a str>,
    /// The recorded snapshot, if any.
    pub snapshot: Option<&'a serde_json::Value>,
    /// When it happened, epoch microseconds — the precision Postgres
    /// stores, so the value reproduces after a round-trip.
    pub created_at_micros: i64,
}

/// Build the MAC pre-image.
///
/// Every field is unit-separated so two different rows cannot share a
/// pre-image by shifting a character across a boundary.
#[must_use]
pub fn preimage(input: &AuditInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(AUDIT_MAC_VERSION);
    field(&input.entity_pid.to_string());
    field(input.action);
    field(input.actor.unwrap_or(""));
    field(&canonical_json(input.snapshot));
    field(&input.created_at_micros.to_string());
    buf
}

/// Canonical serialization of an optional JSON value: the empty string
/// for `None`, else `serde_json`'s output, whose key order is
/// lexicographic (`BTreeMap`, `preserve_order` disabled). That ordering
/// is load-bearing: a re-serialization that reordered keys would report
/// untouched rows as tampered.
///
/// A value that somehow fails to serialize degrades to a sentinel rather
/// than panicking, so a malformed snapshot cannot take the service down;
/// verification then reports a mismatch, which is the conservative
/// outcome.
fn canonical_json(value: Option<&serde_json::Value>) -> String {
    match value {
        None => String::new(),
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "\u{0}unserializable".to_string()),
    }
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
pub fn input_for(row: &crate::models::_entities::audit_logs::Model) -> AuditInput<'_> {
    AuditInput {
        entity_pid: row.entity_pid,
        action: row.action.as_str(),
        actor: row.actor.as_deref(),
        snapshot: row.snapshot.as_ref(),
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
    /// Rows whose MAC did **not** match: the content changed and whoever
    /// changed it did not hold the key.
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
     external-witness checkpoints, which this service does not yet have.";

/// Verify a run of audit rows.
#[must_use]
pub fn verify(rows: &[crate::models::_entities::audit_logs::Model]) -> AuditIntegrityReport {
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

        // The unkeyed digests are checked first because they are present
        // even when no MAC key is configured — on a default deployment
        // they are the only integrity these rows have.
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
            entity_pid: Uuid::from_u128(1),
            action: "created",
            actor: Some("alice"),
            snapshot: None,
            created_at_micros: 1_700_000_000_000_000,
        }
    }

    /// Every field is bound into the pre-image, so none can be edited
    /// without invalidating the MAC. A field left out would be one an
    /// attacker could rewrite freely — the actor especially, which is the
    /// field most worth falsifying.
    #[test]
    fn every_field_is_bound_into_the_preimage() {
        let base = preimage(&input());
        let mutate = |f: &dyn Fn(&mut AuditInput<'static>)| {
            let mut i = input();
            f(&mut i);
            preimage(&i)
        };
        assert_ne!(mutate(&|i| i.entity_pid = Uuid::from_u128(2)), base, "pid");
        assert_ne!(mutate(&|i| i.action = "deleted"), base, "action");
        assert_ne!(mutate(&|i| i.actor = Some("mallory")), base, "actor");
        assert_ne!(mutate(&|i| i.actor = None), base, "actor cleared");
        assert_ne!(mutate(&|i| i.created_at_micros = 1), base, "timestamp");
        let snapshot = serde_json::json!({"name": "x"});
        let mut with_snap = input();
        with_snap.snapshot = Some(&snapshot);
        assert_ne!(preimage(&with_snap), base, "snapshot");
    }

    /// The separator makes field boundaries unambiguous, so two rows
    /// cannot share a pre-image by shifting a character across one.
    #[test]
    fn field_boundaries_are_unambiguous() {
        let mut a = input();
        a.action = "ab";
        a.actor = Some("c");
        let mut b = input();
        b.action = "a";
        b.actor = Some("bc");
        assert_ne!(preimage(&a), preimage(&b));
    }

    /// The version tag leads the pre-image, so a future format change is
    /// distinguishable from tampering rather than looking like it.
    #[test]
    fn the_version_tag_leads_the_preimage() {
        let bytes = preimage(&input());
        assert!(bytes.starts_with(AUDIT_MAC_VERSION.as_bytes()));
    }
}
