//! `merge_records` model — record and query the record-merge history.

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::ModelResult;

/// One merge-history row: which duplicate folded into which survivor,
/// with a snapshot of the transferred payload. One row is written per
/// successful merge for auditability.
///
/// Serialized as-is by `GET /api/organizations/merges/recent`, so the
/// field order and the `DateTime<FixedOffset>` timestamps are the wire
/// shape the `SeaORM` entity had.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row insert time (UTC) — when the merge happened.
    pub created_at: DateTime<FixedOffset>,
    /// Row last-modification time (UTC); merge rows are append-only.
    pub updated_at: DateTime<FixedOffset>,
    /// Auto-increment surrogate primary key; also the recency order key.
    pub id: i64,
    /// The surviving (main) organization's `pid` — the record kept.
    pub main_pid: Uuid,
    /// The merged-away duplicate's `pid` — now soft-deleted.
    pub duplicate_pid: Uuid,
    /// Optional operator-supplied free-text reason for the merge.
    pub reason: Option<String>,
    /// The actor that performed the merge (bearer `sub`), else `None`.
    pub actor: Option<String>,
    /// JSON snapshot of the duplicate's payload at merge time, so the
    /// merged-away data is recoverable from the history.
    pub transferred: Option<serde_json::Value>,
}

/// Recording and query helpers for the merge history.
impl Model {
    /// Record one merge. `transferred` is a snapshot of the merged-away
    /// duplicate's payload. `actor` is the caller's `sub` when a verified
    /// token was presented, else `None`.
    ///
    /// - `main_pid`: the surviving organization (kept).
    /// - `duplicate_pid`: the merged-away organization (now soft-deleted).
    /// - `reason`: optional operator-supplied free text.
    /// - `actor`: verified caller `sub`, or `None` when unauthenticated.
    /// - `transferred`: the duplicate's payload snapshot for recovery.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: &PgPool,
        main_pid: Uuid,
        duplicate_pid: Uuid,
        reason: Option<&str>,
        actor: Option<&str>,
        transferred: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        // `id`, `created_at` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO merge_records (main_pid, duplicate_pid, reason, actor, transferred)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, main_pid, duplicate_pid, reason, actor, transferred"#,
            main_pid,
            duplicate_pid,
            reason,
            actor,
            transferred,
        )
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    /// Most-recent merge records, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, main_pid, duplicate_pid, reason, actor, transferred
               FROM merge_records
               ORDER BY id DESC
               LIMIT $1"#,
            i64::try_from(limit).unwrap_or(i64::MAX),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
