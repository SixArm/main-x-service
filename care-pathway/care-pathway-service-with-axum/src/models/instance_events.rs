//! `instance_events` model — one instance's recorded events (reviews,
//! escalations, notes, referrals, …).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::ModelResult;

/// One recorded event. Serialized as-is by the instance endpoints, in
/// column order, a fixed wire shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp (column default).
    pub created_at: DateTime<FixedOffset>,
    /// Row update timestamp: the column default, never bumped.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal auto-increment primary key.
    pub id: i32,
    /// Public identifier.
    pub pid: Uuid,
    /// The owning instance's `pid`.
    pub instance_pid: Uuid,
    /// The event kind.
    pub kind: String,
    /// When it happened.
    pub occurred_at: DateTime<FixedOffset>,
    /// Free-text note.
    pub note: Option<String>,
    /// Who recorded it.
    pub actor: Option<String>,
}

impl Model {
    /// Insert one event, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(
        db: impl Into<Db<'_>>,
        instance_pid: Uuid,
        kind: &str,
        occurred_at: DateTime<FixedOffset>,
        note: Option<&str>,
        actor: Option<&str>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO instance_events (pid, instance_pid, kind, occurred_at, note, actor)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, kind,
                         occurred_at AS "occurred_at: _", note, actor"#,
            Uuid::new_v4(),
            instance_pid,
            kind,
            occurred_at,
            note,
            actor,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// One instance's events, newest first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance_newest_first(
        db: &PgPool,
        instance_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, kind,
                      occurred_at AS "occurred_at: _", note, actor
               FROM instance_events
               WHERE instance_pid = $1
               ORDER BY occurred_at DESC"#,
            instance_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// How many events of `kind` one instance has.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_of_kind(db: &PgPool, instance_pid: Uuid, kind: &str) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM instance_events
               WHERE instance_pid = $1 AND kind = $2"#,
            instance_pid,
            kind,
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Every event of the given instances, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances(db: &PgPool, instance_pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, kind,
                      occurred_at AS "occurred_at: _", note, actor
               FROM instance_events
               WHERE instance_pid = ANY($1)"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every event of the given instances, oldest first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances_oldest_first(
        db: &PgPool,
        instance_pids: &[Uuid],
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, kind,
                      occurred_at AS "occurred_at: _", note, actor
               FROM instance_events
               WHERE instance_pid = ANY($1)
               ORDER BY occurred_at ASC"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every event of `kind` for the given instances, in no particular
    /// order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances_of_kind(
        db: &PgPool,
        instance_pids: &[Uuid],
        kind: &str,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, kind,
                      occurred_at AS "occurred_at: _", note, actor
               FROM instance_events
               WHERE instance_pid = ANY($1) AND kind = $2"#,
            instance_pids,
            kind,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
