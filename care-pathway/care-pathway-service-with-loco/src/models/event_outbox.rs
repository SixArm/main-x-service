//! `event_outbox` model — the transactional-outbox write + relay surface
//! for the durable event bus (Phase 2; `agents/share/event-bus.md` §3, §5).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` — it is generic over [`ConnectionTrait`] so a
//! request handler can pass its own `&DatabaseTransaction`, giving the
//! entity write and the event the same commit boundary. [`Model::recent`]
//! projects the last N rows to the frozen operator [`EventView`] shape.
//! [`Model::unpublished`] / [`Model::mark_published`] are the Phase-3
//! relay worker's poll + ack (roadmap; the relay itself is not built yet).

use loco_rs::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{ConnectionTrait, QueryOrder, QuerySelect, prelude::DateTimeWithTimeZone};
use uuid::Uuid;

pub use super::_entities::event_outbox::{self, ActiveModel, Entity, Model};
use crate::streaming::{Envelope, EventKind, EventView};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for super::_entities::event_outbox::ActiveModel {}

/// The wire token for an [`EventKind`] (matches the envelope's lowercase
/// serde form).
const fn kind_token(kind: EventKind) -> &'static str {
    match kind {
        EventKind::Created => "created",
        EventKind::Updated => "updated",
        EventKind::Deleted => "deleted",
        EventKind::Merged => "merged",
        EventKind::Linked => "linked",
        EventKind::Unlinked => "unlinked",
    }
}

/// The column values for one outbox row, derived from a canonical
/// [`Envelope`] plus the enqueue timestamp. Kept as a plain struct
/// (separate from the `SeaORM` `ActiveModel`) so the derivation — pid
/// parse, kind token, full-envelope JSON payload — is unit-testable
/// without a database. [`OutboxInsert::insert_on`] turns it into the
/// `ActiveModel` and inserts it on a caller-supplied connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxInsert {
    /// Envelope dedup id.
    pub event_id: Uuid,
    /// Entity name.
    pub entity: String,
    /// Record pid (bus partition key).
    pub entity_pid: Uuid,
    /// Change kind token.
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
    /// Derive the outbox row from an envelope, stamping `occurred_at`
    /// (the Phase-1 envelope carries no wall-clock time, so the handler
    /// supplies it). The whole envelope is stored as `payload` so a
    /// consumer sees the exact §4 shape; `entity_pid` is the parsed pid.
    ///
    /// Pure and DB-free: no clock, no connection — the timestamp is an
    /// argument — so the mapping is fully unit-testable.
    ///
    /// # Errors
    ///
    /// When `env.pid` is not a UUID, or the envelope fails to serialize.
    pub fn from_envelope(env: &Envelope, occurred_at: DateTimeWithTimeZone) -> ModelResult<Self> {
        let entity_pid = Uuid::parse_str(&env.pid).map_err(|e| ModelError::Any(Box::new(e)))?;
        let payload = serde_json::to_value(env).map_err(|e| ModelError::Any(Box::new(e)))?;
        Ok(Self {
            event_id: env.event_id,
            entity: env.entity.to_string(),
            entity_pid,
            kind: kind_token(env.kind).to_string(),
            occurred_at,
            actor: env.actor.clone(),
            schema_version: i32::try_from(env.schema_version).unwrap_or(1),
            payload,
        })
    }

    /// Build the `SeaORM` `ActiveModel` for this row (`published_at` starts
    /// `NULL`). Split out so [`Self::insert_on`] can take `&self`.
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
    /// `&DatabaseTransaction` to share the entity mutation's transaction
    /// (the outbox guarantee: no committed change without its event, and
    /// vice versa).
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
    /// The most recent outbox rows, **newest first**, projected to the
    /// frozen operator [`EventView`] (`{kind, pid, name, seq}`) by
    /// deserializing each stored envelope `payload`. Drives the
    /// `/events/recent` endpoint when the transport is `outbox`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<EventView>> {
        let rows = event_outbox::Entity::find()
            .order_by_desc(event_outbox::Column::Id)
            .limit(limit)
            .lock_with_behavior(
                sea_orm::sea_query::LockType::Update,
                sea_orm::sea_query::LockBehavior::SkipLocked,
            )
            .all(db)
            .await?;
        // Each `payload` is a full canonical envelope; project it to the
        // frozen view shape. A row whose payload cannot be parsed is
        // skipped rather than failing the whole read.
        let views = rows
            .iter()
            .filter_map(|row| serde_json::from_value::<Envelope>(row.payload.clone()).ok())
            .map(|env| EventView::from(&env))
            .collect();
        Ok(views)
    }

    /// The Phase-3 relay poll: the oldest unpublished rows in id order,
    /// capped at `limit`. (The relay wraps this in a transaction with `FOR
    /// UPDATE SKIP LOCKED` so parallel relays don't double-ship; the
    /// ordering + `published_at IS NULL` filter is the shared shape.)
    /// Unused until the Fluvio relay lands (roadmap).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn unpublished<C: ConnectionTrait>(db: &C, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::PublishedAt.is_null())
            .order_by_asc(event_outbox::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// The Phase-3 relay ack: stamp `published_at = now()` on the given
    /// ids after a successful bus send. Returns the number of rows
    /// updated; a no-op on an empty slice. Unused until the relay lands.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn mark_published<C: ConnectionTrait>(db: &C, ids: &[i32]) -> ModelResult<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let res = event_outbox::Entity::update_many()
            .col_expr(event_outbox::Column::PublishedAt, Expr::current_timestamp())
            .filter(event_outbox::Column::Id.is_in(ids.iter().copied()))
            .exec(db)
            .await?;
        Ok(res.rows_affected)
    }
}

/// DB-free pins for the pure envelope→row mapping: every field maps, the
/// full envelope survives as the payload, merged/deleted variants map,
/// a non-UUID pid is rejected, and the kind tokens match the envelope's
/// lowercase serde form.
#[cfg(test)]
mod tests {
    use super::{OutboxInsert, kind_token};
    use crate::streaming::{ENTITY, Envelope, EventKind, SCHEMA_VERSION};
    use chrono::{FixedOffset, TimeZone};
    use uuid::Uuid;

    fn an_envelope(kind: EventKind, pid: &str) -> Envelope {
        Envelope {
            event_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            schema_version: SCHEMA_VERSION,
            entity: ENTITY,
            kind,
            pid: pid.to_string(),
            seq: 7,
            actor: Some("user-1".to_string()),
            name: "Sepsis pathway".to_string(),
            data: None,
        }
    }

    fn an_instant() -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 6, 9, 0, 0)
            .unwrap()
    }

    #[test]
    fn from_envelope_maps_every_column_and_keeps_the_full_payload() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000000";
        let occurred_at = an_instant();
        let env = an_envelope(EventKind::Created, pid);
        let row = OutboxInsert::from_envelope(&env, occurred_at).unwrap();

        assert_eq!(row.event_id, env.event_id);
        assert_eq!(row.entity, "care_pathway");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
        assert_eq!(row.kind, "created");
        assert_eq!(row.occurred_at, occurred_at);
        assert_eq!(row.actor.as_deref(), Some("user-1"));
        assert_eq!(row.schema_version, 1);
        // The payload is the exact envelope (JSON) — every field survives.
        // Asserted as a `Value` rather than back into `Envelope`, whose
        // `entity: &'static str` field ties it to `Deserialize<'static>`.
        assert_eq!(row.payload["pid"], pid);
        assert_eq!(row.payload["seq"], 7);
        assert_eq!(row.payload["name"], "Sepsis pathway");
        assert_eq!(row.payload["entity"], "care_pathway");
        assert_eq!(row.payload["kind"], "created");
        assert_eq!(row.payload["actor"], "user-1");
    }

    #[test]
    fn from_envelope_maps_a_merged_event_with_actor() {
        // Merge and delete are the two kinds beyond create/update; pin the
        // merged mapping (the merge handler emits merged + deleted).
        let pid = "0c4f1e2a-0000-4000-8000-000000000001";
        let row = OutboxInsert::from_envelope(&an_envelope(EventKind::Merged, pid), an_instant())
            .unwrap();
        assert_eq!(row.kind, "merged");
        assert_eq!(row.payload["kind"], "merged");
    }

    #[test]
    fn from_envelope_maps_a_deleted_event() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000002";
        let row = OutboxInsert::from_envelope(&an_envelope(EventKind::Deleted, pid), an_instant())
            .unwrap();
        assert_eq!(row.kind, "deleted");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
    }

    #[test]
    fn from_envelope_rejects_a_non_uuid_pid() {
        assert!(
            OutboxInsert::from_envelope(
                &an_envelope(EventKind::Created, "not-a-uuid"),
                an_instant()
            )
            .is_err()
        );
    }

    #[test]
    fn kind_tokens_match_the_envelope_serde_form() {
        assert_eq!(kind_token(EventKind::Created), "created");
        assert_eq!(kind_token(EventKind::Updated), "updated");
        assert_eq!(kind_token(EventKind::Deleted), "deleted");
        assert_eq!(kind_token(EventKind::Merged), "merged");
    }
}
