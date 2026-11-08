//! `event_outbox` write + relay surface for the durable event bus
//! (Phase 2; `agents/share/event-bus.md` §3, §5).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` on a caller-supplied `&mut PgConnection` — the
//! repository passes its **own** open transaction, giving the entity write
//! and the event the same commit boundary. This lives on the plain
//! `OutboxInsert` helper, **not** on the `dyn`-object-safe
//! [`PersonRepository`](crate::db::PersonRepository) trait.
//!
//! [`Model::recent`] projects the last N rows to the frozen operator
//! [`EventView`] shape; [`Model::unpublished`] / [`Model::mark_published`]
//! are the relay's poll + ack, and [`Model::purge_published`] its
//! retention sweep.

use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::convert::offset_to_ts;
use super::models::{OutboxRaw, event_outbox};
use super::to_i64;
use crate::Result;
use crate::streaming::{Envelope, EventKind, EventView};

pub use event_outbox::Model;

/// The column values for one outbox row, derived from a canonical
/// [`Envelope`] plus the enqueue timestamp. Kept as a plain struct so the
/// derivation — pid parse, kind token, full-envelope JSON payload — is
/// unit-testable without a database. [`OutboxInsert::insert_on`] inserts
/// it on a caller-supplied connection.
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
            .map_err(|e| crate::Error::Validation(format!("person pid is not a UUID: {e}")))?;
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

    /// Convenience: build the row for `kind` applied to `person`, stamping
    /// `occurred_at` to now. Wraps [`Envelope::for_event`](crate::streaming::Envelope::for_event)
    /// + [`Self::from_envelope`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_envelope`].
    pub fn for_event(person: &crate::models::Person, kind: EventKind) -> Result<Self> {
        let env = Envelope::for_event(person, kind);
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
    pub fn for_merge(survivor: &crate::models::Person, merged_from: &Uuid) -> Result<Self> {
        let env = Envelope::for_merge(survivor, merged_from);
        Self::from_envelope(&env, OffsetDateTime::now_utc())
    }

    /// Durably enqueue this event **on the given connection** — pass an
    /// open transaction to share the entity mutation's transaction (the
    /// outbox guarantee: no committed change without its event, and vice
    /// versa). `id` is assigned by Postgres and `published_at` starts
    /// `NULL`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the insert
    /// fails (e.g. a duplicate `event_id`).
    pub async fn insert_on(&self, conn: &mut PgConnection) -> Result<Model> {
        let row = sqlx::query_as!(
            OutboxRaw,
            "INSERT INTO event_outbox
                 (event_id, entity, entity_pid, kind, occurred_at, actor, schema_version,
                  payload, published_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL)
             RETURNING id, event_id, entity, entity_pid, kind, occurred_at, actor,
                       schema_version, payload, published_at",
            self.event_id,
            self.entity,
            self.entity_pid,
            self.kind,
            offset_to_ts(self.occurred_at),
            self.actor,
            self.schema_version,
            self.payload,
        )
        .fetch_one(conn)
        .await?;
        Ok(row.into())
    }
}

impl Model {
    /// The most recent outbox rows, **newest first**, projected to the
    /// frozen operator [`EventView`] (`{kind, pid, name, seq}`) by
    /// deserializing each stored envelope `payload`. Drives the
    /// `/events/recent` endpoint when the transport is `outbox`; a row
    /// whose payload cannot be parsed is skipped rather than failing the
    /// whole read.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the query
    /// fails.
    pub async fn recent(db: &PgPool, limit: u64) -> Result<Vec<EventView>> {
        let payloads = sqlx::query_scalar!(
            "SELECT payload FROM event_outbox ORDER BY id DESC LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        let views = payloads
            .into_iter()
            .filter_map(|payload| serde_json::from_value::<Envelope>(payload).ok())
            .map(|env| EventView::from(&env))
            .collect();
        Ok(views)
    }

    /// The relay poll: the oldest unpublished rows in id order, capped at
    /// `limit`, **claimed** with `FOR UPDATE SKIP LOCKED` (SEC-B6). Must be
    /// called inside a transaction (the relay's): the row lock is what
    /// stops two parallel relay instances from both selecting and
    /// double-shipping the same rows — the second instance skips the
    /// locked rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the query
    /// fails.
    pub async fn unpublished(conn: &mut PgConnection, limit: u64) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            OutboxRaw,
            "SELECT id, event_id, entity, entity_pid, kind, occurred_at, actor,
                    schema_version, payload, published_at
             FROM event_outbox
             WHERE published_at IS NULL
             ORDER BY id ASC
             LIMIT $1
             FOR UPDATE SKIP LOCKED",
            to_i64(limit),
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// The relay ack: stamp `published_at = now()` on the given ids after a
    /// successful bus send. Returns the number of rows updated; a no-op on
    /// an empty slice.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the update
    /// fails.
    pub async fn mark_published(conn: &mut PgConnection, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let done = sqlx::query!(
            "UPDATE event_outbox SET published_at = $2 WHERE id = ANY($1)",
            ids,
            // This process's clock, because `purge_published` measures its
            // cutoff on it: Postgres time would move retention by the drift.
            chrono::Utc::now(),
        )
        .execute(conn)
        .await?;
        Ok(done.rows_affected())
    }

    /// The relay's retention sweep: delete rows published before `cutoff`.
    /// Unpublished rows are never touched.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the delete
    /// fails.
    pub async fn purge_published(db: &PgPool, cutoff: OffsetDateTime) -> Result<u64> {
        let done = sqlx::query!(
            "DELETE FROM event_outbox WHERE published_at IS NOT NULL AND published_at < $1",
            offset_to_ts(cutoff),
        )
        .execute(db)
        .await?;
        Ok(done.rows_affected())
    }

    /// Every outbox row for one record and kind, oldest first — a
    /// ground-truth read for tests.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the query
    /// fails.
    pub async fn rows_for(db: &PgPool, entity_pid: Uuid, kind: Option<&str>) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            OutboxRaw,
            "SELECT id, event_id, entity, entity_pid, kind, occurred_at, actor,
                    schema_version, payload, published_at
             FROM event_outbox
             WHERE entity_pid = $1 AND ($2::text IS NULL OR kind = $2)
             ORDER BY id ASC",
            entity_pid,
            kind,
        )
        .fetch_all(db)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

/// DB-free pins for the pure envelope→row mapping: every field maps, the
/// full envelope survives as the payload, a non-UUID pid is rejected, and
/// the kind tokens match the envelope's lowercase serde form.
#[cfg(test)]
mod tests {
    use super::OutboxInsert;
    use crate::streaming::{ENTITY, Envelope, EventKind, SCHEMA_VERSION};
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
            name: "John Smith".to_string(),
            merged_from: None,
            data: None,
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
        assert_eq!(row.entity, "person");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
        assert_eq!(row.kind, "created");
        assert_eq!(row.occurred_at, occurred_at);
        assert_eq!(row.actor.as_deref(), Some("user-1"));
        assert_eq!(row.schema_version, 1);
        // The payload is the exact envelope (JSON) — every field survives.
        assert_eq!(row.payload["pid"], pid);
        assert_eq!(row.payload["seq"], 7);
        assert_eq!(row.payload["name"], "John Smith");
        assert_eq!(row.payload["entity"], "person");
        assert_eq!(row.payload["kind"], "created");
        assert_eq!(row.payload["actor"], "user-1");
    }

    #[test]
    fn from_envelope_maps_a_merged_event() {
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
}
