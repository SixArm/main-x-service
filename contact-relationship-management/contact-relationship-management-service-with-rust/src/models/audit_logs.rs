//! `audit_logs` model — record and query the audit trail.
//!
//! Every mutation **and every sensitive read** (consent-history and
//! unmasked-amount reads) writes one row. The `snapshot` JSON carries the action
//! detail (old/new state, override reasons, the `owner` where
//! relevant — which is what the owner-scoped query filters on).

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::audit_logs::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::audit_logs::ActiveModel {}

impl Model {
    /// Record one audit entry. `actor` is the caller's `sub` (user
    /// `pid`) when a verified token was presented, else `None`.
    ///
    /// Generic over [`ConnectionTrait`] so it runs on the handler's
    /// `&DatabaseTransaction` — the audit row commits **with** the
    /// mutation it records (family invariant 8), never separately.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record<C: ConnectionTrait>(
        db: &C,
        entity: &str,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        let entry = audit_logs::ActiveModel {
            entity: ActiveValue::set(entity.to_string()),
            entity_pid: ActiveValue::set(entity_pid),
            action: ActiveValue::set(action.to_string()),
            actor: ActiveValue::set(actor.map(ToString::to_string)),
            snapshot: ActiveValue::set(snapshot),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(entry)
    }

    /// Most-recent audit entries, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Audit entries for one record, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: &DatabaseConnection, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::EntityPid.eq(entity_pid))
            .order_by_desc(audit_logs::Column::Id)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// The owner-scoped query: entries since `since` whose
    /// snapshot names this `owner`, most-recent first, capped at
    /// `limit`. The filter is applied over the snapshot JSON in Rust
    /// (cross-backend; the candidate set is already time-bounded).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_owner_since(
        db: &DatabaseConnection,
        owner: &str,
        since: chrono::DateTime<chrono::FixedOffset>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::CreatedAt.gte(since))
            .order_by_desc(audit_logs::Column::Id)
            .limit(limit.saturating_mul(10)) // headroom before the in-app filter
            .all(db)
            .await?;
        let rows = rows
            .into_iter()
            .filter(|row| {
                row.snapshot
                    .as_ref()
                    .and_then(|s| s.get("owner"))
                    .and_then(serde_json::Value::as_str)
                    == Some(owner)
            })
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect();
        Ok(rows)
    }
}
