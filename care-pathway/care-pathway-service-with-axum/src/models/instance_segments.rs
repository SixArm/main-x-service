//! `instance_segments` model — one instance's recorded journey segments,
//! the time-based-analysis primitive (`spec/time-based-analysis.md` §5).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult, to_i64};

/// One journey segment. Serialized as-is by the segment endpoints, in
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
    /// The segment's label.
    pub label: String,
    /// The journey stage.
    pub stage: String,
    /// The value category.
    pub category: String,
    /// The waste type, for a non-value-adding segment.
    pub waste: Option<String>,
    /// When it started.
    pub started_at: DateTime<FixedOffset>,
    /// When it ended; `None` while running.
    pub ended_at: Option<DateTime<FixedOffset>>,
    /// Who did the work (a URN).
    pub actor_ref: Option<String>,
    /// Where (a URN).
    pub location_ref: Option<String>,
    /// Free-text note.
    pub note: Option<String>,
    /// Recording order within the instance.
    pub position: i32,
}

/// The columns a new segment is written with.
#[derive(Debug, Clone)]
pub struct NewSegment<'a> {
    /// The owning instance's `pid`.
    pub instance_pid: Uuid,
    /// The segment's label.
    pub label: &'a str,
    /// The journey stage.
    pub stage: &'a str,
    /// The value category.
    pub category: &'a str,
    /// The waste type.
    pub waste: Option<&'a str>,
    /// When it started.
    pub started_at: DateTime<FixedOffset>,
    /// When it ended.
    pub ended_at: Option<DateTime<FixedOffset>>,
    /// Who did the work.
    pub actor_ref: Option<&'a str>,
    /// Where.
    pub location_ref: Option<&'a str>,
    /// Free-text note.
    pub note: Option<&'a str>,
    /// Recording order within the instance.
    pub position: i32,
}

impl Model {
    /// Insert one segment, returning the row. The partial unique index
    /// `instance_segments_one_open` refuses a second open segment.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewSegment<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO instance_segments
                   (pid, instance_pid, label, stage, category, waste, started_at,
                    ended_at, actor_ref, location_ref, note, position)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, label, stage, category, waste,
                         started_at AS "started_at: _", ended_at AS "ended_at: _",
                         actor_ref, location_ref, note, position"#,
            Uuid::new_v4(),
            new.instance_pid,
            new.label,
            new.stage,
            new.category,
            new.waste,
            new.started_at,
            new.ended_at,
            new.actor_ref,
            new.location_ref,
            new.note,
            new.position,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// One instance's segments in time order (then recording order),
    /// capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance(
        db: &PgPool,
        instance_pid: Uuid,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, stage, category, waste,
                      started_at AS "started_at: _", ended_at AS "ended_at: _",
                      actor_ref, location_ref, note, position
               FROM instance_segments
               WHERE instance_pid = $1
               ORDER BY started_at ASC, position ASC
               LIMIT $2"#,
            instance_pid,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every segment of one instance, in no particular order (the DB
    /// tests' read-back).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance_any_order(db: &PgPool, instance_pid: Uuid) -> ModelResult<Vec<Self>> {
        Self::for_instances(db, &[instance_pid]).await
    }

    /// One instance's open (unended) segment, if any.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn open_for_instance(db: &PgPool, instance_pid: Uuid) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, stage, category, waste,
                      started_at AS "started_at: _", ended_at AS "ended_at: _",
                      actor_ref, location_ref, note, position
               FROM instance_segments
               WHERE instance_pid = $1 AND ended_at IS NULL
               LIMIT 1"#,
            instance_pid,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// How many segments one instance has.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_for_instance(db: &PgPool, instance_pid: Uuid) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM instance_segments WHERE instance_pid = $1"#,
            instance_pid,
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// One segment of one instance by `pid`, or `None`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_in_instance(
        db: &PgPool,
        pid: Uuid,
        instance_pid: Uuid,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, stage, category, waste,
                      started_at AS "started_at: _", ended_at AS "ended_at: _",
                      actor_ref, location_ref, note, position
               FROM instance_segments
               WHERE pid = $1 AND instance_pid = $2
               LIMIT 1"#,
            pid,
            instance_pid,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// Close the segment at `ended_at`.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is
    /// gone.
    pub async fn close(self, db: &PgPool, ended_at: DateTime<FixedOffset>) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE instance_segments SET ended_at = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, label, stage, category, waste,
                         started_at AS "started_at: _", ended_at AS "ended_at: _",
                         actor_ref, location_ref, note, position"#,
            self.id,
            ended_at,
        )
        .fetch_optional(db)
        .await?;
        row.ok_or(ModelError::Sqlx(sqlx::Error::RowNotFound))
    }

    /// Every segment of the given instances, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances(db: &PgPool, instance_pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, stage, category, waste,
                      started_at AS "started_at: _", ended_at AS "ended_at: _",
                      actor_ref, location_ref, note, position
               FROM instance_segments
               WHERE instance_pid = ANY($1)"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every segment of the given instances, in start order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances_by_start(
        db: &PgPool,
        instance_pids: &[Uuid],
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, stage, category, waste,
                      started_at AS "started_at: _", ended_at AS "ended_at: _",
                      actor_ref, location_ref, note, position
               FROM instance_segments
               WHERE instance_pid = ANY($1)
               ORDER BY started_at ASC"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
