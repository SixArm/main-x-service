//! `audit_logs` — record and query the audit trail.
//!
//! Every mutation **and every sensitive read** (patient locate, stay
//! detail) writes one row. The `snapshot` JSON carries the action
//! detail (old/new state, override reasons, the `ward_pid` where
//! relevant — which is what the ward-scoped handover query filters on).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelResult;

/// One audit row: who did what to which record, with a snapshot.
/// Serialized as-is by the audit endpoints (the wire shape the `SeaORM`
/// entity had).
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub entity: String,
    pub entity_pid: Uuid,
    pub action: String,
    pub actor: Option<String>,
    pub snapshot: Option<serde_json::Value>,
}

impl Model {
    /// Record one audit entry. `actor` is the caller's `sub` (user
    /// `pid`) when a verified token was presented, else `None`.
    ///
    /// Takes a [`Db`] so it runs on the handler's transaction — the
    /// audit row commits **with** the mutation it records (family
    /// invariant 8), never separately.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: impl Into<Db<'_>>,
        entity: &str,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        // `id`, `created_at` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO audit_logs (entity, entity_pid, action, actor, snapshot)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, entity, entity_pid, action, actor, snapshot"#,
            entity,
            entity_pid,
            action,
            actor,
            snapshot,
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
    pub async fn recent(db: impl Into<Db<'_>>, limit: u64) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity, entity_pid, action, actor, snapshot
               FROM audit_logs
               ORDER BY id DESC
               LIMIT $1"#,
            super::to_i64(limit),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Audit entries for one record, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: impl Into<Db<'_>>, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity, entity_pid, action, actor, snapshot
               FROM audit_logs
               WHERE entity_pid = $1
               ORDER BY id DESC"#,
            entity_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// The ward-scoped handover query: entries since `since` whose
    /// snapshot names this `ward_pid`, most-recent first, capped at
    /// `limit`. The ward filter is applied over the snapshot JSON in
    /// Rust, over a candidate set already bounded by time and by ten
    /// times `limit`, exactly as before the move to sqlx.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_ward_since(
        db: impl Into<Db<'_>>,
        ward_pid: Uuid,
        since: DateTime<FixedOffset>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity, entity_pid, action, actor, snapshot
               FROM audit_logs
               WHERE created_at >= $1
               ORDER BY id DESC
               LIMIT $2"#,
            since,
            // Headroom before the in-app ward filter.
            super::to_i64(limit.saturating_mul(10)),
        )
        .fetch_all(&mut *conn)
        .await?;
        let ward = ward_pid.to_string();
        let rows = rows
            .into_iter()
            .filter(|row| {
                row.snapshot
                    .as_ref()
                    .and_then(|s| s.get("ward_pid"))
                    .and_then(serde_json::Value::as_str)
                    == Some(ward.as_str())
            })
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect();
        Ok(rows)
    }
}
