//! HIPAA-style audit logging into the `audit_log` table.
//!
//! [`AuditLogRepository`] records who changed what and when, capturing
//! old/new values as JSON plus actor metadata (user ID, IP, user agent). The
//! `log_create` / `log_update` / `log_delete` helpers funnel into a single
//! private `log_action`, and the `get_*` queries back the audit REST
//! endpoints.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
use crate::Result;

/// Actor metadata recorded alongside an audit entry, borrowed for the duration
/// of the write. Groups the user/IP/user-agent triple so the `log_*` helpers
/// take a single argument rather than three.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuditActor<'a> {
    /// Acting user's identifier.
    pub user_id: Option<&'a str>,
    /// Originating client IP address.
    pub ip_address: Option<&'a str>,
    /// Originating client user-agent string.
    pub user_agent: Option<&'a str>,
}

/// Repository that writes and queries entries in the `audit_log` table.
pub struct AuditLogRepository {
    /// `SeaORM` connection used for all audit reads and writes.
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Wraps an existing database connection in an audit repository.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Records a `CREATE` action, storing `new_values` (no prior state).
    ///
    /// The `old_values` column is left null because a created record has no
    /// before-state; `new_values` is the JSON snapshot of the new record.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails (e.g. a
    /// database connectivity or constraint error).
    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action(
            "CREATE",
            entity_type,
            entity_id,
            None,
            Some(new_values),
            actor,
        )
        .await
    }

    /// Records an `UPDATE` action, storing both `old_values` and `new_values`
    /// so the entry captures the full before/after diff.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails.
    pub async fn log_update(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action(
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            actor,
        )
        .await
    }

    /// Records a `DELETE` action, storing `old_values` (no new state).
    ///
    /// The `new_values` column is left null because a deleted record has no
    /// after-state; `old_values` preserves the record as it was before
    /// (soft-)deletion.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails.
    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action(
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            actor,
        )
        .await
    }

    /// Records an `EXPORT` action, storing the export `details` (actor,
    /// filter, format, masking profile, row count) as the new-values
    /// snapshot, with no prior snapshot. Used for the bulk export/read
    /// compliance trail (`bulk-import-export.md` §8, cross-service-linking
    /// §8) — a bulk extract of personal data is itself an audited event.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails.
    pub async fn log_export(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        details: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action("EXPORT", entity_type, entity_id, None, Some(details), actor)
            .await
    }

    /// Inserts one audit row. Shared implementation behind the typed
    /// `log_create`/`log_update`/`log_delete` helpers; stamps a fresh UUID and
    /// the current UTC time so callers never supply identity or timing.
    ///
    /// # Errors
    ///
    /// Returns an error if the row insert fails.
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        // Build the row: server-assigned UUID + server clock; the JSON
        // snapshots and actor metadata pass through verbatim.
        let new_audit = audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            timestamp: Set(time::OffsetDateTime::now_utc()),
            user_id: Set(actor.user_id.map(String::from)),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            old_values: Set(old_values),
            new_values: Set(new_values),
            ip_address: Set(actor.ip_address.map(String::from)),
            user_agent: Set(actor.user_agent.map(String::from)),
        };

        new_audit.insert(&self.db).await?;

        Ok(())
    }

    /// Returns up to `limit` audit entries for one entity, newest first.
    ///
    /// Filters by both `entity_type` and `entity_id` so audit IDs are only
    /// unique within an entity type. Backs the per-record audit endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
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

    /// Returns the `limit` most recent audit entries across all entities,
    /// newest first. Backs the system-wide recent-activity endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Returns up to `limit` audit entries performed by `user_id`, newest
    /// first. Backs the per-user audit endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
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
