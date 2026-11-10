//! `instance_steps` model — one instance's step-completion checklist.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One checklist step. Serialized as-is by the instance endpoints, in
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
    /// The step's label.
    pub label: String,
    /// Whether the step is done.
    pub done: bool,
    /// The day it was done.
    pub done_on: Option<NaiveDate>,
    /// Declared order within the instance.
    pub position: i32,
}

impl Model {
    /// Insert one step, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(
        db: impl Into<Db<'_>>,
        instance_pid: Uuid,
        label: &str,
        done: bool,
        done_on: Option<NaiveDate>,
        position: i32,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO instance_steps (pid, instance_pid, label, done, done_on, position)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, label, done, done_on, position"#,
            Uuid::new_v4(),
            instance_pid,
            label,
            done,
            done_on,
            position,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// One step by `pid`, or `None`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_pid(db: &PgPool, pid: Uuid) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, done, done_on, position
               FROM instance_steps
               WHERE pid = $1
               LIMIT 1"#,
            pid,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// One instance's steps, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance(db: &PgPool, instance_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, done, done_on, position
               FROM instance_steps
               WHERE instance_pid = $1"#,
            instance_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// One instance's steps, in position order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance_by_position(
        db: &PgPool,
        instance_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, done, done_on, position
               FROM instance_steps
               WHERE instance_pid = $1
               ORDER BY position ASC"#,
            instance_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every step of the given instances, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances(db: &PgPool, instance_pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, done, done_on, position
               FROM instance_steps
               WHERE instance_pid = ANY($1)"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every step of the given instances, in position order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances_by_position(
        db: &PgPool,
        instance_pids: &[Uuid],
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, label, done, done_on, position
               FROM instance_steps
               WHERE instance_pid = ANY($1)
               ORDER BY position ASC"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Mark the step done on `done_on`.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is
    /// gone.
    pub async fn complete(self, db: &PgPool, done_on: NaiveDate) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE instance_steps SET done = true, done_on = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, label, done, done_on, position"#,
            self.id,
            done_on,
        )
        .fetch_optional(db)
        .await?;
        row.ok_or(ModelError::Sqlx(sqlx::Error::RowNotFound))
    }
}
