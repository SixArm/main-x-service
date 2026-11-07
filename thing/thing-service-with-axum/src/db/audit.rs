//! Audit-log repository — writes to the `audit_log` table and reads back
//! per-entity / recent histories.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use uuid::Uuid;

use sqlx::PgPool;

use super::models::AuditLogRow;
use crate::Result;

/// Public view of one audit-log row.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditEntry {
    /// Audit-row id.
    pub id: Uuid,
    /// Entity kind (e.g. `"thing"`).
    pub entity_type: String,
    /// Affected entity's id.
    pub entity_id: Uuid,
    /// Operation: `CREATE`, `UPDATE`, or `DELETE`.
    pub action: String,
    /// Acting user id, if known.
    pub user_id: Option<String>,
    /// Originating IP address, if known.
    pub user_ip_address: Option<String>,
    /// Originating user-agent string, if known.
    pub user_agent: Option<String>,
    /// Pre-change snapshot (absent for creates).
    pub old_values: Option<JsonValue>,
    /// Post-change snapshot (absent for deletes).
    pub new_values: Option<JsonValue>,
    /// When the action occurred.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditLogRow> for AuditEntry {
    /// Project a persisted `audit_log` row into the public view.
    fn from(m: AuditLogRow) -> Self {
        Self {
            id: m.id,
            entity_type: m.entity_type,
            entity_id: m.entity_id,
            action: m.action,
            user_id: m.user_id,
            user_ip_address: m.user_ip_address,
            user_agent: m.user_agent,
            old_values: m.old_values,
            new_values: m.new_values,
            created_at: m.created_at,
        }
    }
}

/// Request-scoped actor metadata threaded into each audit write.
#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    /// Acting user id, if authenticated.
    pub user_id: Option<String>,
    /// Originating IP address.
    pub ip_address: Option<String>,
    /// Originating user-agent string.
    pub user_agent: Option<String>,
}

/// Repository for writing and querying the `audit_log` table.
pub struct AuditLogRepository {
    /// Shared connection pool.
    db: PgPool,
}

impl AuditLogRepository {
    /// Wrap an existing connection pool.
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Record a `CREATE` with only post-change values.
    ///
    /// # Errors
    ///
    /// Returns an error if the audit row cannot be inserted.
    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "CREATE",
            entity_type,
            entity_id,
            None,
            Some(new_values),
            ctx,
        )
        .await
    }

    /// Record an `UPDATE` with both pre- and post-change values.
    ///
    /// # Errors
    ///
    /// Returns an error if the audit row cannot be inserted.
    pub async fn log_update(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            ctx,
        )
        .await
    }

    /// Record a `DELETE` with only pre-change values.
    ///
    /// # Errors
    ///
    /// Returns an error if the audit row cannot be inserted.
    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            ctx,
        )
        .await
    }

    /// Shared insert path for all three log_* convenience methods.
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
    ) -> Result<()> {
        let entity_type: String = entity_type.into();
        let action: String = action.into();
        // The timestamp is part of the MAC pre-image, so it is minted
        // here rather than defaulted by the database: the alternative —
        // insert, then stamp — leaves a window in which the row exists
        // unprotected.
        let created_at = chrono::Utc::now();
        // All three digests from one call, so none can be stamped
        // without the others. The two unkeyed ones are written
        // unconditionally; only the MAC depends on a configured key.
        let digests = crate::compliance::audit_integrity::digests(
            &crate::compliance::audit_integrity::AuditInput {
                entity_type: &entity_type,
                entity_id,
                action: &action,
                user_id: ctx.user_id.as_deref(),
                user_ip_address: ctx.ip_address.as_deref(),
                user_agent: ctx.user_agent.as_deref(),
                old_values: old_values.as_ref(),
                new_values: new_values.as_ref(),
                created_at_micros: created_at.timestamp_micros(),
            },
        );
        sqlx::query!(
            "INSERT INTO audit_log
                 (id, entity_type, entity_id, action, user_id, user_ip_address, user_agent,
                  old_values, new_values, created_at, hash, hash_sha3, mac)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            Uuid::new_v4(),
            entity_type,
            entity_id,
            action,
            ctx.user_id,
            ctx.ip_address,
            ctx.user_agent,
            old_values,
            new_values,
            created_at,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Entries for one entity, newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn list_for_entity(&self, entity_id: Uuid, limit: u64) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, entity_type, entity_id, action, user_id, user_ip_address, user_agent,
                    old_values, new_values, created_at, hash, hash_sha3, mac
             FROM audit_log
             WHERE entity_id = $1
             ORDER BY created_at DESC
             LIMIT $2",
            entity_id,
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        Ok(rows.into_iter().map(AuditEntry::from).collect())
    }

    /// System-wide tail, newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn list_recent(&self, limit: u64) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, entity_type, entity_id, action, user_id, user_ip_address, user_agent,
                    old_values, new_values, created_at, hash, hash_sha3, mac
             FROM audit_log
             ORDER BY created_at DESC
             LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        Ok(rows.into_iter().map(AuditEntry::from).collect())
    }

    /// The rows `GET /api/audit/verify` recomputes, whole (digests
    /// included), ordered by `id` descending as they always have been.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn rows_for_verification(&self, limit: u64) -> Result<Vec<AuditLogRow>> {
        let rows = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, entity_type, entity_id, action, user_id, user_ip_address, user_agent,
                    old_values, new_values, created_at, hash, hash_sha3, mac
             FROM audit_log
             ORDER BY id DESC
             LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        Ok(rows)
    }
}

/// A `LIMIT` bound: Postgres takes `BIGINT`, the callers count in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}
