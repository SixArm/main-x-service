//! `event_outbox` model — the transactional-outbox write surface for
//! the durable event bus (Phase 2; `agents/share/event-bus.md` §3).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` — generic over [`ConnectionTrait`] so the
//! handler passes its own `&DatabaseTransaction`, giving the entity
//! write and the event one commit boundary. [`Model::unpublished`] /
//! [`Model::mark_published`] are the Phase-3 relay's poll + ack
//! (roadmap; the relay itself is not built yet).

use loco_rs::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect, prelude::DateTimeWithTimeZone};
use uuid::Uuid;

pub use super::_entities::event_outbox::{self, ActiveModel, Entity, Model};
use crate::streaming::{Envelope, EventView};

impl ActiveModelBehavior for super::_entities::event_outbox::ActiveModel {}

/// The column values for one outbox row, derived from a canonical
/// [`Envelope`] plus the enqueue timestamp. A plain struct (separate
/// from the `ActiveModel`) so the derivation is unit-testable DB-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxInsert {
    /// Envelope dedup id.
    pub event_id: Uuid,
    /// Entity name (`ward` / `bed` / `stay` / …).
    pub entity: String,
    /// Record pid (bus partition key).
    pub entity_pid: Uuid,
    /// Change kind token (`created`, `bed_state_changed`, …).
    pub kind: String,
    /// When the change occurred.
    pub occurred_at: DateTimeWithTimeZone,
    /// Actor pid, if any.
    pub actor: Option<String>,
    /// Envelope schema version.
    pub schema_version: i32,
    /// The full envelope as JSON.
    pub payload: serde_json::Value,
}

impl OutboxInsert {
    /// Derive the outbox row from an envelope, stamping `occurred_at`.
    /// Pure and DB-free: no clock, no connection.
    ///
    /// # Errors
    ///
    /// When `env.pid` is not a UUID, or the envelope fails to serialize.
    pub fn from_envelope(env: &Envelope, occurred_at: DateTimeWithTimeZone) -> ModelResult<Self> {
        let entity_pid = Uuid::parse_str(&env.pid).map_err(|e| ModelError::Any(Box::new(e)))?;
        let payload = serde_json::to_value(env).map_err(|e| ModelError::Any(Box::new(e)))?;
        Ok(Self {
            event_id: env.event_id,
            entity: env.entity.clone(),
            entity_pid,
            kind: env.kind.clone(),
            occurred_at,
            actor: env.actor.clone(),
            schema_version: i32::try_from(env.schema_version).unwrap_or(1),
            payload,
        })
    }

    /// Build the `ActiveModel` (`published_at` starts `NULL`).
    fn active_model(&self) -> ActiveModel {
        event_outbox::ActiveModel {
            event_id: ActiveValue::set(self.event_id),
            entity: ActiveValue::set(self.entity.clone()),
            entity_pid: ActiveValue::set(self.entity_pid),
            kind: ActiveValue::set(self.kind.clone()),
            occurred_at: ActiveValue::set(self.occurred_at),
            actor: ActiveValue::set(self.actor.clone()),
            schema_version: ActiveValue::set(self.schema_version),
            payload: ActiveValue::set(self.payload.clone()),
            published_at: ActiveValue::set(None),
            ..Default::default()
        }
    }

    /// Durably enqueue this event **on the given connection** — pass a
    /// `&DatabaseTransaction` to share the mutation's commit boundary.
    ///
    /// # Errors
    ///
    /// When the insert fails (e.g. a duplicate `event_id`).
    pub async fn insert_on<C: ConnectionTrait>(&self, conn: &C) -> ModelResult<Model> {
        let row = self.active_model().insert(conn).await?;
        Ok(row)
    }
}

impl Model {
    /// The most recent outbox rows, newest first, projected to the
    /// frozen operator [`EventView`] shape. Drives `/events/recent`
    /// when the transport is `outbox`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<EventView>> {
        let rows = event_outbox::Entity::find()
            .order_by_desc(event_outbox::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        let views = rows
            .iter()
            .filter_map(|row| serde_json::from_value::<Envelope>(row.payload.clone()).ok())
            .map(|env| EventView::from(&env))
            .collect();
        Ok(views)
    }

    /// The Phase-3 relay poll: oldest unpublished rows in id order,
    /// claimed with `FOR UPDATE SKIP LOCKED` (SEC-B6). Call inside the
    /// relay's transaction.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn unpublished<C: ConnectionTrait>(db: &C, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::PublishedAt.is_null())
            .order_by_asc(event_outbox::Column::Id)
            .limit(limit)
            .lock_with_behavior(
                sea_orm::sea_query::LockType::Update,
                sea_orm::sea_query::LockBehavior::SkipLocked,
            )
            .all(db)
            .await?;
        Ok(rows)
    }

    /// The Phase-3 relay ack: stamp `published_at = now()` on `ids`.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn mark_published<C: ConnectionTrait>(db: &C, ids: &[i32]) -> ModelResult<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let res = event_outbox::Entity::update_many()
            .col_expr(
                event_outbox::Column::PublishedAt,
                Expr::current_timestamp().into(),
            )
            .filter(event_outbox::Column::Id.is_in(ids.iter().copied()))
            .exec(db)
            .await?;
        Ok(res.rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::OutboxInsert;
    use crate::streaming::Envelope;
    use chrono::{FixedOffset, TimeZone};
    use uuid::Uuid;

    fn an_envelope(kind: &str, pid: &str) -> Envelope {
        Envelope {
            event_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            schema_version: crate::streaming::SCHEMA_VERSION,
            entity: "stay".to_string(),
            kind: kind.to_string(),
            pid: pid.to_string(),
            seq: 7,
            actor: Some("user-1".to_string()),
            name: "Bed 7A".to_string(),
            data: None,
        }
    }

    fn an_instant() -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 17, 9, 0, 0)
            .unwrap()
    }

    /// Every field maps and the full envelope survives as the payload.
    #[test]
    fn from_envelope_maps_every_column_and_keeps_the_full_payload() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000000";
        let env = an_envelope("stay_admitted", pid);
        let row = OutboxInsert::from_envelope(&env, an_instant()).unwrap();
        assert_eq!(row.event_id, env.event_id);
        assert_eq!(row.entity, "stay");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
        assert_eq!(row.kind, "stay_admitted");
        assert_eq!(row.actor.as_deref(), Some("user-1"));
        assert_eq!(row.payload["pid"], pid);
        assert_eq!(row.payload["kind"], "stay_admitted");
    }

    /// A non-UUID pid is rejected, never panics.
    #[test]
    fn from_envelope_rejects_a_non_uuid_pid() {
        assert!(OutboxInsert::from_envelope(&an_envelope("created", "not-a-uuid"), an_instant()).is_err());
    }
}
