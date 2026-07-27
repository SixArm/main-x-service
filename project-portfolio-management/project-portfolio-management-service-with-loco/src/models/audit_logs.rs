//! `audit_logs` model — record and query the CRUD audit trail.

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::audit_logs::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::audit_logs::ActiveModel {}

impl Model {
    /// Record one audit entry. `actor` is the caller's `sub` (user `pid`)
    /// when a verified token was presented, else `None`.
    ///
    /// Generic over [`ConnectionTrait`] so it runs either on the pooled
    /// `&DatabaseConnection` (the best-effort side-channel path, `memory`
    /// transport) **or** on a `&DatabaseTransaction` — under the `outbox`
    /// transport the audit row is written in the *same* transaction as the
    /// entity mutation and its `event_outbox` row, so the three can never
    /// disagree (`agents/share/event-bus.md` §3).
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record<C: ConnectionTrait>(
        db: &C,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        // Set `created_at` explicitly rather than letting the database
        // default it: the timestamp is part of the MAC pre-image, so it
        // must be known before the insert. The alternative — insert, then
        // stamp — leaves a window in which the row exists unprotected.
        let created_at: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let mac = crate::compliance::audit_integrity::tag(
            &crate::compliance::audit_integrity::AuditInput {
                entity_pid,
                action,
                actor,
                snapshot: snapshot.as_ref(),
                created_at_micros: created_at.timestamp_micros(),
            },
        );
        let entry = audit_logs::ActiveModel {
            entity_pid: ActiveValue::set(entity_pid),
            action: ActiveValue::set(action.to_string()),
            actor: ActiveValue::set(actor.map(ToString::to_string)),
            snapshot: ActiveValue::set(snapshot),
            created_at: ActiveValue::set(created_at),
            mac: ActiveValue::set(mac),
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

    /// Audit entries for one plan, most-recent first.
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
}
