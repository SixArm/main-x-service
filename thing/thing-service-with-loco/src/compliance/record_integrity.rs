//! Row-level integrity over the assembled `Thing` record.
//!
//! ## Why the assembled record, not the root row
//!
//! A thing's data lives across several tables — the root `things` row
//! plus its identifiers and links. Hashing only the root row would leave
//! the child tables unprotected, and an identifier is exactly the kind of
//! field worth editing quietly: it is what a downstream system matches
//! on. The pre-image therefore covers the **assembled domain record**,
//! the same value `GET /api/things/{id}` returns.
//!
//! The cost is one repository read per row at verification time, which is
//! why the verify endpoint caps its page size.
//!
//! ## What the three values are for
//!
//! SHA-256 and SHA3-256 are **unkeyed**: their pre-image format is
//! published, so anyone who can write SQL recomputes them. What they
//! catch is careless or unaware modification — a bug, a manual fix, a
//! restore from the wrong backup. The **MAC** is the one an adversary
//! holding only this database cannot forge.
//!
//! Kept as two digests rather than one for structural diversity: SHA-3 is
//! a sponge, unrelated to SHA-256's Merkle-Damgard chaining, so a
//! cryptanalytic advance against one design family does not transfer.

use serde::Serialize;
use uuid::Uuid;

use super::mac::{self, Domain};
use crate::models::thing::Thing;

/// Field separator: ASCII unit separator, which cannot appear in the
/// JSON or the identifiers being joined.
const SEP: char = '\u{1f}';

/// Pre-image format version, bound in as the first field so a later
/// change to the format cannot be mistaken for tampering on old rows.
pub const RECORD_HASH_VERSION: &str = "t-r1";

/// The fields a record's digests cover.
#[derive(Debug, Clone)]
pub struct RecordInput<'a> {
    /// The record's public id.
    pub id: Uuid,
    /// The assembled domain record, including its child tables.
    pub thing: &'a Thing,
    /// Whether the row is soft-deleted. In the pre-image because
    /// resurrecting a deleted record is a content change.
    pub is_deleted: bool,
}

/// Build the pre-image.
#[must_use]
pub fn preimage(input: &RecordInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(RECORD_HASH_VERSION);
    field(&input.id.to_string());
    // `serde_json`'s key order is lexicographic (BTreeMap;
    // `preserve_order` disabled), which is load-bearing: a
    // re-serialization that reordered keys would report untouched rows as
    // tampered. A value that fails to serialize degrades to a sentinel
    // rather than panicking, so a malformed record cannot take the
    // service down; verification then reports a mismatch, which is the
    // conservative outcome.
    field(
        &serde_json::to_string(input.thing).unwrap_or_else(|_| "\u{0}unserializable".to_string()),
    );
    field(if input.is_deleted { "1" } else { "0" });
    buf
}

/// Every digest for one record, computed from one pre-image.
///
/// A **named struct rather than a tuple**: with three values `.0`/`.1`/
/// `.2` is a latent bug, since putting the SHA-3 digest in the SHA-256
/// column type-checks and fails only at the next verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    /// FIPS 180-4 SHA-256.
    pub sha256: String,
    /// FIPS 202 SHA3-256.
    pub sha3: String,
    /// HMAC-SHA256 as `"<scheme>.<key id>:<hex>"`, or `None` when no key
    /// is configured. Unlike the digests this is **not** recomputable by
    /// someone holding only the database.
    pub mac: Option<String>,
}

/// All three digests from one call, so none can be stamped without the
/// others. Stamping one and forgetting another leaves a stale digest that
/// verification reports as tampering on an untouched record — a false
/// accusation, and the likeliest way this breaks.
#[must_use]
pub fn digests(input: &RecordInput<'_>) -> Digests {
    use sha2::Digest as _;

    let bytes = preimage(input);
    Digests {
        sha256: to_hex(&sha2::Sha256::digest(&bytes)),
        // Fully qualified: `sha2::Digest` and `sha3::Digest` are distinct
        // traits with the same method name, so importing both is
        // ambiguous.
        sha3: to_hex(&<sha3::Sha3_256 as sha3::Digest>::digest(&bytes)),
        mac: mac::tag(Domain::Record, &bytes),
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

/// One row as verification sees it.
///
/// A named struct rather than a tuple, for the reason the family learned
/// the hard way: a four-element tuple is how a MAC came to be written on
/// every record in three sibling services and then never verified — it
/// simply had nowhere to go, and nothing pointed that out.
#[derive(Debug, Clone)]
pub struct StoredRecord {
    /// The assembled domain record.
    pub thing: Thing,
    /// The stored SHA-256 digest.
    pub sha256: Option<String>,
    /// The stored SHA-3 digest.
    pub sha3: Option<String>,
    /// The stored keyed MAC.
    pub mac: Option<String>,
    /// The row's soft-delete flag.
    pub is_deleted: bool,
}

/// One record whose stored digests do not match its content.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordMismatch {
    /// The record's public id.
    pub id: String,
    /// The record's name, to make the report readable.
    pub name: String,
}

/// The outcome of verifying a set of records.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordIntegrityReport {
    /// Records examined.
    pub records: usize,
    /// Records whose SHA-256 was recomputed and matched.
    pub intact: usize,
    /// Records written before the column existed. Neither verified nor a
    /// mismatch — adopting the control on a populated table must not
    /// produce a wall of false positives.
    pub unhashed: usize,
    /// Records whose SHA-3 matched.
    pub sha3_intact: usize,
    /// Records carrying no SHA-3 digest.
    pub sha3_unhashed: usize,
    /// Records whose keyed MAC matched — the only counter an adversary
    /// holding just the database cannot inflate.
    pub mac_valid: usize,
    /// Records carrying no MAC (written before a key was configured).
    pub mac_absent: usize,
    /// Records whose MAC names a key or scheme this service cannot check.
    pub mac_unverifiable: usize,
    /// Every mismatch found.
    pub mismatched: Vec<RecordMismatch>,
    /// `true` when no mismatch was found.
    pub verified: bool,
}

/// Verify a set of records by recomputing each digest.
#[must_use]
pub fn verify(rows: &[StoredRecord]) -> RecordIntegrityReport {
    let mut report = RecordIntegrityReport {
        records: rows.len(),
        intact: 0,
        unhashed: 0,
        sha3_intact: 0,
        sha3_unhashed: 0,
        mac_valid: 0,
        mac_absent: 0,
        mac_unverifiable: 0,
        mismatched: Vec::new(),
        verified: true,
    };
    for row in rows {
        let Some(stored) = row.sha256.as_deref() else {
            report.unhashed += 1;
            continue;
        };
        let input = RecordInput {
            id: row.thing.id,
            thing: &row.thing,
            is_deleted: row.is_deleted,
        };
        let computed = digests(&input);
        let sha256_ok = stored == computed.sha256;
        if sha256_ok {
            report.intact += 1;
        }
        let sha3_ok = match row.sha3.as_deref() {
            None => {
                report.sha3_unhashed += 1;
                true
            }
            Some(s) => {
                let ok = s == computed.sha3;
                if ok {
                    report.sha3_intact += 1;
                }
                ok
            }
        };
        let mac_ok = match mac::verify(Domain::Record, row.mac.as_deref(), &preimage(&input)) {
            mac::MacVerdict::Valid => {
                report.mac_valid += 1;
                true
            }
            mac::MacVerdict::Absent => {
                report.mac_absent += 1;
                true
            }
            mac::MacVerdict::UnknownKey(_)
            | mac::MacVerdict::UnknownScheme(_)
            | mac::MacVerdict::Malformed => {
                report.mac_unverifiable += 1;
                true
            }
            mac::MacVerdict::Invalid => false,
        };
        // One entry per record, not one per algorithm: a tampered record
        // is a single incident.
        if !sha256_ok || !sha3_ok || !mac_ok {
            report.mismatched.push(RecordMismatch {
                id: row.thing.id.to_string(),
                name: row.thing.name.clone(),
            });
        }
    }
    report.verified = report.mismatched.is_empty();
    report
}

#[cfg(test)]
mod tests {
    use super::{RECORD_HASH_VERSION, RecordInput, StoredRecord, digests, preimage, verify};
    use crate::models::thing::Thing;
    use uuid::Uuid;

    fn thing(name: &str) -> Thing {
        let mut t = Thing::new(name);
        t.id = Uuid::from_u128(1);
        t
    }

    fn input(t: &Thing) -> RecordInput<'_> {
        RecordInput {
            id: t.id,
            thing: t,
            is_deleted: false,
        }
    }

    /// The version tag is published in `spec/12-compliance.md`, and
    /// changing it invalidates every stored digest.
    #[test]
    fn spec_documents_this_version_tag() {
        assert_eq!(RECORD_HASH_VERSION, "t-r1");
    }

    /// The tag leads the pre-image, so a future format change is
    /// distinguishable from tampering rather than looking like it.
    #[test]
    fn the_version_tag_leads_the_preimage() {
        let t = thing("Widget");
        assert!(preimage(&input(&t)).starts_with(RECORD_HASH_VERSION.as_bytes()));
    }

    /// Content changes change the digests. Without this the whole control
    /// is decorative.
    #[test]
    fn a_content_change_changes_every_digest() {
        let a = thing("Widget");
        let b = thing("Gadget");
        let da = digests(&input(&a));
        let db = digests(&input(&b));
        assert_ne!(da.sha256, db.sha256);
        assert_ne!(da.sha3, db.sha3);
    }

    /// The soft-delete flag is bound in, so resurrecting a deleted record
    /// by flipping the column is caught.
    #[test]
    fn the_soft_delete_flag_is_bound_in() {
        let t = thing("Widget");
        let live = digests(&input(&t));
        let deleted = digests(&RecordInput {
            id: t.id,
            thing: &t,
            is_deleted: true,
        });
        assert_ne!(live.sha256, deleted.sha256);
    }

    /// **Every stored digest is read, not merely written.** Three sibling
    /// services shipped a `content_mac` that was computed on every write
    /// and never verified; this pins that it cannot happen here.
    #[test]
    fn every_stored_digest_is_verified() {
        let t = thing("Widget");
        let d = digests(&input(&t));

        // All consistent ⇒ verified, with the MAC reported absent (no key
        // in unit tests) rather than as a mismatch.
        let report = verify(&[StoredRecord {
            thing: t.clone(),
            sha256: Some(d.sha256.clone()),
            sha3: Some(d.sha3.clone()),
            mac: None,
            is_deleted: false,
        }]);
        assert!(report.verified);
        assert_eq!(report.intact, 1);
        assert_eq!(report.sha3_intact, 1);
        assert_eq!(report.mac_absent, 1);

        // A stale SHA-3 alone is caught — the case that would pass if
        // only SHA-256 were checked.
        let report = verify(&[StoredRecord {
            thing: t.clone(),
            sha256: Some(d.sha256),
            sha3: Some("stale".to_string()),
            mac: None,
            is_deleted: false,
        }]);
        assert!(!report.verified, "a stale SHA-3 must be caught");

        // A MAC we cannot check is unverifiable, never a mismatch.
        let d2 = digests(&input(&t));
        let report = verify(&[StoredRecord {
            thing: t,
            sha256: Some(d2.sha256),
            sha3: Some(d2.sha3),
            mac: Some("d1.k9:00ff".to_string()),
            is_deleted: false,
        }]);
        assert_eq!(report.mac_unverifiable, 1);
        assert!(
            report.verified,
            "an unknown key is a configuration problem, not tampering"
        );
    }

    /// A row with no stored digest is unhashed, never a mismatch.
    #[test]
    fn an_unhashed_row_is_not_a_mismatch() {
        let report = verify(&[StoredRecord {
            thing: thing("Widget"),
            sha256: None,
            sha3: None,
            mac: None,
            is_deleted: false,
        }]);
        assert_eq!(report.unhashed, 1);
        assert!(report.verified);
    }
}
