//! `sites` — a hospital site within the trust. `place_ref` /
//! `organization_ref` are `EntityRef` URNs, never copies.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One `sites` row. Serialized as-is by `GET /api/sites`, so the field
/// order and the `DateTime<FixedOffset>` timestamps are the wire shape
/// the `SeaORM` entity had.
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub name: String,
    pub place_ref: Option<String>,
    pub organization_ref: Option<String>,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new site; `id`, `created_at` and `updated_at` are
/// column defaults, and a new site is never deleted.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewSite<'a> {
    pub pid: Uuid,
    pub name: &'a str,
    pub place_ref: Option<&'a str>,
    pub organization_ref: Option<&'a str>,
}

impl Model {
    /// Insert a site, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewSite<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO sites (pid, name, place_ref, organization_ref, deleted_at)
               VALUES ($1, $2, $3, $4, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, name, place_ref, organization_ref,
                         deleted_at AS "deleted_at: _""#,
            new.pid,
            new.name,
            new.place_ref,
            new.organization_ref,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active (not soft-deleted) site with this `pid`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when absent or soft-deleted.
    pub async fn find_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, place_ref, organization_ref,
                      deleted_at AS "deleted_at: _"
               FROM sites
               WHERE pid = $1 AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(ModelError::EntityNotFound)
    }

    /// The site with this `pid`, soft-deleted or not (the locate read
    /// names the site of a ward even after the site is deleted).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_any(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, place_ref, organization_ref,
                      deleted_at AS "deleted_at: _"
               FROM sites
               WHERE pid = $1"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Active sites in id order, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_active(db: impl Into<Db<'_>>, limit: u64) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, name, place_ref, organization_ref,
                      deleted_at AS "deleted_at: _"
               FROM sites
               WHERE deleted_at IS NULL
               ORDER BY id
               LIMIT $1"#,
            super::to_i64(limit),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
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
        let done = sqlx::query!("UPDATE sites SET deleted_at = $2 WHERE id = $1", id, at)
            .execute(&mut *conn)
            .await?;
        super::updated_one(done.rows_affected())
    }
}
