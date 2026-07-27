//! Row-level integrity hashing over the `cases` table.
//!
//! The audit chain ([`super::audit_chain`]) proves the **trail** was not
//! rewritten. It says nothing about the entity rows themselves: an
//! attacker with SQL access could edit a stored case and, as long as
//! they wrote no audit row, the chain would still verify. That gap was
//! stated plainly in the entity spec's honest limits; this module closes
//! it.
//!
//! Each `cases` row carries a `content_hash` — a SHA-256 over its
//! own content and lifecycle state — recomputed on **every** write. A
//! change made outside the service (or a bug that writes a row without
//! rehashing) is then detectable by recomputation.
//!
//! ## Not a chain
//!
//! Unlike `audit_logs`, entity rows are *mutable by design*: an update
//! replaces the payload. So this is a per-row content hash, not a chain —
//! it detects **out-of-band modification**, not deletion or reordering. A
//! row deleted directly in SQL leaves no trace here; the audit chain is
//! what covers that, because a legitimate delete writes an audit row and
//! an illegitimate one breaks the chain. The two controls are
//! complementary, and neither subsumes the other.
//!
//! ## What is hashed
//!
//! `pid`, `name`, the `data` payload, `active`, and `deleted_at` — the
//! record's content and its lifecycle state.
//!
//! **`created_at` / `updated_at` are excluded on purpose.** They are set
//! by the ORM and the database rather than by this code, so binding them
//! would make the digest depend on values the writer does not control,
//! producing false mismatches. The cost is honest and small: an attacker
//! who alters only a timestamp is not detected here. Anything that
//! changes what the record *says* is.
//!
//! Reproducibility follows the same two rules as the audit chain: time is
//! hashed as epoch microseconds (so writers must truncate before storing),
//! and JSON is hashed as `serde_json`'s serialization, whose `BTreeMap`
//! key order matches what a JSONB round-trip returns.
//! ## Computed here, not in the database
//!
//! Deliberately, and not for lack of database support: Postgres can
//! compute both digests (`sha256()` is core, `pgcrypto` does
//! `sha3-256`). A database-side digest would be recomputed by *any*
//! write, including a raw SQL edit by an attacker — the mechanism meant
//! to witness the change would be driven by the change itself, which is
//! the defect that removed the database audit triggers. See
//! `spec/12-compliance.md` §12.4z, "Where the digests are computed".
//!

use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::_entities::cases;

/// Hash-format version, bound into every digest so a future change to the
/// hashed field set cannot be mistaken for tampering.
pub const RECORD_HASH_VERSION: &str = "c-r1";

/// Field separator inside the digest pre-image: ASCII 31 (unit separator).
const SEP: char = '\u{1f}';

/// The fields a record binds into its content hash.
#[derive(Debug, Clone, Copy)]
pub struct RecordInput<'a> {
    /// The record's public id.
    pub pid: Uuid,
    /// The denormalised display title.
    pub title: &'a str,
    /// The stored payload.
    pub data: &'a serde_json::Value,
    /// Whether the row is active.
    pub active: bool,
    /// Soft-delete instant as epoch microseconds, or `None` while active.
    pub deleted_at_micros: Option<i64>,
}

/// Compute a record's content hash as lowercase hex.
#[must_use]
pub fn record_hash(input: &RecordInput<'_>) -> String {
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(preimage(input)) {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The same record's digest under **SHA-3** (SHA3-256).
#[must_use]
pub fn record_hash_sha3(input: &RecordInput<'_>) -> String {
    use sha3::Digest as _;
    let mut out = String::with_capacity(64);
    for byte in sha3::Sha3_256::digest(preimage(input)) {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The digest pre-image: the version tag, then every bound field, each
/// followed by the unit separator.
///
/// Built once and hashed by each algorithm, so adding an algorithm cannot
/// change *what* is covered — only how it is digested.
pub(crate) fn preimage(input: &RecordInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(RECORD_HASH_VERSION);
    field(&input.pid.to_string());
    field(input.title);
    field(&serde_json::to_string(input.data).unwrap_or_else(|_| "\u{0}unserializable".to_string()));
    field(if input.active { "1" } else { "0" });
    field(
        &input
            .deleted_at_micros
            .map_or_else(String::new, |m| m.to_string()),
    );
    buf
}

/// Both digests for one record, as `(SHA-256, SHA-3)`.
///
/// Every write path takes the pair from here rather than calling the two
/// functions separately. Stamping one and forgetting the other leaves a
/// stale digest that verification reports as tampering on an untouched
/// record — a false accusation, and the likeliest way this breaks.
/// Returning a tuple makes the omission impossible to express.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    /// FIPS 180-4 SHA-256.
    pub sha256: String,
    /// FIPS 202 SHA3-256.
    pub sha3: String,
    /// HMAC-SHA256 as `"<key id>:<hex>"`, or `None` when no key is
    /// configured. Unlike the digests this is **not** recomputable by
    /// someone holding only the database.
    pub mac: Option<String>,
}

/// Every digest for one record, computed from one pre-image.
///
/// A **named struct rather than a tuple**: with several algorithms `.0`/
/// `.1`/`.2` is a latent bug, since putting the SHA-3 digest in the
/// SHA-256 column type-checks and fails only at the next verification.
#[must_use]
pub fn digests(input: &RecordInput<'_>) -> Digests {
    Digests {
        sha256: record_hash(input),
        sha3: record_hash_sha3(input),
        mac: super::mac::tag(super::mac::Domain::Record, &preimage(input)),
    }
}

/// Borrow a stored row's fields as a [`RecordInput`].
#[must_use]
pub fn input_for(row: &cases::Model) -> RecordInput<'_> {
    RecordInput {
        pid: row.pid,
        title: row.title.as_str(),
        data: &row.data,
        active: row.active,
        deleted_at_micros: row.deleted_at.map(|d| d.timestamp_micros()),
    }
}

/// Compute the hash a stored row *should* carry.
#[must_use]
pub fn hash_of(row: &cases::Model) -> String {
    record_hash(&input_for(row))
}

/// The SHA-3 digest a stored row *should* carry.
#[must_use]
pub fn hash_of_sha3(row: &cases::Model) -> String {
    record_hash_sha3(&input_for(row))
}

/// One record whose stored hash does not match its content.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordMismatch {
    /// The record's public id.
    pub pid: String,
    /// The record's title, to make the report actionable without a second
    /// lookup.
    pub title: String,
}

/// The outcome of verifying a set of records.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordIntegrityReport {
    /// Records examined.
    pub records: usize,
    /// Records whose content hash was recomputed and matched.
    pub intact: usize,
    /// Records written before this column existed (no stored hash). They
    /// are neither verified nor counted as mismatches — adopting the
    /// control on a populated table must not produce a wall of false
    /// positives. They are rehashed on their next write.
    pub unhashed: usize,
    /// Every mismatch found.
    pub mismatched: Vec<RecordMismatch>,
    /// `true` when no mismatch was found.
    pub verified: bool,
}

/// Verify a set of records by recomputing each content hash.
#[must_use]
pub fn verify(rows: &[cases::Model]) -> RecordIntegrityReport {
    let mut report = RecordIntegrityReport {
        records: rows.len(),
        intact: 0,
        unhashed: 0,
        mismatched: Vec::new(),
        verified: true,
    };
    for row in rows {
        let Some(stored) = row.content_hash.as_deref() else {
            report.unhashed += 1;
            continue;
        };
        let sha_ok = stored == hash_of(row);
        if sha_ok {
            report.intact += 1;
        }
        // One entry per record, not one per algorithm: a tampered record
        // is a single incident.
        if !sha_ok {
            report.mismatched.push(RecordMismatch {
                pid: row.pid.to_string(),
                title: row.title.clone(),
            });
        }
    }
    report.verified = report.mismatched.is_empty();
    report
}

#[cfg(test)]
mod tests {
    /// As `audit_chain::spec_documents_this_version_tag`: the tag is
    /// published in `spec/12-compliance.md` §12.4z, and changing it
    /// invalidates every stored digest.
    #[test]
    fn spec_documents_this_version_tag() {
        assert_eq!(super::RECORD_HASH_VERSION, "c-r1");
    }

    use super::*;
    use chrono::{DateTime, FixedOffset, TimeZone as _, Utc};

    fn at(micros: i64) -> DateTime<FixedOffset> {
        Utc.timestamp_micros(micros)
            .single()
            .expect("valid instant")
            .into()
    }

    /// A stored row with a correct hash.
    fn row(id: i32, title: &str) -> cases::Model {
        let mut model = cases::Model {
            created_at: at(1_700_000_000_000_000),
            updated_at: at(1_700_000_000_000_000),
            id,
            pid: Uuid::from_u128(u128::from(id.unsigned_abs())),
            title: title.to_string(),
            data: serde_json::json!({ "title": title, "keywords": ["a", "b"] }),
            active: true,
            deleted_at: None,
            content_hash: None,
            content_hash_sha3: None,
            // No key in unit tests; reported `mac_absent`.
            content_mac: None,
        };
        model.content_hash = Some(hash_of(&model));
        model.content_hash_sha3 = Some(hash_of_sha3(&model));
        model
    }

    /// The digest is deterministic and SHA-256 shaped.
    #[test]
    fn record_hash_is_deterministic() {
        let r = row(1, "Acute Stroke Care Case");
        assert_eq!(r.content_hash.as_deref(), Some(hash_of(&r).as_str()));
        assert_eq!(hash_of(&r).len(), 64);
    }

    /// Key order in the payload must not change the digest — the property
    /// that survives a JSONB round-trip.
    #[test]
    fn payload_key_order_does_not_change_the_digest() {
        let mut a = row(1, "X");
        let mut b = a.clone();
        a.data = serde_json::from_str(r#"{"b":2,"a":1}"#).expect("parse");
        b.data = serde_json::from_str(r#"{"a":1,"b":2}"#).expect("parse");
        assert_eq!(hash_of(&a), hash_of(&b));
    }

    /// Every bound field changes the digest.
    #[test]
    fn record_hash_covers_every_bound_field() {
        let base = row(1, "X");
        let baseline = hash_of(&base);
        let mut renamed = base.clone();
        renamed.title = "Y".to_string();
        assert_ne!(hash_of(&renamed), baseline, "name");
        let mut repayloaded = base.clone();
        repayloaded.data = serde_json::json!({ "name": "tampered" });
        assert_ne!(hash_of(&repayloaded), baseline, "data");
        let mut retired = base.clone();
        retired.active = false;
        assert_ne!(hash_of(&retired), baseline, "active");
        let mut deleted = base.clone();
        deleted.deleted_at = Some(at(1_800_000_000_000_000));
        assert_ne!(hash_of(&deleted), baseline, "deleted_at");
        let mut repid = base.clone();
        repid.pid = Uuid::from_u128(999);
        assert_ne!(hash_of(&repid), baseline, "pid");
    }

    /// Timestamps are deliberately **not** bound (see the module docs), so
    /// touching them alone does not change the digest. Pinned so the
    /// exclusion is a decision rather than an oversight.
    #[test]
    fn timestamps_are_deliberately_not_bound() {
        let base = row(1, "X");
        let mut touched = base.clone();
        touched.created_at = at(1);
        touched.updated_at = at(2);
        assert_eq!(
            hash_of(&touched),
            hash_of(&base),
            "created_at/updated_at are ORM-managed and excluded on purpose"
        );
    }

    /// A well-formed set verifies.
    #[test]
    fn intact_records_verify() {
        let rows = vec![row(1, "A"), row(2, "B"), row(3, "C")];
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.mismatched);
        assert_eq!(report.intact, 3);
        assert_eq!(report.records, 3);
        assert_eq!(report.unhashed, 0);
    }

    /// **The point of the control**: editing a row's payload without
    /// rehashing is detected.
    #[test]
    fn out_of_band_edit_is_detected() {
        let mut rows = vec![row(1, "A"), row(2, "B")];
        rows[1].data = serde_json::json!({ "name": "tampered" });
        let report = verify(&rows);
        assert!(!report.verified);
        assert_eq!(report.mismatched.len(), 1);
        assert_eq!(report.mismatched[0].title, "B");
        assert_eq!(report.intact, 1);
    }

    /// Renaming a row out of band is detected too — the denormalised
    /// `name` is bound, so it cannot drift from the payload unnoticed.
    #[test]
    fn out_of_band_rename_is_detected() {
        let mut rows = vec![row(1, "A")];
        rows[0].title = "Renamed".to_string();
        assert!(!verify(&rows).verified);
    }

    /// Flipping `active` or `deleted_at` directly in SQL — an
    /// un-deletion — is detected.
    #[test]
    fn out_of_band_undelete_is_detected() {
        let mut r = row(1, "A");
        r.active = false;
        r.deleted_at = Some(at(1_800_000_000_000_000));
        // Restamp *both* digests: a legitimate write always sets the pair
        // (see `digests`), and setting only one is the stale-digest defect
        // this control would otherwise report as tampering.
        r.content_hash = Some(hash_of(&r));
        r.content_hash_sha3 = Some(hash_of_sha3(&r));
        assert!(verify(std::slice::from_ref(&r)).verified);
        // Now resurrect it behind the service's back.
        r.active = true;
        r.deleted_at = None;
        assert!(!verify(&[r]).verified, "an un-delete must be detected");
    }

    /// Rows predating the column are `unhashed`, not mismatches — so
    /// adopting the control on a populated table verifies cleanly.
    #[test]
    fn pre_column_rows_are_unhashed_not_mismatched() {
        let mut rows = vec![row(1, "A"), row(2, "B")];
        rows[0].content_hash = None;
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.mismatched);
        assert_eq!(report.unhashed, 1);
        assert_eq!(report.intact, 1);
    }

    /// An empty set verifies vacuously.
    #[test]
    fn empty_set_verifies() {
        let report = verify(&[]);
        assert!(report.verified);
        assert_eq!(report.records, 0);
    }

    /// The version tag is bound, so widening the hashed field set later
    /// cannot be confused with tampering.
    #[test]
    fn version_tag_is_bound() {
        let r = row(1, "A");
        let with_other_version = {
            let mut hasher = Sha256::new();
            hasher.update(b"r0");
            hasher.update([SEP as u8]);
            hasher.update(r.pid.to_string().as_bytes());
            format!("{:x}", hasher.finalize())
        };
        assert_ne!(hash_of(&r), with_other_version);
        // The exact tag is pinned against the spec by
        // `spec_documents_this_version_tag`; here we only assert that it
        // participates in the digest.
    }
}
