//! Tamper-evident audit history — a SHA-256 hash chain over `audit_logs`.
//!
//! HIPAA §164.312(c)(1)–(2) requires protecting records from improper
//! alteration and providing a **mechanism to corroborate that data has not
//! been altered**. An append-only table is a convention; a hash chain is
//! evidence. Every audit row binds its own content **and its predecessor's
//! hash**, so inserting, deleting, reordering, or editing any row breaks
//! verification at that point and everywhere after it.
//!
//! ## The hash
//!
//! ```text
//! hash = SHA-256( "v1" ␟ prev_hash ␟ entity_pid ␟ action ␟ actor
//!                      ␟ created_at_micros ␟ snapshot ␟ context ␟ disclosure )
//! ```
//!
//! Two properties make the digest reproducible at verification time from
//! the row as it comes **back out of Postgres**:
//!
//! - **Time is an integer.** `created_at` is hashed as microseconds since
//!   the Unix epoch, not as a formatted string, so the session time zone
//!   and the subsecond precision Postgres keeps cannot change the digest.
//!   Writers must truncate to microseconds before inserting
//!   ([`crate::models::audit_logs`] does).
//! - **JSON is canonical.** `snapshot` / `context` are hashed as
//!   `serde_json`'s serialization of the parsed value. `serde_json::Map` is
//!   a `BTreeMap` here (the `preserve_order` feature is off), so keys are
//!   emitted in lexicographic order both when writing and after a JSONB
//!   round-trip, which reorders keys server-side.
//!
//! ## GDPR Art. 17 and redaction
//!
//! Erasure and an immutable trail collide. The resolution
//! (`agents/share/compliance-for-healthcare.md` §2.2) is **redaction**: a
//! row's content is destroyed and `redacted_at` is stamped, while its
//! stored `hash` and `prev_hash` survive. [`verify`] then skips the
//! content check for that row but still checks its **linkage**, so the
//! chain remains verifiable end to end and still proves the event
//! occurred — without retaining the erased data.
//!
//! ## Scope, stated plainly
//!
//! The chain proves the **audit trail** was not rewritten. It does **not**
//! prove the `care_pathways` rows were not; row-level integrity hashing
//! over the entity table is not built.

use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::_entities::audit_logs;

/// Chain-format version, bound into every digest so a future change to the
/// hashed field set cannot be confused with tampering.
pub const CHAIN_VERSION: &str = "v1";

/// Field separator inside the digest pre-image: ASCII 31 (unit separator),
/// which cannot occur in a UUID, an action verb, or serialized JSON.
const SEP: char = '\u{1f}';

/// The fields one audit row binds into its chain hash.
#[derive(Debug, Clone, Copy)]
pub struct ChainInput<'a> {
    /// The preceding row's [`ChainInput::hash`], or `None` for the genesis row.
    pub prev_hash: Option<&'a str>,
    /// The care-pathway `pid` the entry concerns.
    pub entity_pid: Uuid,
    /// The action verb (`created` / `read` / `merged` / `erased` / …).
    pub action: &'a str,
    /// The acting user's `sub`, or `None` when unauthenticated.
    pub actor: Option<&'a str>,
    /// `created_at` as microseconds since the Unix epoch (time-zone free).
    pub created_at_micros: i64,
    /// The payload snapshot, if any.
    pub snapshot: Option<&'a serde_json::Value>,
    /// The request/processing context, if any.
    pub context: Option<&'a serde_json::Value>,
    /// Whether the access was an outward disclosure.
    pub disclosure: bool,
}

/// Compute a row's chain hash as lowercase hex.
#[must_use]
pub fn row_hash(input: &ChainInput<'_>) -> String {
    to_hex(&Sha256::digest(preimage(input)))
}

/// The same row's digest under **SHA-3** (SHA3-256).
///
/// Third sibling of [`row_hash`] and [`row_hash_blake3`], over the
/// byte-identical pre-image. SHA-3 is a sponge construction, unrelated to
/// SHA-256's Merkle-Damgard chaining and to BLAKE3's ARX tree, so the
/// three span three distinct design families.
#[must_use]
pub fn row_hash_sha3(input: &ChainInput<'_>) -> String {
    use sha3::Digest as _;
    to_hex(&sha3::Sha3_256::digest(preimage(input)))
}

/// The digest pre-image: version tag, then every bound field, each
/// followed by the unit separator.
///
/// Built once and hashed by each algorithm, so adding an algorithm cannot
/// accidentally change *what* is covered — only how it is digested.
pub(crate) fn preimage(input: &ChainInput<'_>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    let mut field = |value: &str| {
        buf.extend_from_slice(value.as_bytes());
        buf.push(SEP as u8);
    };
    field(CHAIN_VERSION);
    field(input.prev_hash.unwrap_or(""));
    field(&input.entity_pid.to_string());
    field(input.action);
    field(input.actor.unwrap_or(""));
    field(&input.created_at_micros.to_string());
    field(&canonical_json(input.snapshot));
    field(&canonical_json(input.context));
    field(if input.disclosure { "1" } else { "0" });
    buf
}

/// Canonical serialization of an optional JSON value: the empty string for
/// `None`, else `serde_json`'s output (lexicographic key order — see the
/// module docs). A value that somehow fails to serialize degrades to a
/// sentinel rather than panicking, so a malformed snapshot can never take
/// the service down; verification then reports a content break, which is
/// the correct, conservative outcome.
fn canonical_json(value: Option<&serde_json::Value>) -> String {
    match value {
        None => String::new(),
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "\u{0}unserializable".to_string()),
    }
}

/// Lowercase hex encoding (no external dependency).
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Infallible: writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Borrow a stored row's fields as a [`ChainInput`], using `prev` as the
/// predecessor hash. Used both when writing (with the current chain head)
/// and when verifying (with the previous row's stored hash).
#[must_use]
pub fn input_for<'a>(row: &'a audit_logs::Model, prev: Option<&'a str>) -> ChainInput<'a> {
    ChainInput {
        prev_hash: prev,
        entity_pid: row.entity_pid,
        action: row.action.as_str(),
        actor: row.actor.as_deref(),
        created_at_micros: row.created_at.timestamp_micros(),
        snapshot: row.snapshot.as_ref(),
        context: row.context.as_ref(),
        disclosure: row.disclosure,
    }
}

/// One detected break in the chain.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChainBreak {
    /// The `audit_logs.id` at which the break was detected.
    pub id: i32,
    /// `linkage` — the row does not point at its predecessor's hash (a row
    /// was inserted, deleted, or reordered); `content` — the row's stored
    /// hash does not match its own content (the row was edited).
    pub kind: &'static str,
    /// Human-readable detail for an operator.
    pub detail: String,
}

/// The outcome of verifying a contiguous, ascending-`id` run of rows.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChainReport {
    /// Rows examined.
    pub rows: usize,
    /// Rows whose content hash was recomputed and matched.
    pub intact: usize,
    /// Rows redacted under GDPR Art. 17 — linkage checked, content skipped.
    pub redacted: usize,
    /// Rows written before the chain existed (no stored hash); they are
    /// neither verified nor counted as breaks, and they reset the linkage
    /// expectation for the row that follows.
    pub unchained: usize,
    /// Rows whose **SHA-3** digest was recomputed and matched.
    pub sha3_intact: usize,
    /// Rows carrying no SHA-3 digest — written before the third algorithm
    /// was adopted. Neither verified nor a break.
    pub sha3_unhashed: usize,
    /// Rows whose **keyed MAC** was recomputed and matched — the only
    /// counter an adversary holding just the database cannot inflate.
    pub mac_valid: usize,
    /// Rows carrying no MAC (written before a key was configured).
    pub mac_absent: usize,
    /// Rows whose MAC names a key this service does not hold.
    pub mac_unverifiable: usize,
    /// Every break found, in `id` order. Empty ⇒ the run verifies.
    pub breaks: Vec<ChainBreak>,
    /// The hash of the last chained row examined — the chain head an
    /// operator can record externally to detect wholesale truncation.
    pub head: Option<String>,
    /// `true` when no breaks were found.
    pub verified: bool,
}

/// Verify a run of audit rows supplied in **ascending `id` order**.
///
/// Each chained row must (a) carry its predecessor's stored hash and
/// (b) — unless redacted — hash to its stored value. A row with no stored
/// hash predates the chain: it is counted as `unchained` and clears the
/// linkage expectation, so introducing the chain to an existing table does
/// not produce a spurious break at the boundary.
#[must_use]
/// Recompute every digest and the MAC for one row, updating the report's
/// counters and returning which checks passed.
///
/// Split out of [`verify`] to keep it readable: the loop is otherwise a
/// hundred lines of three near-identical arms, and the differences
/// between them — which counter, which predecessor, which verdict is
/// tolerated — are exactly what a reader needs to see.
/// Recompute every digest and the MAC for one row, updating the report's
/// counters and returning which checks passed.
///
/// Split out of [`verify`] to keep it readable: the loop is otherwise a
/// hundred lines of three near-identical arms, and the differences
/// between them — which counter, which predecessor, which verdict is
/// tolerated — are exactly what a reader needs to see.
fn check_row(
    row: &audit_logs::Model,
    stored: &str,
    report: &mut ChainReport,
) -> (bool, bool, bool) {
    let sha256_ok = row_hash(&input_for(row, row.prev_hash.as_deref())) == stored;
    if sha256_ok {
        report.intact += 1;
    }
    let sha3_ok = match row.hash_sha3.as_deref() {
        None => {
            report.sha3_unhashed += 1;
            true
        }
        Some(stored_sha3) => {
            let ok = row_hash_sha3(&input_for(row, row.prev_hash_sha3.as_deref())) == stored_sha3;
            if ok {
                report.sha3_intact += 1;
            }
            ok
        }
    };
    // **One** break per row, naming which digests disagreed —
    // reporting the same tampered row twice would double-count it
    // and make the break list read as two separate incidents.
    // Which algorithms disagree is itself diagnostic: both means
    // the content changed, exactly one means that digest column
    // was edited or a hashing path is inconsistent.
    // The keyed check: a mismatch here means the content changed
    // and whoever changed it did not hold the key.
    let mac_ok = match super::mac::verify(
        row.mac.as_deref(),
        &preimage(&input_for(row, row.prev_hash.as_deref())),
    ) {
        super::mac::MacVerdict::Valid => {
            report.mac_valid += 1;
            true
        }
        super::mac::MacVerdict::Absent => {
            report.mac_absent += 1;
            true
        }
        super::mac::MacVerdict::UnknownKey(_) | super::mac::MacVerdict::Malformed => {
            report.mac_unverifiable += 1;
            true
        }
        super::mac::MacVerdict::Invalid => false,
    };
    (sha256_ok, sha3_ok, mac_ok)
}

/// Verify a run of audit rows supplied in **ascending `id` order**.
///
/// Each chained row must (a) carry its predecessor's stored hash and
/// (b) — unless redacted — hash to its stored value, under every
/// algorithm it carries. A row with no stored hash predates the chain: it
/// is counted as `unchained` and clears the linkage expectation, so
/// introducing the chain to an existing table does not produce a spurious
/// break at the boundary.
#[must_use]
pub fn verify(rows: &[audit_logs::Model]) -> ChainReport {
    let mut report = ChainReport {
        rows: rows.len(),
        intact: 0,
        redacted: 0,
        unchained: 0,
        sha3_intact: 0,
        sha3_unhashed: 0,
        mac_valid: 0,
        mac_absent: 0,
        mac_unverifiable: 0,
        breaks: Vec::new(),
        head: None,
        verified: true,
    };
    // `None` = no expectation yet (start of run, or just after an
    // unchained row). `Some(x)` = the next row must carry `prev_hash == x`.
    let mut expected_prev: Option<Option<String>> = None;

    for row in rows {
        let Some(stored) = row.hash.as_deref() else {
            report.unchained += 1;
            expected_prev = None;
            continue;
        };
        if let Some(expected) = &expected_prev
            && row.prev_hash.as_deref() != expected.as_deref()
        {
            report.breaks.push(ChainBreak {
                id: row.id,
                kind: "linkage",
                detail: format!(
                    "prev_hash {} does not match the preceding row's hash {}",
                    row.prev_hash.as_deref().unwrap_or("<none>"),
                    expected.as_deref().unwrap_or("<none>"),
                ),
            });
        }
        if row.redacted_at.is_some() {
            report.redacted += 1;
        } else {
            let (sha256_ok, sha3_ok, mac_ok) = check_row(row, stored, &mut report);
            if !sha256_ok || !sha3_ok || !mac_ok {
                // Name every algorithm that disagreed. All three means the
                // content changed; a subset means those digest columns
                // were edited, or a write path stamped some and not
                // others — different incidents needing different fixes.
                let mut disagreed = Vec::new();
                if !sha256_ok {
                    disagreed.push("SHA-256");
                }
                if !sha3_ok {
                    disagreed.push("SHA-3");
                }
                if !mac_ok {
                    disagreed.push("the keyed MAC");
                }
                let agreed = 3 - disagreed.len();
                let hint = if agreed == 0 {
                    " (all digests disagree — the content changed)"
                } else {
                    " (the other digests still match — suspect those hash columns)"
                };
                report.breaks.push(ChainBreak {
                    id: row.id,
                    kind: "content",
                    detail: format!(
                        "row content does not match its stored {} hash{hint}",
                        disagreed.join(" and ")
                    ),
                });
            }
        }
        expected_prev = Some(Some(stored.to_string()));
        report.head = Some(stored.to_string());
    }
    report.verified = report.breaks.is_empty();
    report
}

#[cfg(test)]
mod tests {
    /// The two chains are **independent**: the SHA-3 digest binds the
    /// SHA-3 predecessor, not the SHA-256 one.
    ///
    /// This is the property that makes keeping two algorithms worth the
    /// columns. Had the SHA-3 digest been taken over a pre-image binding
    /// the *SHA-256* predecessor — the obvious shortcut, since it needs
    /// only one pre-image — then forging a predecessor with a colliding
    /// SHA-256 hash would leave the successor's pre-image unchanged and
    /// *both* digests still valid. The second algorithm would inherit the
    /// first's weakness and attest to nothing.
    #[test]
    fn the_sha3_chain_does_not_depend_on_the_sha256_chain() {
        let rows = chain(3);
        // Same content, same position, different SHA-3 predecessor ⇒
        // different SHA-3 digest. If the SHA-3 digest bound the SHA-256
        // predecessor instead, these would be equal.
        let a = row_with(2, rows[0].hash.as_deref(), Some("aaaa"), "created", None);
        let b = row_with(2, rows[0].hash.as_deref(), Some("bbbb"), "created", None);
        assert_eq!(a.hash, b.hash, "the SHA-256 digest is unaffected");
        assert_ne!(
            a.hash_sha3, b.hash_sha3,
            "the SHA-3 digest must bind its own predecessor"
        );
    }

    /// A row predating the second algorithm is counted, not failed — the
    /// same tolerance the chain already extends to pre-chain rows.
    #[test]
    fn rows_without_a_sha3_digest_are_counted_not_broken() {
        let mut rows = chain(2);
        rows[1].hash_sha3 = None;
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.sha3_unhashed, 1);
        assert_eq!(report.sha3_intact, 1);
    }

    /// Editing content breaks **both** digests, and is reported once.
    #[test]
    fn tampering_breaks_both_digests_but_reports_one_incident() {
        let mut rows = chain(3);
        rows[1].snapshot = Some(serde_json::json!({ "name": "Tampered" }));
        let report = verify(&rows);
        assert!(!report.verified);
        let content: Vec<&ChainBreak> = report
            .breaks
            .iter()
            .filter(|b| b.kind == "content")
            .collect();
        assert_eq!(content.len(), 1, "one incident, not one per algorithm");
        assert!(
            content[0].detail.contains("SHA-256 and SHA-3"),
            "the report must name both: {}",
            content[0].detail
        );
    }

    /// **Golden vectors.** A fixed input hashes to these exact digests.
    ///
    /// Every other test in this module recomputes with the same code, so
    /// they stay green even if the pre-image changes — and a changed
    /// pre-image silently invalidates every digest already stored, which
    /// is indistinguishable from mass tampering. These constants are the
    /// only thing standing between a refactor and that outcome.
    ///
    /// If this test fails, the hash format changed. That is a breaking
    /// change to stored data: bump the version tag deliberately and plan
    /// the migration — do not update the constants to match.
    #[test]
    fn golden_vectors_pin_the_wire_format() {
        let input = ChainInput {
            prev_hash: Some("0123456789abcdef"),
            entity_pid: Uuid::from_u128(42),
            action: "created",
            actor: Some("alice"),
            created_at_micros: 1_700_000_000_000_000,
            snapshot: None,
            context: None,
            disclosure: false,
        };
        assert_eq!(
            row_hash(&input),
            "14f23b709d100add4394c8f6ee792260a0023dfa1c59af7dc6dd1faadc92c56d"
        );
        assert_eq!(
            row_hash_sha3(&input),
            "32e550558df063509903945a05fc56903cc12bf12d9dfe9e5e567988d07d6c72"
        );
    }

    /// The version tag is published in this entity's
    /// `spec/12-compliance.md` §12.4z hashing reference, and a reader
    /// verifying a digest by hand relies on it. Changing the constant
    /// without changing the spec makes that reference silently wrong —
    /// and changing it at all invalidates every stored digest, so this
    /// pin exists to make the decision deliberate rather than incidental.
    #[test]
    fn spec_documents_this_version_tag() {
        assert_eq!(super::CHAIN_VERSION, "v1");
    }

    use super::*;
    use chrono::{DateTime, FixedOffset, TimeZone as _, Utc};

    /// A fixed instant so digests in these tests are stable.
    fn at(micros: i64) -> DateTime<FixedOffset> {
        Utc.timestamp_micros(micros)
            .single()
            .expect("valid instant")
            .into()
    }

    /// Build a chained row the way the writer does, binding an explicit
    /// BLAKE3 predecessor so the fixture builds *both* chains — otherwise
    /// every test row would be `blake3_unhashed` and the second algorithm
    /// would go untested.
    fn row_with(
        id: i32,
        prev: Option<&str>,
        prev_sha3: Option<&str>,
        action: &str,
        snapshot: Option<serde_json::Value>,
    ) -> audit_logs::Model {
        let mut model = audit_logs::Model {
            created_at: at(1_700_000_000_000_000 + i64::from(id)),
            updated_at: at(1_700_000_000_000_000 + i64::from(id)),
            id,
            entity_pid: Uuid::from_u128(u128::from(id.unsigned_abs())),
            action: action.to_string(),
            actor: Some("alice".to_string()),
            snapshot,
            prev_hash: prev.map(ToString::to_string),
            hash: None,
            prev_hash_sha3: prev_sha3.map(ToString::to_string),
            hash_sha3: None,
            // No key in unit tests: the MAC is absent, which verification
            // reports as `mac_absent` rather than as a mismatch.
            mac: None,
            context: None,
            disclosure: false,
            redacted_at: None,
        };
        model.hash = Some(row_hash(&input_for(&model, prev)));
        model.hash_sha3 = Some(row_hash_sha3(&input_for(&model, prev_sha3)));
        model
    }

    /// Build a well-formed chain of `n` rows.
    fn chain(n: i32) -> Vec<audit_logs::Model> {
        let mut rows: Vec<audit_logs::Model> = Vec::new();
        for id in 1..=n {
            let prev = rows.last().and_then(|r| r.hash.clone());
            let prev_sha3 = rows.last().and_then(|r| r.hash_sha3.clone());
            rows.push(row_with(
                id,
                prev.as_deref(),
                prev_sha3.as_deref(),
                "created",
                Some(serde_json::json!({ "name": format!("pathway {id}") })),
            ));
        }
        rows
    }

    /// The digest is deterministic for identical input.
    #[test]
    fn row_hash_is_deterministic() {
        let rows = chain(1);
        let again = row_hash(&input_for(&rows[0], None));
        assert_eq!(rows[0].hash.as_deref(), Some(again.as_str()));
        assert_eq!(again.len(), 64, "SHA-256 hex is 64 chars");
    }

    /// Key order in the snapshot must not change the digest — the property
    /// that survives a JSONB round-trip (Postgres reorders keys).
    #[test]
    fn canonical_json_is_key_order_independent() {
        let a: serde_json::Value = serde_json::from_str(r#"{"b":2,"a":1}"#).expect("parse");
        let b: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).expect("parse");
        assert_eq!(canonical_json(Some(&a)), canonical_json(Some(&b)));
    }

    /// Re-hash `base` with one field changed by `apply`.
    fn hash_with(base: &audit_logs::Model, apply: impl FnOnce(&mut audit_logs::Model)) -> String {
        let mut model = base.clone();
        apply(&mut model);
        row_hash(&input_for(&model, None))
    }

    /// Changing any bound field changes the digest.
    #[test]
    fn row_hash_covers_every_bound_field() {
        let base = chain(1).remove(0);
        let baseline = base.hash.clone().expect("hashed");
        let variants = [
            ("action", hash_with(&base, |m| m.action = "deleted".into())),
            (
                "actor",
                hash_with(&base, |m| m.actor = Some("mallory".into())),
            ),
            ("created_at", hash_with(&base, |m| m.created_at = at(1))),
            (
                "snapshot",
                hash_with(&base, |m| {
                    m.snapshot = Some(serde_json::json!({ "name": "tampered" }));
                }),
            ),
            (
                "context",
                hash_with(&base, |m| {
                    m.context = Some(serde_json::json!({ "purpose": "research" }));
                }),
            ),
            ("disclosure", hash_with(&base, |m| m.disclosure = true)),
        ];
        for (field, digest) in variants {
            assert_ne!(digest, baseline, "changing {field} must change the digest");
        }
        // …and so does the predecessor.
        assert_ne!(row_hash(&input_for(&base, Some("other"))), baseline);
    }

    /// A well-formed chain verifies.
    #[test]
    fn intact_chain_verifies() {
        let rows = chain(5);
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.intact, 5);
        assert_eq!(report.rows, 5);
        assert_eq!(report.head, rows[4].hash);
    }

    /// Editing a row's content breaks verification at that row.
    #[test]
    fn edited_row_is_a_content_break() {
        let mut rows = chain(3);
        rows[1].snapshot = Some(serde_json::json!({ "name": "tampered" }));
        let report = verify(&rows);
        assert!(!report.verified);
        assert_eq!(report.breaks.len(), 1);
        assert_eq!(report.breaks[0].kind, "content");
        assert_eq!(report.breaks[0].id, 2);
    }

    /// Deleting a row breaks the linkage of its successor — the property an
    /// append-only convention alone cannot provide.
    #[test]
    fn deleted_row_is_a_linkage_break() {
        let mut rows = chain(4);
        rows.remove(1);
        let report = verify(&rows);
        assert!(!report.verified);
        assert!(report.breaks.iter().any(|b| b.kind == "linkage"));
    }

    /// Reordering rows breaks linkage.
    #[test]
    fn reordered_rows_are_a_linkage_break() {
        let mut rows = chain(3);
        rows.swap(1, 2);
        assert!(!verify(&rows).verified);
    }

    /// A redacted row still verifies: its content check is skipped (the
    /// data is gone under Art. 17) but its linkage is checked, so the chain
    /// as a whole remains sound and still proves the event happened.
    #[test]
    fn redacted_row_keeps_the_chain_verifiable() {
        let mut rows = chain(3);
        rows[1].snapshot = None;
        rows[1].redacted_at = Some(at(1_800_000_000_000_000));
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.redacted, 1);
        assert_eq!(report.intact, 2);
    }

    /// Redacting a row must not let an attacker also detach its successor:
    /// linkage is still enforced across the redacted row.
    #[test]
    fn redaction_does_not_disable_linkage_checking() {
        let mut rows = chain(3);
        rows[1].redacted_at = Some(at(1_800_000_000_000_000));
        rows[1].snapshot = None;
        rows[2].prev_hash = Some("forged".to_string());
        let report = verify(&rows);
        assert!(!report.verified);
        assert_eq!(report.breaks[0].kind, "linkage");
        assert_eq!(report.breaks[0].id, 3);
    }

    /// Rows written before the chain existed are `unchained`, not breaks,
    /// and they clear the expectation for the row that follows — so
    /// adopting the chain on a populated table verifies cleanly.
    #[test]
    fn pre_chain_rows_are_unchained_not_broken() {
        let mut rows = chain(3);
        rows[0].hash = None;
        rows[0].prev_hash = None;
        rows[1].prev_hash = None; // the first chained row after the gap
        rows[1].hash = Some(row_hash(&input_for(&rows[1], None)));
        let prev = rows[1].hash.clone();
        rows[2].prev_hash = prev.clone();
        rows[2].hash = Some(row_hash(&input_for(&rows[2], prev.as_deref())));
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.unchained, 1);
        assert_eq!(report.intact, 2);
    }

    /// An empty trail verifies vacuously and reports no head.
    #[test]
    fn empty_run_verifies() {
        let report = verify(&[]);
        assert!(report.verified);
        assert_eq!(report.rows, 0);
        assert!(report.head.is_none());
    }
}
