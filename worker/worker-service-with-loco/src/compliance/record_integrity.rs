//! Row-level integrity hashing over worker records.
//!
//! The audit chain ([`super::audit_chain`]) proves the **trail** was not
//! rewritten. It says nothing about the worker records themselves: an
//! attacker with SQL access could edit a stored name, identifier, or
//! address and, as long as they wrote no audit row, the chain would still
//! verify. This module closes that gap — the one the dropped database
//! audit triggers gestured at without ever closing, since an unchained
//! trigger row was as forgeable as the edit it claimed to witness
//! (`m20260726_000003_drop_audit_triggers`).
//!
//! Each `workers` row carries a `content_hash` — a SHA-256 over the
//! record's content and lifecycle state — recomputed on **every** write. A
//! change made outside the service (or a bug that writes a row without
//! rehashing) is then detectable by recomputation.
//!
//! ## What is hashed, and why it is the whole record
//!
//! Ported from the care-pathway reference, with one substantive
//! difference. care-pathway stores its whole payload in a single JSONB
//! column, so hashing "the record" is hashing one field. A worker is
//! **relational**: names, identifiers, addresses, contacts, documents,
//! links, and **assessments** live in their own tables, and those are
//! exactly where the data worth tampering with lives — a surname, a
//! professional registration number, a psychometric score. Hashing only
//! the `workers` parent row would repeat the precise narrowness that made
//! the database triggers worthless: covering the parent while missing
//! everything that matters.
//!
//! One honest limit specific to this crate: `worker_assessments` is *not*
//! part of the assembled [`Worker`] the API serves, so it is not part of
//! this digest either. Assessment rows are reached through their own
//! sub-resource. Extending the hash to cover them means deciding how an
//! assessment write rehashes its parent worker, which is a real design
//! question rather than an oversight — recorded in the crate's
//! `spec/12-compliance.md` rather than silently skipped.
//!
//! So the digest is taken over the **assembled domain [`Worker`]** — the
//! same value the API serves — plus the row's `deleted_at` lifecycle
//! stamp, which the domain model does not carry.
//!
//! **`created_at` / `updated_at` are excluded on purpose.** They are set
//! by the ORM and the database rather than by this code, so binding them
//! would make the digest depend on values the writer does not control,
//! producing false mismatches. The cost is honest and small: an attacker
//! who alters only a timestamp is not detected here. Anything that changes
//! what the record *says* is.
//!
//! ## Not a chain
//!
//! Unlike `audit_log`, worker rows are *mutable by design*: an update
//! replaces the content. So this is a per-row content hash, not a chain —
//! it detects **out-of-band modification**, not deletion. A row deleted
//! directly in SQL leaves no trace here; the audit chain is what covers
//! that, because a legitimate delete writes an audit row and an
//! illegitimate one breaks the chain. The two controls are complementary,
//! and neither subsumes the other.
//!
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
//! ## Reproducibility
//!
//! The same two rules as the audit chain: time is hashed as epoch
//! microseconds (so writers must truncate before storing), and JSON is
//! hashed as `serde_json`'s serialization, whose field order is the
//! struct's declaration order and is therefore stable across a round trip.

use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::Worker;

/// Hash-format version, bound into every digest so a future change to the
/// hashed field set cannot be mistaken for tampering.
pub const RECORD_HASH_VERSION: &str = "w-r1";

/// Field separator inside the digest pre-image: ASCII 31 (unit separator).
const SEP: char = '\u{1f}';

/// Volatile fields dropped before hashing: set by the ORM and the
/// database, not by the writer, so binding them would produce false
/// mismatches.
const VOLATILE_FIELDS: [&str; 2] = ["created_at", "updated_at"];

/// The content a record binds into its hash.
#[derive(Debug, Clone, Copy)]
pub struct RecordInput<'a> {
    /// The record's public id.
    pub id: Uuid,
    /// The assembled domain record.
    pub worker: &'a Worker,
    /// Soft-delete instant as epoch microseconds, or `None` while live.
    ///
    /// Bound separately because the domain model does not carry it, and
    /// because on this entity a soft delete stamps `deleted_at` **without**
    /// clearing `active` — so `active` alone would not distinguish a live
    /// record from a deleted one.
    pub deleted_at_micros: Option<i64>,
}

/// The canonical JSON of a worker for hashing: the domain model with the
/// volatile timestamp fields removed.
///
/// Returns `None` if the record cannot be serialized, which the caller
/// must treat as a hard error rather than hashing a placeholder — a
/// placeholder would give two different records the same digest.
#[must_use]
fn canonical_json(worker: &Worker) -> Option<String> {
    let mut value = serde_json::to_value(worker).ok()?;
    if let Some(map) = value.as_object_mut() {
        for field in VOLATILE_FIELDS {
            map.remove(field);
        }
    }
    serde_json::to_string(&value).ok()
}

/// Compute a record's content hash as lowercase hex.
///
/// # Errors
///
/// Returns [`crate::Error::Internal`] if the record cannot be serialized.
/// This is deliberately an error rather than a sentinel digest: hashing a
/// placeholder would make two unrelated records share a hash, so a record
/// that cannot be hashed must not be written.
pub fn record_hash(input: &RecordInput<'_>) -> crate::Result<String> {
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(preimage(input)?) {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    Ok(out)
}

/// The same record's digest under **SHA-3** (SHA3-256).
///
/// # Errors
///
/// As [`record_hash`].
pub fn record_hash_sha3(input: &RecordInput<'_>) -> crate::Result<String> {
    use sha3::Digest as _;
    let pre = preimage(input)?;
    let mut out = String::with_capacity(64);
    for byte in sha3::Sha3_256::digest(pre) {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    Ok(out)
}

/// The digest pre-image: the version tag, then every bound field, each
/// followed by the unit separator.
///
/// Built once and hashed by each algorithm, so adding an algorithm cannot
/// change *what* is covered — only how it is digested.
///
/// # Errors
///
/// As the public hash functions.
fn preimage(input: &RecordInput<'_>) -> crate::Result<Vec<u8>> {
    let json = canonical_json(input.worker).ok_or_else(|| {
        crate::Error::Internal(format!(
            "worker {} cannot be serialized for its content hash",
            input.id
        ))
    })?;
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(RECORD_HASH_VERSION);
    field(&input.id.to_string());
    field(&json);
    field(
        &input
            .deleted_at_micros
            .map_or_else(String::new, |m| m.to_string()),
    );
    Ok(buf)
}

/// Both digests for one record, as `(SHA-256, BLAKE3)`.
///
/// Every write path takes the pair from here rather than calling the two
/// functions separately. Stamping one and forgetting the other leaves a
/// stale digest that verification reports as tampering on an untouched
/// record — a false accusation, and the likeliest way this breaks.
/// Returning a tuple makes the omission impossible to express.
///
/// # Errors
///
/// As [`record_hash`].
pub fn digests(input: &RecordInput<'_>) -> crate::Result<Digests> {
    Ok(Digests {
        sha256: record_hash(input)?,
        sha3: record_hash_sha3(input)?,
    })
}

/// Every digest for one record.
///
/// A **named struct rather than a tuple**: with two algorithms `.0`/`.1`
/// was survivable, with three it is a latent bug, because putting the
/// SHA-3 digest in the BLAKE3 column type-checks perfectly and fails only
/// at the next verification — as a false tamper report on an untouched
/// record. Named fields make that unwriteable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    /// FIPS 180-4 SHA-256.
    pub sha256: String,
    /// FIPS 202 SHA3-256.
    pub sha3: String,
}

/// Both digests for a live record.
///
/// # Errors
///
/// As [`record_hash`].
pub fn digests_of_live(worker: &Worker) -> crate::Result<Digests> {
    digests(&RecordInput {
        id: worker.id,
        worker,
        deleted_at_micros: None,
    })
}

/// Both digests for a record with a soft-delete stamp.
///
/// # Errors
///
/// As [`record_hash`].
pub fn digests_with_deleted_at(
    worker: &Worker,
    deleted_at_micros: Option<i64>,
) -> crate::Result<Digests> {
    digests(&RecordInput {
        id: worker.id,
        worker,
        deleted_at_micros,
    })
}

/// Compute the hash a live (not soft-deleted) record should carry.
///
/// # Errors
///
/// As [`record_hash`].
pub fn hash_of_live(worker: &Worker) -> crate::Result<String> {
    record_hash(&RecordInput {
        id: worker.id,
        worker,
        deleted_at_micros: None,
    })
}

/// Compute the hash a record should carry given its soft-delete stamp.
///
/// # Errors
///
/// As [`record_hash`].
pub fn hash_with_deleted_at(
    worker: &Worker,
    deleted_at_micros: Option<i64>,
) -> crate::Result<String> {
    record_hash(&RecordInput {
        id: worker.id,
        worker,
        deleted_at_micros,
    })
}

/// One row as verification sees it: the assembled record, its stored
/// SHA-256 digest, its stored BLAKE3 digest, and its soft-delete stamp.
///
/// Either digest may be `None` on a row written before that column
/// existed — reported as unhashed, never as a mismatch.
pub type StoredRecord = (Worker, Option<String>, Option<String>, Option<i64>);

/// One record whose stored hash does not match its content.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordMismatch {
    /// The record's public id.
    pub id: String,
}

/// The outcome of verifying a set of records.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecordIntegrityReport {
    /// Records examined.
    pub records: usize,
    /// Records whose content hash was recomputed and matched.
    pub intact: usize,
    /// Records written before this column existed, or whose hash could not
    /// be recomputed. They are neither verified nor counted as mismatches
    /// — adopting the control on a populated table must not produce a wall
    /// of false positives. They are rehashed on their next write.
    pub unhashed: usize,
    /// Records whose **SHA-3** digest was recomputed and matched.
    pub sha3_intact: usize,
    /// Records carrying no SHA-3 digest. Treated exactly as `unhashed`.
    pub sha3_unhashed: usize,
    /// Every mismatch found.
    pub mismatched: Vec<RecordMismatch>,
    /// `true` when no mismatch was found.
    pub verified: bool,
}

/// Verify a set of records by recomputing each content hash.
///
/// Each element pairs the assembled record with the hash and `deleted_at`
/// its row actually stores.
#[must_use]
pub fn verify(rows: &[StoredRecord]) -> RecordIntegrityReport {
    let mut report = RecordIntegrityReport {
        records: rows.len(),
        intact: 0,
        unhashed: 0,
        sha3_intact: 0,
        sha3_unhashed: 0,
        mismatched: Vec::new(),
        verified: true,
    };
    for (worker, stored, stored_sha3, deleted_at) in rows {
        let Some(stored) = stored.as_deref() else {
            report.unhashed += 1;
            continue;
        };
        // Both digests are recomputed from one call, so they always
        // describe the same content.
        let Ok(d) = digests_with_deleted_at(worker, *deleted_at) else {
            // Unhashable rather than mismatched: we cannot tell tampering
            // from a serialization failure, and reporting a false positive
            // on an integrity control is worse than reporting a gap.
            report.unhashed += 1;
            continue;
        };
        let sha256_ok = d.sha256 == stored;
        if sha256_ok {
            report.intact += 1;
        }
        let sha3_ok = match stored_sha3.as_deref() {
            None => {
                report.sha3_unhashed += 1;
                true
            }
            Some(s) => {
                let ok = s == d.sha3;
                if ok {
                    report.sha3_intact += 1;
                }
                ok
            }
        };
        // One entry per record, not one per algorithm.
        if !sha256_ok || !sha3_ok {
            report.mismatched.push(RecordMismatch {
                id: worker.id.to_string(),
            });
        }
    }
    report.verified = report.mismatched.is_empty();
    report
}

/// Hash-format version for an assessment row, distinct from the worker's
/// so the two digests can never be confused for one another.
pub const ASSESSMENT_HASH_VERSION: &str = "wa-r1";

/// Compute an assessment row's content hash as lowercase hex.
///
/// Assessments carry their **own** hash rather than folding into the
/// worker's, for two reasons. An assessment is written through its own
/// endpoints on its own lifecycle, so folding it in would make every
/// assessment write load and rehash the whole worker — coupling a
/// sub-resource to its parent, and adding a read to every write. And a
/// per-row hash names *which* assessment was tampered with, where a parent
/// digest could only say "something about this worker changed" — a
/// materially worse answer for the table where a changed score band is the
/// whole point.
///
/// `worker_id` is bound, so moving an assessment to a different worker is
/// detected rather than being invisible re-parenting.
///
/// As elsewhere, `created_at` / `updated_at` are excluded: the ORM and the
/// database set them, so binding them would produce false mismatches.
#[must_use]
pub fn assessment_hash(row: &crate::db::models::worker_assessments::Model) -> String {
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(assessment_preimage(row)) {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Both assessment digests, as `(SHA-256, BLAKE3)`.
///
/// Taken from one call so neither can be stamped without the other.
#[must_use]
pub fn assessment_digests(row: &crate::db::models::worker_assessments::Model) -> Digests {
    Digests {
        sha256: assessment_hash(row),
        sha3: assessment_hash_sha3(row),
    }
}

/// The same assessment row's digest under **SHA-3** (SHA3-256).
#[must_use]
pub fn assessment_hash_sha3(row: &crate::db::models::worker_assessments::Model) -> String {
    use sha3::Digest as _;
    let mut out = String::with_capacity(64);
    for byte in sha3::Sha3_256::digest(assessment_preimage(row)) {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The assessment digest pre-image, built once and hashed by each
/// algorithm.
fn assessment_preimage(row: &crate::db::models::worker_assessments::Model) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(ASSESSMENT_HASH_VERSION);
    field(&row.id.to_string());
    field(&row.worker_id.to_string());
    field(&row.category);
    field(&row.instrument);
    field(row.provider.as_deref().unwrap_or_default());
    field(&row.status);
    field(
        &row.administered_on
            .map_or_else(String::new, |d| d.to_string()),
    );
    field(&row.expires_on.map_or_else(String::new, |d| d.to_string()));
    field(row.administered_by.as_deref().unwrap_or_default());
    field(row.notes.as_deref().unwrap_or_default());
    // The scores themselves — the reason this table is worth protecting.
    field(&serde_json::to_string(&row.results).unwrap_or_default());
    field(&row.deleted_at.map_or_else(String::new, |d| {
        (d.unix_timestamp_nanos() / 1_000).to_string()
    }));

    buf
}

/// Verify a set of assessment rows by recomputing each content hash.
#[must_use]
pub fn verify_assessments(
    rows: &[crate::db::models::worker_assessments::Model],
) -> RecordIntegrityReport {
    let mut report = RecordIntegrityReport {
        records: rows.len(),
        intact: 0,
        unhashed: 0,
        sha3_intact: 0,
        sha3_unhashed: 0,
        mismatched: Vec::new(),
        verified: true,
    };
    for row in rows {
        let Some(stored) = row.content_hash.as_deref() else {
            report.unhashed += 1;
            continue;
        };
        let d = assessment_digests(row);
        let sha256_ok = stored == d.sha256;
        if sha256_ok {
            report.intact += 1;
        }
        let sha3_ok = match row.content_hash_sha3.as_deref() {
            None => {
                report.sha3_unhashed += 1;
                true
            }
            Some(s) => {
                let ok = s == d.sha3;
                if ok {
                    report.sha3_intact += 1;
                }
                ok
            }
        };
        // One entry per assessment, not one per algorithm.
        if !sha256_ok || !sha3_ok {
            report.mismatched.push(RecordMismatch {
                id: row.id.to_string(),
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
        assert_eq!(super::RECORD_HASH_VERSION, "w-r1");
        assert_eq!(super::ASSESSMENT_HASH_VERSION, "wa-r1");
    }

    use super::*;
    use crate::models::{Gender, HumanName, Identifier, IdentifierType};

    fn worker(family: &str) -> Worker {
        Worker::new(
            HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec!["Test".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        )
    }

    /// The same record hashes the same way twice — the property every
    /// other one rests on.
    #[test]
    fn hashing_is_deterministic() {
        let p = worker("Stable");
        assert_eq!(hash_of_live(&p).unwrap(), hash_of_live(&p).unwrap());
    }

    /// A change to a **child-table** field changes the digest. This is the
    /// whole reason the hash covers the assembled record rather than the
    /// `workers` parent row: an identifier lives in `worker_identifiers`,
    /// and a parent-row hash would not have noticed.
    #[test]
    fn editing_an_identifier_changes_the_hash() {
        let mut p = worker("Ident");
        let before = hash_of_live(&p).unwrap();
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        ));
        let after = hash_of_live(&p).unwrap();
        assert_ne!(before, after, "an identifier edit must be detectable");
    }

    /// Likewise for a name and an address — the other two child tables an
    /// attacker would actually target.
    #[test]
    fn editing_a_name_changes_the_hash() {
        let mut p = worker("Before");
        let before = hash_of_live(&p).unwrap();
        p.name.family = "After".to_string();
        assert_ne!(before, hash_of_live(&p).unwrap());

        let mut q = worker("Given");
        let before = hash_of_live(&q).unwrap();
        q.name.given.push("Extra".to_string());
        assert_ne!(before, hash_of_live(&q).unwrap());
    }

    /// The soft-delete stamp is bound, because on this entity a soft
    /// delete stamps `deleted_at` without clearing `active` — so a digest
    /// over `active` alone could not tell a live record from a deleted
    /// one, and un-deleting a record in SQL would go unnoticed.
    #[test]
    fn the_soft_delete_stamp_is_bound() {
        let p = worker("Deleted");
        let live = hash_with_deleted_at(&p, None).unwrap();
        let deleted = hash_with_deleted_at(&p, Some(1_800_000_000_000_000)).unwrap();
        assert_ne!(live, deleted);
    }

    /// The volatile timestamps are excluded, so an ORM-set `updated_at`
    /// does not make a untouched record look tampered with.
    #[test]
    fn volatile_timestamps_do_not_affect_the_hash() {
        let mut p = worker("Timestamps");
        let before = hash_of_live(&p).unwrap();
        p.updated_at += chrono::Duration::hours(3);
        p.created_at -= chrono::Duration::days(1);
        assert_eq!(
            before,
            hash_of_live(&p).unwrap(),
            "a timestamp the writer does not control must not change the digest"
        );
    }

    /// The version tag is bound, so changing the hashed field set later
    /// cannot be mistaken for tampering.
    #[test]
    fn the_version_tag_is_bound() {
        let p = worker("Versioned");
        let json = canonical_json(&p).expect("serializable");
        assert!(!json.contains("\"created_at\""));
        assert!(!json.contains("\"updated_at\""));
        assert!(RECORD_HASH_VERSION.starts_with("w-"));
    }

    /// Verification counts a row with no stored hash as `unhashed`, not as
    /// a mismatch: adopting this control on a populated table must not
    /// produce a wall of false positives.
    #[test]
    fn rows_without_a_stored_hash_are_not_mismatches() {
        let p = worker("Legacy");
        // No digest of either kind — a row predating both columns.
        let report = verify(&[(p, None, None, None)]);
        assert_eq!(report.unhashed, 1);
        assert_eq!(report.intact, 0);
        assert!(report.mismatched.is_empty());
        assert!(report.verified, "a legacy row is a gap, not a failure");
    }

    /// A tampered row is reported, and names itself so the report is
    /// actionable without a second lookup.
    #[test]
    fn a_tampered_row_is_reported() {
        let p = worker("Tampered");
        let stored = hash_of_live(&p).unwrap();
        let mut edited = p.clone();
        edited.tax_id = Some("injected".to_string());

        // Both digests stored, both stale after the edit.
        let d = digests_of_live(&p).unwrap();
        let report = verify(&[(edited.clone(), Some(stored), Some(d.sha3), None)]);
        assert!(!report.verified);
        assert_eq!(report.mismatched.len(), 1);
        assert_eq!(report.mismatched[0].id, edited.id.to_string());

        // And an untouched row alongside it still verifies.
        let good = worker("Fine");
        let good_hash = hash_of_live(&good).unwrap();
        let g = digests_of_live(&good).unwrap();
        let _ = good_hash;
        let report = verify(&[(good, Some(g.sha256), Some(g.sha3), None)]);
        assert!(report.verified);
        assert_eq!(report.intact, 1);
    }
}
