//! `beds` — one physical (or virtual-slot) bed with its live
//! state-machine columns (spec `bed-management.md`).

#![allow(clippy::struct_excessive_bools)] // bed attribute columns are independent flags

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One `beds` row, serialized as-is by the bed endpoints. The `virtual`
/// column is `is_virtual` here and on the wire.
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub bay_pid: Uuid,
    pub number: String,
    pub state: String,
    pub state_since: DateTime<FixedOffset>,
    pub closure_reason: Option<String>,
    pub deep_clean_required: bool,
    pub isolation_capable: bool,
    pub oxygen: bool,
    pub bariatric: bool,
    pub is_virtual: bool,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new bed.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewBed<'a> {
    pub pid: Uuid,
    pub bay_pid: Uuid,
    pub number: &'a str,
    pub state: &'a str,
    pub state_since: DateTime<FixedOffset>,
    pub isolation_capable: bool,
    pub oxygen: bool,
    pub bariatric: bool,
    pub is_virtual: bool,
}

/// The state-machine columns one transition writes.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct StateChange<'a> {
    pub state: &'a str,
    pub state_since: DateTime<FixedOffset>,
    pub deep_clean_required: bool,
    pub closure_reason: Option<&'a str>,
}

impl Model {
    /// Insert a bed (no closure reason, no deep clean pending), returning
    /// the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewBed<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO beds (pid, bay_pid, number, state, state_since, closure_reason,
                                 deep_clean_required, isolation_capable, oxygen, bariatric,
                                 virtual, deleted_at)
               VALUES ($1, $2, $3, $4, $5, NULL, false, $6, $7, $8, $9, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, bay_pid, number, state, state_since AS "state_since: _",
                         closure_reason, deep_clean_required, isolation_capable, oxygen,
                         bariatric, virtual AS is_virtual, deleted_at AS "deleted_at: _""#,
            new.pid,
            new.bay_pid,
            new.number,
            new.state,
            new.state_since,
            new.isolation_capable,
            new.oxygen,
            new.bariatric,
            new.is_virtual,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active (not soft-deleted) bed with this `pid`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when absent or soft-deleted.
    pub async fn find_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, bay_pid, number, state, state_since AS "state_since: _",
                      closure_reason, deep_clean_required, isolation_capable, oxygen,
                      bariatric, virtual AS is_virtual, deleted_at AS "deleted_at: _"
               FROM beds
               WHERE pid = $1 AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(ModelError::EntityNotFound)
    }

    /// The active bed with this `pid`, locked `FOR UPDATE` for the
    /// read-check-write of a transition (PF-D9). Pass the transaction.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn lock_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, bay_pid, number, state, state_since AS "state_since: _",
                      closure_reason, deep_clean_required, isolation_capable, oxygen,
                      bariatric, virtual AS is_virtual, deleted_at AS "deleted_at: _"
               FROM beds
               WHERE pid = $1 AND deleted_at IS NULL
               FOR UPDATE"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active beds in these bays, in id order (the whiteboard).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn active_in_bays(
        db: impl Into<Db<'_>>,
        bay_pids: &[Uuid],
    ) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, bay_pid, number, state, state_since AS "state_since: _",
                      closure_reason, deep_clean_required, isolation_capable, oxygen,
                      bariatric, virtual AS is_virtual, deleted_at AS "deleted_at: _"
               FROM beds
               WHERE bay_pid = ANY($1) AND deleted_at IS NULL
               ORDER BY id"#,
            bay_pids,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Every active bed, in no particular order (the estate walks).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_active(db: impl Into<Db<'_>>) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, bay_pid, number, state, state_since AS "state_since: _",
                      closure_reason, deep_clean_required, isolation_capable, oxygen,
                      bariatric, virtual AS is_virtual, deleted_at AS "deleted_at: _"
               FROM beds
               WHERE deleted_at IS NULL"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Write one transition's state columns to the row with this `id`,
    /// returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn set_state(
        db: impl Into<Db<'_>>,
        id: i64,
        change: &StateChange<'_>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE beds
               SET updated_at = now(), state = $2, state_since = $3, deep_clean_required = $4,
                   closure_reason = $5
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, bay_pid, number, state, state_since AS "state_since: _",
                         closure_reason, deep_clean_required, isolation_capable, oxygen,
                         bariatric, virtual AS is_virtual, deleted_at AS "deleted_at: _""#,
            id,
            change.state,
            change.state_since,
            change.deep_clean_required,
            change.closure_reason,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Soft-delete the row with this `id`, stamping `deleted_at`.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn soft_delete(
        db: impl Into<Db<'_>>,
        id: i64,
        at: DateTime<FixedOffset>,
    ) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        let done = sqlx::query!(
            "UPDATE beds SET updated_at = now(), deleted_at = $2 WHERE id = $1",
            id,
            at
        )
        .execute(&mut *conn)
        .await?;
        super::updated_one(done.rows_affected())
    }
}
