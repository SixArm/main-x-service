//! Audit-log repository for the HIPAA-style change trail.
//!
//! [`AuditLogRepository`] appends immutable rows to the `audit_log`
//! table — one per CREATE / UPDATE / DELETE — capturing the actor
//! (`user_id`, `ip_address`, `user_agent`), the entity, and old/new
//! JSON snapshots. Read-side queries fetch the trail per entity, per
//! user, or system-wide, always newest-first.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
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
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Create a new audit log repository wrapping the given pooled
    /// `SeaORM` [`DatabaseConnection`].
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Log a CREATE action.
    ///
    /// Records the post-create state only: `new_values` is the JSON
    /// snapshot of the freshly created entity, with no `old_values`.
    /// Delegates to [`Self::log_action`] with action `"CREATE"`.
    ///
    /// # Errors
    ///
    /// Propagates any database insert error from [`Self::log_action`].
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
    /// Delegates to [`Self::log_action`] with action `"UPDATE"`.
    ///
    /// # Errors
    ///
    /// Propagates any database insert error from [`Self::log_action`].
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
    /// no `new_values`. Delegates to [`Self::log_action`] with action
    /// `"DELETE"`.
    ///
    /// # Errors
    ///
    /// Propagates any database insert error from [`Self::log_action`].
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
    /// Returns the `SeaORM` error if the row insert fails (converted
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
        // UTC timestamp from the `time` crate (the column type is
        // `OffsetDateTime`). `old_values`/`new_values` map straight to
        // the JSONB columns.
        // The timestamp is part of the MAC pre-image, so it is minted here
        // and reused, rather than being generated twice or defaulted by
        // the database — the alternative (insert, then stamp) leaves a
        // window in which the row exists unprotected.
        let timestamp = time::OffsetDateTime::now_utc();
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
                created_at_micros: i64::try_from(timestamp.unix_timestamp_nanos() / 1_000)
                    .unwrap_or(0),
            },
        );
        let new_audit = audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            timestamp: Set(timestamp),
            user_id: Set(context.user_id.clone()),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            old_values: Set(old_values),
            new_values: Set(new_values),
            ip_address: Set(context.ip_address.clone()),
            user_agent: Set(context.user_agent.clone()),
            hash: Set(Some(digests.sha256)),
            hash_sha3: Set(Some(digests.sha3)),
            mac: Set(digests.mac),
        };

        new_audit.insert(&self.db).await?;

        Ok(())
    }

    /// Get up to `limit` audit logs for one specific entity, newest
    /// first (filtered by `entity_type` + `entity_id`, ordered by
    /// descending `timestamp`).
    ///
    /// # Errors
    ///
    /// Returns the `SeaORM` error if the query fails.
    pub async fn get_logs_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .filter(audit_log::Column::EntityType.eq(entity_type))
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Get the most recent `limit` audit logs system-wide, newest first
    /// (no entity/user filter; ordered by descending `timestamp`).
    ///
    /// # Errors
    ///
    /// Returns the `SeaORM` error if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Get up to `limit` audit logs attributed to one `user_id`, newest
    /// first (ordered by descending `timestamp`).
    ///
    /// # Errors
    ///
    /// Returns the `SeaORM` error if the query fails.
    pub async fn get_logs_by_user(
        &self,
        user_id: &str,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .filter(audit_log::Column::UserId.eq(user_id))
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }
}
