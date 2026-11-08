//! `transfers` — the immutable record of each move (admission
//! placement, ward/bed transfer, discharge).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelResult;

/// One `transfers` row, serialized as-is in the stay detail (the wire
/// shape the `SeaORM` entity had).
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub stay_pid: Uuid,
    pub from_bed_pid: Option<Uuid>,
    pub to_bed_pid: Option<Uuid>,
    pub reason: String,
    pub moved_at: DateTime<FixedOffset>,
    pub moved_by_ref: Option<String>,
}

/// The columns of a new transfer.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewTransfer<'a> {
    pub pid: Uuid,
    pub stay_pid: Uuid,
    pub from_bed_pid: Option<Uuid>,
    pub to_bed_pid: Option<Uuid>,
    pub reason: &'a str,
    pub moved_at: DateTime<FixedOffset>,
    pub moved_by_ref: Option<&'a str>,
}

impl Model {
    /// Record one move.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewTransfer<'_>) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        sqlx::query!(
            r#"INSERT INTO transfers (pid, stay_pid, from_bed_pid, to_bed_pid, reason,
                                      moved_at, moved_by_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            new.pid,
            new.stay_pid,
            new.from_bed_pid,
            new.to_bed_pid,
            new.reason,
            new.moved_at,
            new.moved_by_ref,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// One stay's moves, in id order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_stay(db: impl Into<Db<'_>>, stay_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, stay_pid, from_bed_pid, to_bed_pid, reason,
                      moved_at AS "moved_at: _", moved_by_ref
               FROM transfers
               WHERE stay_pid = $1
               ORDER BY id"#,
            stay_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }
}
