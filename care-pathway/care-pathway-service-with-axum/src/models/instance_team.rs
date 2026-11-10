//! `instance_team` model — one instance's care-team roster.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::ModelResult;

/// One (member, role) on an instance's team. Serialized as-is by the
/// instance endpoints, in
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
    /// The member, a `worker:`/`person:`/`organization:` URN.
    pub member_ref: String,
    /// The member's role on this instance.
    pub role: String,
}

impl Model {
    /// Insert one team member, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails (a repeated `(instance, member, role)`
    /// violates its unique constraint).
    pub async fn insert(
        db: impl Into<Db<'_>>,
        instance_pid: Uuid,
        member_ref: &str,
        role: &str,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO instance_team (pid, instance_pid, member_ref, role)
               VALUES ($1, $2, $3, $4)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, member_ref, role"#,
            Uuid::new_v4(),
            instance_pid,
            member_ref,
            role,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The row holding `(instance, member, role)`, if any.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find(
        db: &PgPool,
        instance_pid: Uuid,
        member_ref: &str,
        role: &str,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, member_ref, role
               FROM instance_team
               WHERE instance_pid = $1 AND member_ref = $2 AND role = $3
               LIMIT 1"#,
            instance_pid,
            member_ref,
            role,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// One instance's team, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance(db: &PgPool, instance_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, member_ref, role
               FROM instance_team
               WHERE instance_pid = $1"#,
            instance_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// The teams of the given instances, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances(db: &PgPool, instance_pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, member_ref, role
               FROM instance_team
               WHERE instance_pid = ANY($1)"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every team row, in no particular order (the care-team load view).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all(db: &PgPool) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, member_ref, role
               FROM instance_team"#,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
