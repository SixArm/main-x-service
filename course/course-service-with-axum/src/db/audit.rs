//! Audit-log repository — writes to the `audit_log` table and reads
//! back per-entity / recent histories.

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use uuid::Uuid;

use super::models::audit_log;
use crate::Result;

/// Public view of one audit-log row. Mirrors the column set with
/// camelCase-friendly serde rename so the JSON envelope matches the
/// rest of the API surface.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditEntry {
    /// Audit-row id.
    pub id: Uuid,
    /// Entity kind (e.g. `"course"`, `"course_instance"`).
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

impl From<audit_log::Model> for AuditEntry {
    /// Project a persisted `audit_log` row into the public view.
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
            created_at: super::convert::offset_to_ts(m.created_at),
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
    /// Shared `SeaORM` connection pool.
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Wrap an existing connection pool.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Record a `CREATE` with only post-change values.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if the audit-log row cannot be inserted.
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
    /// Returns [`enum@crate::Error`] if the audit-log row cannot be inserted.
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
    /// Returns [`enum@crate::Error`] if the audit-log row cannot be inserted.
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
        let created_at = time::OffsetDateTime::now_utc();
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
                created_at_micros: i64::try_from(created_at.unix_timestamp_nanos() / 1_000)
                    .unwrap_or(0),
            },
        );
        let row = audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            entity_type: Set(entity_type),
            entity_id: Set(entity_id),
            action: Set(action),
            user_id: Set(ctx.user_id.clone()),
            user_ip_address: Set(ctx.ip_address.clone()),
            user_agent: Set(ctx.user_agent.clone()),
            old_values: Set(old_values),
            new_values: Set(new_values),
            created_at: Set(created_at),
            hash: Set(Some(digests.sha256)),
            hash_sha3: Set(Some(digests.sha3)),
            mac: Set(digests.mac),
        };
        row.insert(&self.db)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(())
    }

    /// FR-14 — entries for a Course (or any of its child entities
    /// whose `entity_id` was set to the course id), newest first.
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if the query fails.
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
    ///
    /// # Errors
    ///
    /// Returns [`enum@crate::Error`] if the query fails.
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
