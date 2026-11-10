//! `audit_log` model — record reads/writes touching governed
//! `subject_of` (case↔person) edges (spec §10.4 / design §10). The case
//! service audits access to the underlying record; the aggregator audits
//! access to the link, so the two trails are consistent.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelResult, to_i64};

/// A persisted governance-audit row. One row per read/write touching a
/// governed `subject_of` (case↔person) edge (spec §10.4 / design §10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row id (UUID v4, generated at record time).
    pub id: Uuid,
    /// The caller's bearer `sub`, or `None` for a bus-driven write.
    pub actor: Option<String>,
    /// `read_edge` | `read_single_view` | `apply_linked`.
    pub action: String,
    /// The governed edge's kind token.
    pub edge_kind: Option<String>,
    /// The governed edge's `from` endpoint URN.
    pub from_ref: Option<String>,
    /// The governed edge's `to` endpoint URN.
    pub to_ref: Option<String>,
    /// When the audited action occurred.
    pub occurred_at: DateTime<FixedOffset>,
    /// Caller IP (best-effort).
    pub user_ip: Option<String>,
    /// Caller `User-Agent` (best-effort).
    pub user_agent: Option<String>,
    /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
    ///
    /// Unkeyed, so anyone holding the database can recompute it — what it
    /// catches is careless or unaware modification. Written
    /// unconditionally, unlike the MAC, which needs a key: with no key
    /// configured these two digests are the row's only integrity.
    pub hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub hash_sha3: Option<String>,
    /// HMAC-SHA256 over this row's pre-image, as
    /// `"<scheme>.<key id>:<hex>"`.
    ///
    /// Detects a row whose content was altered. It does **not** detect a
    /// row deleted wholesale — that needs the hash chain this service
    /// does not have (see `crate::compliance`).
    pub mac: Option<String>,
}

/// The request context stamped on a governance-audit row: the caller
/// identity and best-effort network attribution. Borrowed, so a handler
/// can build it once per request and audit several edges.
#[derive(Debug, Default, Clone, Copy)]
pub struct AuditContext<'a> {
    /// The caller's verified bearer `sub`, if any.
    pub actor: Option<&'a str>,
    /// The client's address, from `axa::client::ClientAddr`. `None` when no
    /// request is behind the write, as with a bus-driven `apply_linked`.
    pub user_ip: Option<&'a str>,
    /// The caller `User-Agent` header, if present.
    pub user_agent: Option<&'a str>,
}

impl Model {
    /// Record one governance-audit row for `action` on the edge
    /// `from_ref --edge_kind--> to_ref`, stamping the caller context and
    /// the current time. A fresh UUID keys the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: impl Into<Db<'_>>,
        ctx: &AuditContext<'_>,
        action: &str,
        edge_kind: &str,
        from_ref: &str,
        to_ref: &str,
    ) -> ModelResult<()> {
        // The timestamp is part of the MAC pre-image, so it is minted here
        // and reused rather than generated twice — the alternative
        // (insert, then stamp) leaves a window in which the row exists
        // unprotected.
        let occurred_at = Utc::now().fixed_offset();
        let action = action.to_string();
        let edge_kind = edge_kind.to_string();
        let from_ref = from_ref.to_string();
        let to_ref = to_ref.to_string();
        // All three digests from one call, so none can be stamped
        // without the others. The two unkeyed ones are written
        // unconditionally; only the MAC depends on a configured key.
        let digests = crate::compliance::audit_integrity::digests(
            &crate::compliance::audit_integrity::AuditInput {
                actor: ctx.actor,
                action: &action,
                edge_kind: Some(&edge_kind),
                from_ref: Some(&from_ref),
                to_ref: Some(&to_ref),
                user_ip: ctx.user_ip,
                user_agent: ctx.user_agent,
                occurred_at_micros: occurred_at.timestamp_micros(),
            },
        );
        let mut conn = db.into().acquire().await?;
        sqlx::query!(
            "INSERT INTO audit_log
                 (id, actor, action, edge_kind, from_ref, to_ref, occurred_at,
                  user_ip, user_agent, hash, hash_sha3, mac)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            Uuid::new_v4(),
            ctx.actor,
            action,
            edge_kind,
            from_ref,
            to_ref,
            occurred_at,
            ctx.user_ip,
            ctx.user_agent,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// The newest `limit` rows, for `GET /api/compliance/audit/verify`.
    ///
    /// Newest is `occurred_at` descending: the table has no sequence
    /// column, and `id` is a random UUID, so ordering by `id` alone would
    /// pick an arbitrary sample (LG-A1). `id`
    /// breaks ties between rows stamped in the same microsecond, so the
    /// sample is deterministic. `audit_log_occurred` serves the sort.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT id, actor, action, edge_kind, from_ref, to_ref,
                      occurred_at AS "occurred_at: _", user_ip, user_agent,
                      hash, hash_sha3, mac
               FROM audit_log
               ORDER BY occurred_at DESC, id DESC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every audit row, in no particular order. The DB-gated tests read the
    /// trail with it.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all(db: &PgPool) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT id, actor, action, edge_kind, from_ref, to_ref,
                      occurred_at AS "occurred_at: _", user_ip, user_agent,
                      hash, hash_sha3, mac
               FROM audit_log"#,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
