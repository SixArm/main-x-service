//! HIPAA-style audit logging into the `audit_log` table.
//!
//! [`AuditLogRepository`] records who changed what and when, capturing
//! old/new values as JSON plus actor metadata (user ID, IP, user agent). The
//! `log_create` / `log_update` / `log_delete` helpers funnel into a single
//! private `log_action`, and the `get_*` queries back the audit REST
//! endpoints.

use sea_orm::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
use crate::Result;

/// Repository that writes and queries entries in the `audit_log` table.
pub struct AuditLogRepository {
    /// SeaORM connection used for all audit reads and writes.
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Wraps an existing database connection in an audit repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Records a `CREATE` action, storing `new_values` (no prior state).
    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        user_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        self.log_action(
            "CREATE",
            entity_type,
            entity_id,
            None,
            Some(new_values),
            user_id,
            ip_address,
            user_agent,
        )
        .await
    }

    /// Records an `UPDATE` action, storing both `old_values` and `new_values`.
    pub async fn log_update(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        user_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        self.log_action(
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            user_id,
            ip_address,
            user_agent,
        )
        .await
    }

    /// Records a `DELETE` action, storing `old_values` (no new state).
    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        user_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        self.log_action(
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            user_id,
            ip_address,
            user_agent,
        )
        .await
    }

    /// Inserts one audit row. Shared implementation behind the typed
    /// `log_create`/`log_update`/`log_delete` helpers; stamps a fresh UUID and
    /// the current UTC time.
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        user_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        let new_audit = audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            timestamp: Set(time::OffsetDateTime::now_utc()),
            user_id: Set(user_id),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            old_values: Set(old_values),
            new_values: Set(new_values),
            ip_address: Set(ip_address),
            user_agent: Set(user_agent),
        };

        new_audit.insert(&self.db).await?;

        Ok(())
    }

    /// Returns up to `limit` audit entries for one entity, newest first.
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

    /// Returns the `limit` most recent audit entries across all entities.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Returns up to `limit` audit entries performed by `user_id`, newest first.
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
