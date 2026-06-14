//! Audit log repository: HIPAA-style write/query of the `audit_log` table.
//!
//! [`AuditLogRepository`] records who did what, when, and to which entity,
//! capturing old/new JSON snapshots plus request provenance (user id, IP,
//! user agent). The `log_create` / `log_update` / `log_delete` helpers are
//! thin wrappers over the private `log_action` insert. Query helpers back
//! the audit REST endpoints (per-entity, recent, per-user history).

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
use super::repositories::AuditContext;
use crate::Result;

/// Reads and writes the `audit_log` table.
///
/// Holds a cloned [`DatabaseConnection`]; construct one per shared
/// application state and wrap in an `Arc` to share across handlers.
pub struct AuditLogRepository {
    /// The `SeaORM` connection used for every audit query/insert.
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Wrap a database connection in an audit repository.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Record a `CREATE`: stores `new_values`, with no prior snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
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

    /// Record an `UPDATE`: stores both the prior and new JSON snapshots.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
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

    /// Record a `DELETE`: stores the prior snapshot, with no new values.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
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

    /// Insert one audit row. Shared backend for the typed `log_*` helpers.
    ///
    /// Stamps a fresh UUID and the current UTC time; `old_values` /
    /// `new_values` are `None` for the side that does not apply.
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
    ) -> Result<()> {
        let new_audit = audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            timestamp: Set(time::OffsetDateTime::now_utc()),
            user_id: Set(ctx.user_id.clone()),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            old_values: Set(old_values),
            new_values: Set(new_values),
            ip_address: Set(ctx.ip_address.clone()),
            user_agent: Set(ctx.user_agent.clone()),
        };

        new_audit.insert(&self.db).await?;

        Ok(())
    }

    /// Return up to `limit` audit rows for one entity, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
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

    /// Return the `limit` most recent audit rows system-wide.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Return up to `limit` audit rows for one user id, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
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
