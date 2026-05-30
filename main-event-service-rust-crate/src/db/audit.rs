//! Audit log repository for tracking changes

use sea_orm::*;
use uuid::Uuid;
use serde_json::Value as JsonValue;

use crate::Result;
use super::models::audit_log;

/// Audit log repository for recording changes
pub struct AuditLogRepository {
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Create a new audit log repository
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Log a create action
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
        ).await
    }

    /// Log an update action
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
        ).await
    }

    /// Log a delete action
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
        ).await
    }

    /// Log a generic action
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
            timestamp: Set(chrono::Utc::now()),
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

    /// Get audit logs for a specific entity
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

    /// Get recent audit logs
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Get audit logs by user
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
