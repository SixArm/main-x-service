//! `wards` — a ward within a site; `kind` is `inpatient` /
//! `assessment` / `virtual` (spec `domain-model.md`).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One `wards` row, serialized as-is by the ward endpoints; the field order
/// is a fixed wire shape.
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub name: String,
    pub code: String,
    pub kind: String,
    pub specialty: Option<String>,
    pub open: bool,
    pub escalation: bool,
    pub closed_to_admissions: bool,
    pub place_ref: Option<String>,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new ward.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewWard<'a> {
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub name: &'a str,
    pub code: &'a str,
    pub kind: &'a str,
    pub specialty: Option<&'a str>,
    pub open: bool,
    pub escalation: bool,
    pub closed_to_admissions: bool,
    pub place_ref: Option<&'a str>,
}

/// A partial ward update: `None` leaves the column as it
/// is.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default)]
pub struct WardChanges {
    pub name: Option<String>,
    pub specialty: Option<String>,
    pub open: Option<bool>,
    pub escalation: Option<bool>,
    pub closed_to_admissions: Option<bool>,
}

impl Model {
    /// Insert a ward, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewWard<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO wards (pid, site_pid, name, code, kind, specialty, open,
                                  escalation, closed_to_admissions, place_ref, deleted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, site_pid, name, code, kind, specialty, open,
                         escalation, closed_to_admissions, place_ref,
                         deleted_at AS "deleted_at: _""#,
            new.pid,
            new.site_pid,
            new.name,
            new.code,
            new.kind,
            new.specialty,
            new.open,
            new.escalation,
            new.closed_to_admissions,
            new.place_ref,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active (not soft-deleted) ward with this `pid`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when absent or soft-deleted.
    pub async fn find_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, site_pid, name, code, kind, specialty, open,
                      escalation, closed_to_admissions, place_ref,
                      deleted_at AS "deleted_at: _"
               FROM wards
               WHERE pid = $1 AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(ModelError::EntityNotFound)
    }

    /// Active wards in id order, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_active(db: impl Into<Db<'_>>, limit: u64) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, site_pid, name, code, kind, specialty, open,
                      escalation, closed_to_admissions, place_ref,
                      deleted_at AS "deleted_at: _"
               FROM wards
               WHERE deleted_at IS NULL
               ORDER BY id
               LIMIT $1"#,
            super::to_i64(limit),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Every active ward, in id order (the at-a-glance estate walk).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_active_ordered(db: impl Into<Db<'_>>) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, site_pid, name, code, kind, specialty, open,
                      escalation, closed_to_admissions, place_ref,
                      deleted_at AS "deleted_at: _"
               FROM wards
               WHERE deleted_at IS NULL
               ORDER BY id"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Every active ward, in no particular order (the allocator's
    /// estate walk, which matches by pid).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_active(db: impl Into<Db<'_>>) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, site_pid, name, code, kind, specialty, open,
                      escalation, closed_to_admissions, place_ref,
                      deleted_at AS "deleted_at: _"
               FROM wards
               WHERE deleted_at IS NULL"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Apply a partial update to the row with this `id`, returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn update(
        db: impl Into<Db<'_>>,
        id: i64,
        changes: &WardChanges,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE wards
               SET updated_at = now(), name = COALESCE($2, name),
                   specialty = COALESCE($3, specialty),
                   open = COALESCE($4, open),
                   escalation = COALESCE($5, escalation),
                   closed_to_admissions = COALESCE($6, closed_to_admissions)
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, site_pid, name, code, kind, specialty, open,
                         escalation, closed_to_admissions, place_ref,
                         deleted_at AS "deleted_at: _""#,
            id,
            changes.name,
            changes.specialty,
            changes.open,
            changes.escalation,
            changes.closed_to_admissions,
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
            "UPDATE wards SET updated_at = now(), deleted_at = $2 WHERE id = $1",
            id,
            at
        )
        .execute(&mut *conn)
        .await?;
        super::updated_one(done.rows_affected())
    }
}
