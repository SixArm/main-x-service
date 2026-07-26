//! Tamper-evident audit history — a SHA-256 hash chain over `audit_log`.
//!
//! Adopted from the care-pathway reference implementation per
//! [`spec/compliance` §8.5](../../../../spec/compliance/index.md) step 3.
//! Worker is the identity spine of the family and its records are
//! personal — often special-category — data, so a silently editable audit
//! trail is the worst failure mode here.
//!
//! HIPAA §164.312(c)(1)–(2) requires protecting records from improper
//! alteration and providing a **mechanism to corroborate that data has not
//! been altered**. An append-only table is a convention; a hash chain is
//! evidence. Every audit row binds its own content **and its
//! predecessor's hash**, so inserting, deleting, reordering, or editing
//! any row breaks verification at that point and everywhere after it.
//!
//! ## Two differences from the reference implementation
//!
//! Worker's `audit_log` predates the loco-style services and is shaped
//! differently, so this is a port rather than a copy:
//!
//! - **Order comes from `seq`, not the primary key.** The PK is an
//!   application-assigned `Uuid`, which carries no insertion order, and a
//!   chain needs a total order to mean anything. `timestamp` alone is not
//!   enough — two rows can share a microsecond, and the tie-break would be
//!   arbitrary — so the migration adds a `BIGSERIAL`.
//! - **The row carries an old/new value pair**, plus request provenance
//!   (`ip_address`, `user_agent`). All of it is bound, so an attacker
//!   cannot rewrite *who* did something while leaving *what* intact.
//!
//! ## Reproducibility
//!
//! As in the reference: time is hashed as epoch **microseconds** (so the
//! session time zone and Postgres's microsecond precision cannot change
//! the digest — writers truncate before storing), and JSON is hashed as
//! `serde_json`'s serialization, whose `BTreeMap` key order matches what a
//! JSONB round-trip returns.

use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::models::audit_log;

/// Chain-format version, bound into every digest so a future change to the
/// hashed field set cannot be confused with tampering.
pub const CHAIN_VERSION: &str = "p1";

/// Field separator inside the digest pre-image: ASCII 31 (unit separator).
const SEP: char = '\u{1f}';

/// The fields one audit row binds into its chain hash.
#[derive(Debug, Clone, Copy)]
pub struct ChainInput<'a> {
    /// The preceding row's hash, or `None` for the genesis row.
    pub prev_hash: Option<&'a str>,
    /// The row's own id.
    pub id: Uuid,
    /// `timestamp` as microseconds since the Unix epoch (time-zone free).
    pub timestamp_micros: i64,
    /// The acting user id, if known.
    pub user_id: Option<&'a str>,
    /// The action verb.
    pub action: &'a str,
    /// The entity type acted on.
    pub entity_type: &'a str,
    /// The entity acted on.
    pub entity_id: Uuid,
    /// Pre-change snapshot, if any.
    pub old_values: Option<&'a serde_json::Value>,
    /// Post-change snapshot, if any.
    pub new_values: Option<&'a serde_json::Value>,
    /// Request IP, if captured.
    pub ip_address: Option<&'a str>,
    /// Request user-agent, if captured.
    pub user_agent: Option<&'a str>,
    /// Request/processing context, if any.
    pub context: Option<&'a serde_json::Value>,
    /// Whether the access was an outward disclosure.
    pub disclosure: bool,
}

/// Compute a row's chain hash as lowercase hex.
#[must_use]
pub fn row_hash(input: &ChainInput<'_>) -> String {
    let mut hasher = Sha256::new();
    let mut field = |value: &str| {
        hasher.update(value.as_bytes());
        hasher.update([SEP as u8]);
    };
    field(CHAIN_VERSION);
    field(input.prev_hash.unwrap_or(""));
    field(&input.id.to_string());
    field(&input.timestamp_micros.to_string());
    field(input.user_id.unwrap_or(""));
    field(input.action);
    field(input.entity_type);
    field(&input.entity_id.to_string());
    field(&canonical_json(input.old_values));
    field(&canonical_json(input.new_values));
    field(input.ip_address.unwrap_or(""));
    field(input.user_agent.unwrap_or(""));
    field(&canonical_json(input.context));
    field(if input.disclosure { "1" } else { "0" });
    to_hex(&hasher.finalize())
}

/// Canonical serialization of an optional JSON value: the empty string for
/// `None`, else `serde_json`'s output. A value that somehow fails to
/// serialize degrades to a sentinel rather than panicking; verification
/// then reports a content break, which is the correct conservative
/// outcome.
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

/// `time::OffsetDateTime` as epoch microseconds.
///
/// Postgres stores `timestamptz` at microsecond precision, so a writer
/// must truncate to microseconds before inserting or the value hashed on
/// the way in will not match the value read back.
#[must_use]
pub fn micros(ts: time::OffsetDateTime) -> i64 {
    // Nanoseconds since the epoch fits i128; microseconds fit i64 for any
    // date this system will see. Saturating rather than wrapping keeps an
    // absurd timestamp from silently aliasing onto a valid one.
    i64::try_from(ts.unix_timestamp_nanos() / 1_000).unwrap_or(i64::MAX)
}

/// Truncate an instant to whole microseconds, matching Postgres.
#[must_use]
pub fn trunc_micros(ts: time::OffsetDateTime) -> time::OffsetDateTime {
    ts.replace_nanosecond(ts.microsecond() * 1_000)
        .unwrap_or(ts)
}

/// Borrow a stored row's fields as a [`ChainInput`], using `prev` as the
/// predecessor hash.
#[must_use]
pub fn input_for<'a>(row: &'a audit_log::Model, prev: Option<&'a str>) -> ChainInput<'a> {
    ChainInput {
        prev_hash: prev,
        id: row.id,
        timestamp_micros: micros(row.timestamp),
        user_id: row.user_id.as_deref(),
        action: row.action.as_str(),
        entity_type: row.entity_type.as_str(),
        entity_id: row.entity_id,
        old_values: row.old_values.as_ref(),
        new_values: row.new_values.as_ref(),
        ip_address: row.ip_address.as_deref(),
        user_agent: row.user_agent.as_deref(),
        context: row.context.as_ref(),
        disclosure: row.disclosure,
    }
}

/// One detected break in the chain.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChainBreak {
    /// The `audit_log.seq` at which the break was detected.
    pub seq: i64,
    /// The row's id, so an operator can find it by primary key.
    pub id: String,
    /// `linkage` — the row does not point at its predecessor's hash (a row
    /// was inserted, deleted, or reordered); `content` — the row's stored
    /// hash does not match its own content (the row was edited).
    pub kind: &'static str,
    /// Human-readable detail.
    pub detail: String,
}

/// The outcome of verifying a contiguous, ascending-`seq` run of rows.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChainReport {
    /// Rows examined.
    pub rows: usize,
    /// Rows whose content hash was recomputed and matched.
    pub intact: usize,
    /// Rows redacted under GDPR Art. 17 — linkage checked, content skipped.
    pub redacted: usize,
    /// Rows written before the chain existed (no stored hash); neither
    /// verified nor counted as breaks, and they reset the linkage
    /// expectation for the row that follows.
    pub unchained: usize,
    /// Every break found, in `seq` order. Empty ⇒ the run verifies.
    pub breaks: Vec<ChainBreak>,
    /// The hash of the last chained row examined — the chain head an
    /// operator can record externally to detect wholesale truncation.
    pub head: Option<String>,
    /// `true` when no breaks were found.
    pub verified: bool,
}

/// Verify a run of audit rows supplied in **ascending `seq` order**.
#[must_use]
pub fn verify(rows: &[audit_log::Model]) -> ChainReport {
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
                seq: row.seq,
                id: row.id.to_string(),
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
        } else if row_hash(&input_for(row, row.prev_hash.as_deref())) == stored {
            report.intact += 1;
        } else {
            report.breaks.push(ChainBreak {
                seq: row.seq,
                id: row.id.to_string(),
                kind: "content",
                detail: "row content does not match its stored hash".to_string(),
            });
        }
        expected_prev = Some(Some(stored.to_string()));
        report.head = Some(stored.to_string());
    }
    report.verified = report.breaks.is_empty();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed instant so digests in these tests are stable.
    fn at(micros_since_epoch: i64) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(micros_since_epoch) * 1_000)
            .expect("valid instant")
    }

    /// Build a chained row the way the writer does.
    fn row(seq: i64, prev: Option<&str>, action: &str) -> audit_log::Model {
        let mut model = audit_log::Model {
            id: Uuid::from_u128(u128::try_from(seq).unwrap_or(0)),
            timestamp: at(1_700_000_000_000_000 + seq),
            user_id: Some("alice".to_string()),
            action: action.to_string(),
            entity_type: "person".to_string(),
            entity_id: Uuid::from_u128(999),
            old_values: None,
            new_values: Some(serde_json::json!({ "family_name": format!("Row{seq}") })),
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: Some("curl/8".to_string()),
            seq,
            prev_hash: prev.map(ToString::to_string),
            hash: None,
            context: None,
            disclosure: false,
            redacted_at: None,
        };
        model.hash = Some(row_hash(&input_for(&model, prev)));
        model
    }

    /// A well-formed chain of `n` rows.
    fn chain(n: i64) -> Vec<audit_log::Model> {
        let mut rows: Vec<audit_log::Model> = Vec::new();
        for seq in 1..=n {
            let prev = rows.last().and_then(|r| r.hash.clone());
            rows.push(row(seq, prev.as_deref(), "CREATE"));
        }
        rows
    }

    /// The digest is deterministic and SHA-256 shaped.
    #[test]
    fn row_hash_is_deterministic() {
        let rows = chain(1);
        assert_eq!(
            rows[0].hash.as_deref(),
            Some(row_hash(&input_for(&rows[0], None)).as_str())
        );
        assert_eq!(rows[0].hash.as_deref().map(str::len), Some(64));
    }

    /// Key order in a snapshot must not change the digest — the property
    /// that survives a JSONB round-trip.
    #[test]
    fn canonical_json_is_key_order_independent() {
        let a: serde_json::Value = serde_json::from_str(r#"{"b":2,"a":1}"#).expect("parse");
        let b: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).expect("parse");
        assert_eq!(canonical_json(Some(&a)), canonical_json(Some(&b)));
    }

    /// Every bound field changes the digest — including the request
    /// provenance, so an attacker cannot rewrite *who* acted while leaving
    /// *what* they did intact.
    #[test]
    fn row_hash_covers_every_bound_field() {
        let base = chain(1).remove(0);
        let baseline = base.hash.clone().expect("hashed");
        let mutate = |f: &dyn Fn(&mut audit_log::Model)| {
            let mut m = base.clone();
            f(&mut m);
            row_hash(&input_for(&m, None))
        };
        assert_ne!(mutate(&|m| m.action = "DELETE".into()), baseline, "action");
        assert_ne!(
            mutate(&|m| m.user_id = Some("mallory".into())),
            baseline,
            "user_id"
        );
        assert_ne!(mutate(&|m| m.timestamp = at(1)), baseline, "timestamp");
        assert_ne!(
            mutate(&|m| m.entity_type = "worker".into()),
            baseline,
            "entity_type"
        );
        assert_ne!(
            mutate(&|m| m.entity_id = Uuid::from_u128(7)),
            baseline,
            "entity_id"
        );
        assert_ne!(
            mutate(&|m| m.new_values = Some(serde_json::json!({ "family_name": "Tampered" }))),
            baseline,
            "new_values"
        );
        assert_ne!(
            mutate(&|m| m.old_values = Some(serde_json::json!({ "family_name": "Was" }))),
            baseline,
            "old_values"
        );
        assert_ne!(
            mutate(&|m| m.ip_address = Some("198.51.100.1".into())),
            baseline,
            "ip_address"
        );
        assert_ne!(
            mutate(&|m| m.user_agent = Some("evil/1".into())),
            baseline,
            "user_agent"
        );
        assert_ne!(mutate(&|m| m.disclosure = true), baseline, "disclosure");
        assert_ne!(
            row_hash(&input_for(&base, Some("other"))),
            baseline,
            "prev_hash"
        );
    }

    /// A well-formed chain verifies.
    #[test]
    fn intact_chain_verifies() {
        let rows = chain(5);
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.intact, 5);
        assert_eq!(report.head, rows[4].hash);
    }

    /// Editing a row's content breaks verification at that row.
    #[test]
    fn edited_row_is_a_content_break() {
        let mut rows = chain(3);
        rows[1].new_values = Some(serde_json::json!({ "family_name": "Tampered" }));
        let report = verify(&rows);
        assert!(!report.verified);
        assert_eq!(report.breaks.len(), 1);
        assert_eq!(report.breaks[0].kind, "content");
        assert_eq!(report.breaks[0].seq, 2);
    }

    /// Deleting a row breaks its successor's linkage — the property an
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

    /// A redacted row keeps the chain verifiable: content skipped (the
    /// data is gone under Art. 17), linkage still enforced.
    #[test]
    fn redacted_row_keeps_the_chain_verifiable() {
        let mut rows = chain(3);
        rows[1].new_values = None;
        rows[1].old_values = None;
        rows[1].redacted_at = Some(at(1_800_000_000_000_000));
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.redacted, 1);
        assert_eq!(report.intact, 2);
    }

    /// Redaction must not let an attacker detach the redacted row's
    /// successor.
    #[test]
    fn redaction_does_not_disable_linkage_checking() {
        let mut rows = chain(3);
        rows[1].redacted_at = Some(at(1_800_000_000_000_000));
        rows[1].new_values = None;
        rows[2].prev_hash = Some("forged".to_string());
        let report = verify(&rows);
        assert!(!report.verified);
        assert_eq!(report.breaks[0].kind, "linkage");
    }

    /// Rows written before the chain existed are `unchained`, not breaks,
    /// and clear the expectation for the row after — so adopting the chain
    /// on a populated table verifies cleanly.
    #[test]
    fn pre_chain_rows_are_unchained_not_broken() {
        let mut rows = chain(3);
        rows[0].hash = None;
        rows[0].prev_hash = None;
        rows[1].prev_hash = None;
        rows[1].hash = Some(row_hash(&input_for(&rows[1], None)));
        let prev = rows[1].hash.clone();
        rows[2].prev_hash = prev.clone();
        rows[2].hash = Some(row_hash(&input_for(&rows[2], prev.as_deref())));
        let report = verify(&rows);
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.unchained, 1);
        assert_eq!(report.intact, 2);
    }

    /// An empty trail verifies vacuously.
    #[test]
    fn empty_run_verifies() {
        let report = verify(&[]);
        assert!(report.verified);
        assert_eq!(report.rows, 0);
        assert!(report.head.is_none());
    }

    /// Microsecond truncation is idempotent and drops sub-microsecond
    /// precision — the property that makes a digest survive Postgres.
    #[test]
    fn truncation_matches_postgres_precision() {
        let ts = time::OffsetDateTime::from_unix_timestamp_nanos(1_700_000_000_123_456_789)
            .expect("valid");
        let truncated = trunc_micros(ts);
        assert_eq!(truncated.nanosecond() % 1_000, 0, "sub-microsecond dropped");
        assert_eq!(trunc_micros(truncated), truncated, "idempotent");
        assert_eq!(micros(truncated), 1_700_000_000_123_456);
    }
}
