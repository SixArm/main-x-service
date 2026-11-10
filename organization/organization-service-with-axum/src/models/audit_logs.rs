//! `audit_logs` model — record and query the CRUD audit trail.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::ModelResult;

/// One audit-trail row: one per CRUD action on an organization (who /
/// what / when + a snapshot).
///
/// Serialized as-is by the audit endpoints, so the field order and the
/// `DateTime<FixedOffset>` timestamps (`+00:00`) are a fixed
/// wire shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row insert time (UTC) — effectively the "when" of the action.
    pub created_at: DateTime<FixedOffset>,
    /// Row last-modification time (UTC); audit rows are append-only so
    /// this is the column default.
    pub updated_at: DateTime<FixedOffset>,
    /// Auto-increment surrogate primary key; also the recency order key.
    pub id: i64,
    /// The organization `pid` this entry concerns (the "what").
    pub entity_pid: Uuid,
    /// The action verb: `created` / `updated` / `deleted` / `merged` /
    /// `merged_into`.
    pub action: String,
    /// The actor that caused the change — the bearer token `sub` (user
    /// `pid`) when one was presented, else `None` (the "who").
    pub actor: Option<String>,
    /// Optional JSON snapshot of the record's payload at action time
    /// (`None` for deletes, where there is nothing to snapshot).
    pub snapshot: Option<serde_json::Value>,
    /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
    ///
    /// Unkeyed, so anyone holding the database can recompute it — what it
    /// catches is careless or unaware modification. It is written
    /// unconditionally, unlike the MAC, which needs a key: with no key
    /// configured these two digests are the row's only integrity.
    pub hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub hash_sha3: Option<String>,
    /// HMAC-SHA256 over this audit row's pre-image, as
    /// `"<scheme>.<key id>:<hex>"`.
    ///
    /// Detects an audit row whose *content* was edited. It does not
    /// detect a row deleted wholesale — that needs the hash chain this
    /// service does not yet have (see `crate::compliance`).
    pub mac: Option<String>,
}

/// A `LIMIT` bound: Postgres takes `BIGINT`, the callers count in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Recording and query helpers for the audit trail.
impl Model {
    /// Record one audit entry.
    ///
    /// - `entity_pid`: the organization the action concerns.
    /// - `action`: the verb (`created`/`updated`/`deleted`/`merged`/…).
    /// - `actor`: verified caller `sub`, or `None` when unauthenticated.
    /// - `snapshot`: the record's payload after the action (`None` for
    ///   deletes, where there is nothing to capture).
    ///
    /// Takes a [`Db`], so it runs either on the pool (the best-effort
    /// side-channel path, `memory` transport) **or** on the caller's
    /// transaction — under the `outbox` transport the audit row is written
    /// in the *same* transaction as the entity mutation and its
    /// `event_outbox` row, so the three can never disagree
    /// (`agents/share/event-bus.md` §3).
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: impl Into<Db<'_>>,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        // Set `created_at` explicitly rather than letting the database
        // default it: the timestamp is part of the MAC pre-image, so it
        // must be known before the insert. The alternative — insert, then
        // stamp — leaves a window in which the row exists unprotected.
        let created_at: DateTime<FixedOffset> = chrono::Utc::now().into();
        // All three digests from one call, so none can be stamped without
        // the others. The two unkeyed ones are written unconditionally;
        // only the MAC depends on a configured key.
        let digests = crate::compliance::audit_integrity::digests(
            &crate::compliance::audit_integrity::AuditInput {
                entity_pid,
                action,
                actor,
                snapshot: snapshot.as_ref(),
                created_at_micros: created_at.timestamp_micros(),
            },
        );
        let mut conn = db.into().acquire().await?;
        // `id` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO audit_logs
                   (entity_pid, action, actor, snapshot, created_at, hash, hash_sha3, mac)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, entity_pid, action, actor, snapshot, hash, hash_sha3, mac"#,
            entity_pid,
            action,
            actor,
            snapshot,
            created_at,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Most-recent audit entries, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot, hash, hash_sha3, mac
               FROM audit_logs
               ORDER BY id DESC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Audit entries for one organization, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: &PgPool, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot, hash, hash_sha3, mac
               FROM audit_logs
               WHERE entity_pid = $1
               ORDER BY id DESC"#,
            entity_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
