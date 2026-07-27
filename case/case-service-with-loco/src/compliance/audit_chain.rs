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
//! prove the `cases` rows were not; row-level integrity hashing
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
    /// The case `pid` the entry concerns.
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

/// The same digest under **BLAKE3**, over the byte-identical pre-image.
///
/// For the chain, callers pass the **BLAKE3** predecessor in
/// `prev_hash`, so the two chains link independently: neither depends on
/// the other's collision resistance.
///
/// See `spec/12-compliance.md` §12.4z for why both algorithms are kept:
/// SHA-256 for conservatism and auditor familiarity, BLAKE3 for speed and
/// for the agility to survive a future weakness in either.
///
#[must_use]
pub fn row_hash_blake3(input: &ChainInput<'_>) -> String {
    let pre = preimage(input);
    blake3::hash(&pre).to_hex().to_string()
}

/// The digest pre-image: the version tag, then every bound field, each
/// followed by the unit separator.
///
/// Built once and hashed by each algorithm, so adding an algorithm cannot
/// change *what* is covered — only how it is digested.
fn preimage(input: &ChainInput<'_>) -> Vec<u8> {
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
pub fn verify(rows: &[audit_logs::Model]) -> ChainReport {
    let mut report = ChainReport {
        rows: rows.len(),
        intact: 0,
        redacted: 0,
        unchained: 0,
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
            let recomputed = row_hash(&input_for(row, row.prev_hash.as_deref()));
            if recomputed == stored {
                report.intact += 1;
            } else {
                report.breaks.push(ChainBreak {
                    id: row.id,
                    kind: "content",
                    detail: "row content does not match its stored hash".to_string(),
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
    /// **Golden vector.** A fixed input hashes to this exact digest.
    ///
    /// Every other test recomputes with the same code, so they stay green
    /// even if the pre-image changes — and a changed pre-image silently
    /// invalidates every digest already stored, which is
    /// indistinguishable from mass tampering. This constant, cross-checked
    /// against an independent implementation of the format documented in
    /// `spec/12-compliance.md` §12.4z, is the only thing standing between
    /// a refactor and that outcome.
    ///
    /// If this fails, the hash format changed: bump the version tag
    /// deliberately and plan the migration — do not update the constant.
    #[test]
    fn golden_vector_pins_the_wire_format() {
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

    /// Build a chained row the way the writer does: hash over the row's own
    /// content plus `prev`.
    fn row(
        id: i32,
        prev: Option<&str>,
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
            context: None,
            disclosure: false,
            redacted_at: None,
        };
        model.hash = Some(row_hash(&input_for(&model, prev)));
        model
    }

    /// Build a well-formed chain of `n` rows.
    fn chain(n: i32) -> Vec<audit_logs::Model> {
        let mut rows: Vec<audit_logs::Model> = Vec::new();
        for id in 1..=n {
            let prev = rows.last().and_then(|r| r.hash.clone());
            rows.push(row(
                id,
                prev.as_deref(),
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
