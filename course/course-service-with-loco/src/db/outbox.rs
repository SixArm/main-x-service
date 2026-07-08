//! `course_outbox` write + relay surface for the durable event bus
//! (Phase 2; `agents/share/event-bus.md` §3, §5).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` — it is generic over
//! [`ConnectionTrait`](sea_orm::ConnectionTrait) so the repository can
//! pass its **own** `&DatabaseTransaction`, giving the entity write and
//! the event the same commit boundary. This genericity lives on the
//! plain `OutboxInsert` helper, **not** on the `dyn`-object-safe
//! [`CourseRepository`](crate::db::CourseRepository) trait (whose methods
//! cannot be generic and stay object-safe).
//!
//! [`Model::recent`] projects the last N rows to the operator
//! [`EventView`] shape; [`Model::unpublished`] / [`Model::mark_published`]
//! are the Phase-3 relay worker's poll + ack (roadmap — the relay itself
//! is not built yet).

use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::db::models::course_outbox;
use crate::streaming::EventView;
use crate::streaming::envelope::{Envelope, EventKind};

pub use course_outbox::Model;

/// Map a `SeaORM` error into the crate's [`Error::Database`](crate::Error).
fn map_db(e: &sea_orm::DbErr) -> crate::Error {
    crate::Error::Database(e.to_string())
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
    pub occurred_at: OffsetDateTime,
    /// Actor pid, if any.
    pub actor: Option<String>,
    /// Envelope schema version.
    pub schema_version: i32,
    /// The full envelope as JSON.
    pub payload: serde_json::Value,
}

impl OutboxInsert {
    /// Derive the outbox row from an envelope, stamping `occurred_at`
    /// (the envelope carries no wall-clock time, so the caller supplies
    /// it). The whole envelope is stored as `payload` so a consumer sees
    /// the exact §4 shape; `entity_pid` is the parsed pid.
    ///
    /// Pure and DB-free: no clock, no connection — the timestamp is an
    /// argument — so the mapping is fully unit-testable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`](crate::Error::Validation) when
    /// `env.pid` is not a UUID, or [`Error::Streaming`](crate::Error::Streaming)
    /// when the envelope fails to serialize.
    pub fn from_envelope(env: &Envelope, occurred_at: OffsetDateTime) -> Result<Self> {
        let entity_pid = Uuid::parse_str(&env.pid)
            .map_err(|e| crate::Error::Validation(format!("event pid is not a UUID: {e}")))?;
        let payload = serde_json::to_value(env)
            .map_err(|e| crate::Error::Streaming(format!("failed to serialize envelope: {e}")))?;
        Ok(Self {
            event_id: env.event_id,
            entity: env.entity.to_string(),
            entity_pid,
            kind: env.kind.token().to_string(),
            occurred_at,
            actor: env.actor.clone(),
            schema_version: i32::try_from(env.schema_version).unwrap_or(1),
            payload,
        })
    }

    /// Convenience: build the row for `kind` applied to `course`, stamping
    /// `occurred_at` to now. Wraps [`Envelope::for_event`](crate::streaming::Envelope::for_event)
    /// + [`Self::from_envelope`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_envelope`].
    pub fn for_event(course: &crate::models::Course, kind: EventKind) -> Result<Self> {
        let env = Envelope::for_event(course, kind);
        Self::from_envelope(&env, OffsetDateTime::now_utc())
    }

    /// Convenience: build the [`EventKind::Merged`] row for the surviving
    /// `survivor` record, carrying the absorbed `merged_from` duplicate's
    /// pid, stamping `occurred_at` to now. Wraps
    /// [`Envelope::for_merge`](crate::streaming::Envelope::for_merge) +
    /// [`Self::from_envelope`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_envelope`].
    pub fn for_merge(survivor: &crate::models::Course, merged_from: &Uuid) -> Result<Self> {
        let env = Envelope::for_merge(survivor, merged_from);
        Self::from_envelope(&env, OffsetDateTime::now_utc())
    }

    /// Build the `SeaORM` `ActiveModel` for this row (`id` auto,
    /// `published_at` starts `NULL`). Split out so [`Self::insert_on`]
    /// can take `&self`.
    fn active_model(&self) -> course_outbox::ActiveModel {
        course_outbox::ActiveModel {
            event_id: Set(self.event_id),
            entity: Set(self.entity.clone()),
            entity_pid: Set(self.entity_pid),
            kind: Set(self.kind.clone()),
            occurred_at: Set(self.occurred_at),
            actor: Set(self.actor.clone()),
            schema_version: Set(self.schema_version),
            payload: Set(self.payload.clone()),
            published_at: Set(None),
            ..Default::default()
        }
    }

    /// Durably enqueue this event **on the given connection** — pass a
    /// `&DatabaseTransaction` to share the entity mutation's transaction
    /// (the outbox guarantee: no committed change without its event, and
    /// vice versa).
    ///
    /// Generic over [`ConnectionTrait`](sea_orm::ConnectionTrait), so it
    /// accepts either the pooled connection or an open transaction. This
    /// is the seam that lets a `dyn`-trait repository thread the outbox
    /// insert into its own `begin()`…`commit()`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the insert
    /// fails (e.g. a duplicate `event_id`).
    pub async fn insert_on<C: ConnectionTrait>(&self, conn: &C) -> Result<Model> {
        let row = self.active_model().insert(conn).await.map_err(|e| map_db(&e))?;
        Ok(row)
    }
}

impl Model {
    /// The most recent outbox rows, **newest first**, projected to the
    /// operator [`EventView`] (`{kind, pid, name, seq}`) by deserializing
    /// each stored envelope `payload`. Drives a future `/events/recent`
    /// surface when the transport is `outbox`; a row whose payload cannot
    /// be parsed is skipped rather than failing the whole read.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the query
    /// fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> Result<Vec<EventView>> {
        let rows = course_outbox::Entity::find()
            .order_by_desc(course_outbox::Column::Id)
            .limit(limit)
            .all(db)
            .await
            .map_err(|e| map_db(&e))?;
        let views = rows
            .iter()
            .filter_map(|row| serde_json::from_value::<Envelope>(row.payload.clone()).ok())
            .map(|env| EventView::from(&env))
            .collect();
        Ok(views)
    }

    /// The Phase-3 relay poll: the oldest unpublished rows in id order,
    /// capped at `limit`. (The relay wraps this in a transaction with
    /// `FOR UPDATE SKIP LOCKED` so parallel relays don't double-ship; the
    /// ordering + `published_at IS NULL` filter is the shared shape.)
    /// Unused until the Fluvio relay lands (roadmap).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the query
    /// fails.
    pub async fn unpublished(db: &DatabaseConnection, limit: u64) -> Result<Vec<Self>> {
        let rows = course_outbox::Entity::find()
            .filter(course_outbox::Column::PublishedAt.is_null())
            .order_by_asc(course_outbox::Column::Id)
            .limit(limit)
            .all(db)
            .await
            .map_err(|e| map_db(&e))?;
        Ok(rows)
    }

    /// The Phase-3 relay ack: stamp `published_at = now()` on the given
    /// ids after a successful bus send. Returns the number of rows
    /// updated; a no-op on an empty slice. Unused until the relay lands.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the update
    /// fails.
    pub async fn mark_published<C: ConnectionTrait>(db: &C, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let res = course_outbox::Entity::update_many()
            .col_expr(
                course_outbox::Column::PublishedAt,
                Expr::current_timestamp().into(),
            )
            .filter(course_outbox::Column::Id.is_in(ids.iter().copied()))
            .exec(db)
            .await
            .map_err(|e| map_db(&e))?;
        Ok(res.rows_affected)
    }
}

/// DB-free pins for the pure envelope→row mapping: every field maps, the
/// full envelope survives as the payload, a non-UUID pid is rejected, and
/// the kind tokens match the envelope's lowercase serde form.
#[cfg(test)]
mod tests {
    use super::OutboxInsert;
    use crate::streaming::envelope::{ENTITY, Envelope, EventKind, SCHEMA_VERSION};
    use time::OffsetDateTime;
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
            name: "Intro to CS".to_string(),
            merged_from: None,
        }
    }

    fn an_instant() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap()
    }

    #[test]
    fn from_envelope_maps_every_column_and_keeps_the_full_payload() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000000";
        let occurred_at = an_instant();
        let env = an_envelope(EventKind::Created, pid);
        let row = OutboxInsert::from_envelope(&env, occurred_at).unwrap();

        assert_eq!(row.event_id, env.event_id);
        assert_eq!(row.entity, "course");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
        assert_eq!(row.kind, "created");
        assert_eq!(row.occurred_at, occurred_at);
        assert_eq!(row.actor.as_deref(), Some("user-1"));
        assert_eq!(row.schema_version, 1);
        // The payload is the exact envelope (JSON) — every field survives.
        assert_eq!(row.payload["pid"], pid);
        assert_eq!(row.payload["seq"], 7);
        assert_eq!(row.payload["name"], "Intro to CS");
        assert_eq!(row.payload["entity"], "course");
        assert_eq!(row.payload["kind"], "created");
        assert_eq!(row.payload["actor"], "user-1");
    }

    #[test]
    fn from_envelope_maps_a_merged_event() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000001";
        let row =
            OutboxInsert::from_envelope(&an_envelope(EventKind::Merged, pid), an_instant()).unwrap();
        assert_eq!(row.kind, "merged");
        assert_eq!(row.payload["kind"], "merged");
    }

    #[test]
    fn from_envelope_maps_a_deleted_event() {
        let pid = "0c4f1e2a-0000-4000-8000-000000000002";
        let row =
            OutboxInsert::from_envelope(&an_envelope(EventKind::Deleted, pid), an_instant())
                .unwrap();
        assert_eq!(row.kind, "deleted");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
    }

    #[test]
    fn from_envelope_rejects_a_non_uuid_pid() {
        assert!(
            OutboxInsert::from_envelope(&an_envelope(EventKind::Created, "not-a-uuid"), an_instant())
                .is_err()
        );
    }
}
