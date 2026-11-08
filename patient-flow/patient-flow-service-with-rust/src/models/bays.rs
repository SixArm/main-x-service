//! `bays` — a room/bay within a ward; side rooms are single-bed bays;
//! `sex_designation` feeds allocation rule 2.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One `bays` row, serialized as-is by `PUT /api/bays/{pid}` (the wire
/// shape the `SeaORM` entity had).
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub ward_pid: Uuid,
    pub name: String,
    pub sex_designation: String,
    pub side_room: bool,
    pub closed_to_admissions: bool,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new bay.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewBay<'a> {
    pub pid: Uuid,
    pub ward_pid: Uuid,
    pub name: &'a str,
    pub sex_designation: &'a str,
    pub side_room: bool,
    pub closed_to_admissions: bool,
}

impl Model {
    /// Insert a bay, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewBay<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO bays (pid, ward_pid, name, sex_designation, side_room,
                                 closed_to_admissions, deleted_at)
               VALUES ($1, $2, $3, $4, $5, $6, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, ward_pid, name, sex_designation, side_room,
                         closed_to_admissions, deleted_at AS "deleted_at: _""#,
            new.pid,
            new.ward_pid,
            new.name,
            new.sex_designation,
            new.side_room,
            new.closed_to_admissions,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active (not soft-deleted) bay with this `pid`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when absent or soft-deleted.
    pub async fn find_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, ward_pid, name, sex_designation, side_room,
                      closed_to_admissions, deleted_at AS "deleted_at: _"
               FROM bays
               WHERE pid = $1 AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(ModelError::EntityNotFound)
    }

    /// The active bays of one ward, in id order (the whiteboard).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn active_in_ward(db: impl Into<Db<'_>>, ward_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, ward_pid, name, sex_designation, side_room,
                      closed_to_admissions, deleted_at AS "deleted_at: _"
               FROM bays
               WHERE ward_pid = $1 AND deleted_at IS NULL
               ORDER BY id"#,
            ward_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Every active bay, in no particular order (the estate walks,
    /// which match by pid).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_active(db: impl Into<Db<'_>>) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, ward_pid, name, sex_designation, side_room,
                      closed_to_admissions, deleted_at AS "deleted_at: _"
               FROM bays
               WHERE deleted_at IS NULL"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Apply a partial update (`None` leaves the column as it is) to the
    /// row with this `id`, returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn update(
        db: impl Into<Db<'_>>,
        id: i64,
        sex_designation: Option<&str>,
        closed_to_admissions: Option<bool>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bays
               SET sex_designation = COALESCE($2, sex_designation),
                   closed_to_admissions = COALESCE($3, closed_to_admissions)
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, ward_pid, name, sex_designation, side_room,
                         closed_to_admissions, deleted_at AS "deleted_at: _""#,
            id,
            sex_designation,
            closed_to_admissions,
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
        let done = sqlx::query!("UPDATE bays SET deleted_at = $2 WHERE id = $1", id, at)
            .execute(&mut *conn)
            .await?;
        super::updated_one(done.rows_affected())
    }
}
