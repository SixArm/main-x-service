//! Audit-log repository — writes to the `audit_log` table and reads
//! back per-entity / recent histories.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
use crate::Result;

/// Public view of one audit-log row. Mirrors the column set with
/// camelCase-friendly serde rename so the JSON envelope matches the
/// rest of the API surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub user_id: Option<String>,
    pub user_ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub old_values: Option<JsonValue>,
    pub new_values: Option<JsonValue>,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<audit_log::Model> for AuditEntry {
    fn from(m: audit_log::Model) -> Self {
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

#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub struct AuditLogRepository {
    db: DatabaseConnection,
}

impl AuditLogRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action("CREATE", entity_type, entity_id, None, Some(new_values), ctx).await
    }

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

    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action("DELETE", entity_type, entity_id, Some(old_values), None, ctx).await
    }

    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
    ) -> Result<()> {
        let row = audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            entity_type: Set(entity_type.into()),
            entity_id: Set(entity_id),
            action: Set(action.into()),
            user_id: Set(ctx.user_id.clone()),
            user_ip_address: Set(ctx.ip_address.clone()),
            user_agent: Set(ctx.user_agent.clone()),
            old_values: Set(old_values),
            new_values: Set(new_values),
            created_at: Set(Utc::now()),
        };
        row.insert(&self.db)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(())
    }

    /// FR-14 — entries for a Course (or any of its child entities
    /// whose `entity_id` was set to the course id), newest first.
    pub async fn list_for_entity(&self, entity_id: Uuid, limit: u64) -> Result<Vec<AuditEntry>> {
        let rows = audit_log::Entity::find()
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .order_by_desc(audit_log::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(rows.into_iter().map(AuditEntry::from).collect())
    }

    /// `GET /api/audit/recent` — system-wide tail, newest first.
    pub async fn list_recent(&self, limit: u64) -> Result<Vec<AuditEntry>> {
        let rows = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(rows.into_iter().map(AuditEntry::from).collect())
    }
}
