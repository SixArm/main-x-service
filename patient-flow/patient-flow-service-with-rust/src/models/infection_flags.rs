//! `infection_flags` — per-stay IPC precaution flags (spec
//! `infection-control.md`).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelResult;

/// One `infection_flags` row, serialized as-is in the stay detail; the field
/// order is a fixed wire shape.
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub stay_pid: Uuid,
    pub precaution: String,
    pub organism: Option<String>,
    pub status: String,
    pub requires_side_room: bool,
    pub flagged_at: DateTime<FixedOffset>,
    pub cleared_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new flag; it starts uncleared.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewFlag<'a> {
    pub pid: Uuid,
    pub stay_pid: Uuid,
    pub precaution: &'a str,
    pub organism: Option<&'a str>,
    pub status: &'a str,
    pub requires_side_room: bool,
    pub flagged_at: DateTime<FixedOffset>,
}

impl Model {
    /// Raise a flag, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewFlag<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO infection_flags (pid, stay_pid, precaution, organism, status,
                                            requires_side_room, flagged_at, cleared_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, stay_pid, precaution, organism, status,
                         requires_side_room, flagged_at AS "flagged_at: _",
                         cleared_at AS "cleared_at: _""#,
            new.pid,
            new.stay_pid,
            new.precaution,
            new.organism,
            new.status,
            new.requires_side_room,
            new.flagged_at,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// One stay's flags, cleared or not, in id order (the stay detail).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_stay(db: impl Into<Db<'_>>, stay_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, stay_pid, precaution, organism, status,
                      requires_side_room, flagged_at AS "flagged_at: _",
                      cleared_at AS "cleared_at: _"
               FROM infection_flags
               WHERE stay_pid = $1
               ORDER BY id"#,
            stay_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// One stay's uncleared flags.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn open_for_stay(db: impl Into<Db<'_>>, stay_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, stay_pid, precaution, organism, status,
                      requires_side_room, flagged_at AS "flagged_at: _",
                      cleared_at AS "cleared_at: _"
               FROM infection_flags
               WHERE stay_pid = $1 AND cleared_at IS NULL"#,
            stay_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// The uncleared flags of any of these stays (the whiteboard).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn open_for_stays(
        db: impl Into<Db<'_>>,
        stay_pids: &[Uuid],
    ) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, stay_pid, precaution, organism, status,
                      requires_side_room, flagged_at AS "flagged_at: _",
                      cleared_at AS "cleared_at: _"
               FROM infection_flags
               WHERE stay_pid = ANY($1) AND cleared_at IS NULL"#,
            stay_pids,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// The flag with this `pid` on this stay, cleared or not.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_on_stay(
        db: impl Into<Db<'_>>,
        pid: Uuid,
        stay_pid: Uuid,
    ) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, stay_pid, precaution, organism, status,
                      requires_side_room, flagged_at AS "flagged_at: _",
                      cleared_at AS "cleared_at: _"
               FROM infection_flags
               WHERE pid = $1 AND stay_pid = $2
               LIMIT 1"#,
            pid,
            stay_pid,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Clear the flag with this `id` at `at`.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn clear(
        db: impl Into<Db<'_>>,
        id: i64,
        at: DateTime<FixedOffset>,
    ) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        let done = sqlx::query!(
            "UPDATE infection_flags SET updated_at = now(), status = 'cleared', cleared_at = $2 WHERE id = $1",
            id,
            at,
        )
        .execute(&mut *conn)
        .await?;
        super::updated_one(done.rows_affected())
    }
}
