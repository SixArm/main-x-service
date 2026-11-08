//! Audit-log repository for the HIPAA-style change trail.
//!
//! [`AuditLogRepository`] appends immutable rows to the `audit_log`
//! table — one per CREATE / UPDATE / DELETE — capturing the actor
//! (`user_id`, `ip_address`, `user_agent`), the entity, and old/new
//! JSON snapshots. Read-side queries fetch the trail per entity, per
//! user, or system-wide, always newest-first.

use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::AuditLogRow;
use crate::Result;

/// Who/where/what context attached to each audited write.
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Acting user id (defaults to `"system"`).
    pub user_id: Option<String>,
    /// Originating IP address.
    pub ip_address: Option<String>,
    /// Originating user-agent string.
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    /// A `system`-attributed context with no IP / user-agent.
    fn default() -> Self {
        Self {
            user_id: Some("system".into()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// Audit log repository for recording changes
pub struct AuditLogRepository {
    /// The database connection/pool used for audit inserts and queries.
    db: PgPool,
}

impl AuditLogRepository {
    /// Create a new audit log repository wrapping the given connection
    /// pool.
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log a CREATE action.
    ///
    /// Records the post-create state only: `new_values` is the JSON
    /// snapshot of the freshly created entity, with no `old_values`.
    /// Delegates to `Self::log_action` with action `"CREATE"`.
    ///
    /// # Errors
    ///
    /// Propagates any database insert error from `Self::log_action`.
    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        context: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "CREATE",
            entity_type,
            entity_id,
            None,
            Some(new_values),
            context,
        )
        .await
    }

    /// Log an UPDATE action.
    ///
    /// Captures both before (`old_values`) and after (`new_values`)
    /// JSON snapshots so the diff is reconstructable from the trail.
    /// Delegates to `Self::log_action` with action `"UPDATE"`.
    ///
    /// # Errors
    ///
    /// Propagates any database insert error from `Self::log_action`.
    pub async fn log_update(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        context: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            context,
        )
        .await
    }

    /// Log a DELETE action.
    ///
    /// Records the pre-delete state only: `old_values` is the JSON
    /// snapshot of the entity as it was before the (soft) delete, with
    /// no `new_values`. Delegates to `Self::log_action` with action
    /// `"DELETE"`.
    ///
    /// # Errors
    ///
    /// Propagates any database insert error from `Self::log_action`.
    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        context: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            context,
        )
        .await
    }

    /// Log a generic action — the shared insert path behind
    /// [`Self::log_create`] / [`Self::log_update`] / [`Self::log_delete`].
    ///
    /// Mints a fresh row `id`, stamps the current UTC instant, and
    /// inserts one immutable `audit_log` row. The `old_values` /
    /// `new_values` `Option<JsonValue>`s are stored verbatim into the
    /// JSONB columns (a `None` becomes SQL `NULL`).
    ///
    /// # Errors
    ///
    /// Returns the database error if the row insert fails (converted
    /// into the crate's `Result` via the `?` operator's `From` impl).
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        context: &AuditContext,
    ) -> Result<()> {
        // Build the immutable audit row: a fresh UUID and a server-side
        // UTC timestamp. `old_values`/`new_values` map straight to the
        // JSONB columns.
        // The timestamp is part of the MAC pre-image, so it is minted here
        // and reused, rather than being generated twice or defaulted by
        // the database — the alternative (insert, then stamp) leaves a
        // window in which the row exists unprotected.
        let timestamp = chrono::Utc::now();
        // All three digests from one call, so none can be stamped
        // without the others. The two unkeyed ones are written
        // unconditionally; only the MAC depends on a configured key.
        let digests = crate::compliance::audit_integrity::digests(
            &crate::compliance::audit_integrity::AuditInput {
                entity_type,
                entity_id,
                action,
                user_id: context.user_id.as_deref(),
                ip_address: context.ip_address.as_deref(),
                user_agent: context.user_agent.as_deref(),
                old_values: old_values.as_ref(),
                new_values: new_values.as_ref(),
                created_at_micros: timestamp.timestamp_micros(),
            },
        );
        sqlx::query!(
            "INSERT INTO audit_log
                 (id, timestamp, user_id, action, entity_type, entity_id, old_values,
                  new_values, ip_address, user_agent, hash, hash_sha3, mac)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            Uuid::new_v4(),
            timestamp,
            context.user_id,
            action,
            entity_type,
            entity_id,
            old_values,
            new_values,
            context.ip_address,
            context.user_agent,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get up to `limit` audit logs for one specific entity, newest
    /// first (filtered by `entity_type` + `entity_id`, ordered by
    /// descending `timestamp`).
    ///
    /// # Errors
    ///
    /// Returns the database error if the query fails.
    pub async fn get_logs_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: u64,
    ) -> Result<Vec<AuditLogRow>> {
        let logs = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, hash, hash_sha3, mac
             FROM audit_log
             WHERE entity_type = $1 AND entity_id = $2
             ORDER BY timestamp DESC
             LIMIT $3",
            entity_type,
            entity_id,
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }

    /// Get the most recent `limit` audit logs system-wide, newest first
    /// (no entity/user filter; ordered by descending `timestamp`).
    ///
    /// # Errors
    ///
    /// Returns the database error if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<AuditLogRow>> {
        let logs = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, hash, hash_sha3, mac
             FROM audit_log
             ORDER BY timestamp DESC
             LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }

    /// Get up to `limit` audit logs attributed to one `user_id`, newest
    /// first (ordered by descending `timestamp`).
    ///
    /// # Errors
    ///
    /// Returns the database error if the query fails.
    pub async fn get_logs_by_user(&self, user_id: &str, limit: u64) -> Result<Vec<AuditLogRow>> {
        let logs = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, hash, hash_sha3, mac
             FROM audit_log
             WHERE user_id = $1
             ORDER BY timestamp DESC
             LIMIT $2",
            user_id,
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }

    /// The newest `limit` rows, whole (digests included), for
    /// `GET /api/audit/verify` to recompute.
    ///
    /// Newest is `timestamp` descending. `id` is a random UUID, so
    /// ordering by it alone picked an arbitrary sample and a tampered
    /// recent row could go unchecked (repo `tasks.md` LG-A1). `id` breaks
    /// ties between rows stamped in the same microsecond, so the sample
    /// is deterministic.
    ///
    /// # Errors
    ///
    /// Returns the database error if the query fails.
    pub async fn rows_for_verification(&self, limit: u64) -> Result<Vec<AuditLogRow>> {
        let rows = sqlx::query_as!(
            AuditLogRow,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, hash, hash_sha3, mac
             FROM audit_log
             ORDER BY timestamp DESC, id DESC
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
