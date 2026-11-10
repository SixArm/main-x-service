//! `bed_requests` — the demand queue the allocator serves (spec
//! `domain-model.md`).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One `bed_requests` row, serialized as-is (flattened into the list
/// view) by the bed-request endpoints; the field order is a fixed
/// wire shape.
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub person_ref: String,
    pub origin: String,
    pub target_ward_pid: Option<Uuid>,
    pub specialty: Option<String>,
    pub priority: String,
    pub requirements: serde_json::Value,
    pub status: String,
    pub allocated_bed_pid: Option<Uuid>,
    pub requested_at: DateTime<FixedOffset>,
    pub resolved_at: Option<DateTime<FixedOffset>>,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new request; it starts `open`, unallocated and
/// unresolved.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewBedRequest<'a> {
    pub pid: Uuid,
    pub person_ref: &'a str,
    pub origin: &'a str,
    pub target_ward_pid: Option<Uuid>,
    pub specialty: Option<&'a str>,
    pub priority: &'a str,
    pub requirements: serde_json::Value,
    pub requested_at: DateTime<FixedOffset>,
}

impl Model {
    /// Insert a request, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewBedRequest<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO bed_requests (pid, person_ref, origin, target_ward_pid, specialty,
                                         priority, requirements, status, allocated_bed_pid,
                                         requested_at, resolved_at, deleted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', NULL, $8, NULL, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, origin, target_ward_pid, specialty, priority,
                         requirements, status, allocated_bed_pid,
                         requested_at AS "requested_at: _", resolved_at AS "resolved_at: _",
                         deleted_at AS "deleted_at: _""#,
            new.pid,
            new.person_ref,
            new.origin,
            new.target_ward_pid,
            new.specialty,
            new.priority,
            new.requirements,
            new.requested_at,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active (not soft-deleted) request with this `pid`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when absent or soft-deleted.
    pub async fn find_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, origin, target_ward_pid, specialty, priority,
                      requirements, status, allocated_bed_pid,
                      requested_at AS "requested_at: _", resolved_at AS "resolved_at: _",
                      deleted_at AS "deleted_at: _"
               FROM bed_requests
               WHERE pid = $1 AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(ModelError::EntityNotFound)
    }

    /// The active requests with this `status`, in id order (the demand
    /// board; the open board is then re-sorted by priority and wait).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn with_status(db: impl Into<Db<'_>>, status: &str) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, origin, target_ward_pid, specialty, priority,
                      requirements, status, allocated_bed_pid,
                      requested_at AS "requested_at: _", resolved_at AS "resolved_at: _",
                      deleted_at AS "deleted_at: _"
               FROM bed_requests
               WHERE deleted_at IS NULL AND status = $1
               ORDER BY id"#,
            status,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Every active open request, in no particular order (the
    /// at-a-glance demand count).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_open(db: impl Into<Db<'_>>) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, origin, target_ward_pid, specialty, priority,
                      requirements, status, allocated_bed_pid,
                      requested_at AS "requested_at: _", resolved_at AS "resolved_at: _",
                      deleted_at AS "deleted_at: _"
               FROM bed_requests
               WHERE deleted_at IS NULL AND status = 'open'"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Reserve `bed_pid` for the request: status `allocated`, returning
    /// the row.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn allocate(db: impl Into<Db<'_>>, id: i64, bed_pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bed_requests SET updated_at = now(), status = 'allocated', allocated_bed_pid = $2
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, origin, target_ward_pid, specialty, priority,
                         requirements, status, allocated_bed_pid,
                         requested_at AS "requested_at: _", resolved_at AS "resolved_at: _",
                         deleted_at AS "deleted_at: _""#,
            id,
            bed_pid,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Resolve the request (`fulfilled` or `cancelled`) at `at`,
    /// returning the row.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn resolve(
        db: impl Into<Db<'_>>,
        id: i64,
        status: &str,
        at: DateTime<FixedOffset>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bed_requests SET updated_at = now(), status = $2, resolved_at = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, origin, target_ward_pid, specialty, priority,
                         requirements, status, allocated_bed_pid,
                         requested_at AS "requested_at: _", resolved_at AS "resolved_at: _",
                         deleted_at AS "deleted_at: _""#,
            id,
            status,
            at,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }
}
