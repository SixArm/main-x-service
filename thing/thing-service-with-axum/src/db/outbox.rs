//! `event_outbox` write + relay surface for the durable event bus
//! (Phase 2; `agents/share/event-bus.md` §3, §5).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` on a caller-supplied `&mut PgConnection` — the
//! repository passes its **own** open transaction, giving the entity write
//! and the event the same commit boundary.
//!
//! [`recent`] projects the last N rows to the operator
//! [`EventView`] shape; [`unpublished`] / [`mark_published`] are the
//! relay's poll + ack, and [`purge_published`] its retention sweep.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::streaming::envelope::{Envelope, EventKind, EventView};

pub use super::models::OutboxRow;

/// The column values for one outbox row, derived from a canonical
/// [`Envelope`] plus the enqueue timestamp. Kept as a plain struct
/// (separate from the stored [`OutboxRow`], which adds the database's
/// `id` and `published_at`) so the derivation — pid parse, kind token,
/// full-envelope JSON payload — is unit-testable without a database.
/// [`OutboxInsert::insert_on`] inserts it on a caller-supplied connection.
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

    /// Convenience: build the row for `kind` applied to `thing`, stamping
    /// `occurred_at` to now. Wraps [`Envelope::for_event`](crate::streaming::Envelope::for_event)
    /// + [`Self::from_envelope`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_envelope`].
    pub fn for_event(thing: &crate::models::thing::Thing, kind: EventKind) -> Result<Self> {
        let env = Envelope::for_event(thing, kind);
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
    pub fn for_merge(survivor: &crate::models::thing::Thing, merged_from: &Uuid) -> Result<Self> {
        let env = Envelope::for_merge(survivor, merged_from);
        Self::from_envelope(&env, OffsetDateTime::now_utc())
    }

    /// Durably enqueue this event **on the given connection** — pass the
    /// open transaction of the entity mutation (the outbox guarantee: no
    /// committed change without its event, and vice versa). `id` is the
    /// `BIGSERIAL` default and `published_at` starts `NULL`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Database`](crate::Error::Database) if the insert
    /// fails (e.g. a duplicate `event_id`).
    pub async fn insert_on(&self, conn: &mut PgConnection) -> Result<OutboxRow> {
        let row = sqlx::query_as!(
            OutboxRow,
            "INSERT INTO event_outbox
                 (event_id, entity, entity_pid, kind, occurred_at, actor, schema_version, payload)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, event_id, entity, entity_pid, kind, occurred_at, actor,
                       schema_version, payload, published_at",
            self.event_id,
            self.entity,
            self.entity_pid,
            self.kind,
            super::convert::offset_to_ts(self.occurred_at),
            self.actor,
            self.schema_version,
            self.payload,
        )
        .fetch_one(conn)
        .await?;
        Ok(row)
    }
}

/// The most recent outbox rows, **newest first**, projected to the
/// operator [`EventView`] (`{kind, pid, name, seq}`) by deserializing each
/// stored envelope `payload`. Drives an operator `recent` surface when the
/// transport is `outbox`; a row whose payload cannot be parsed is skipped
/// rather than failing the whole read.
///
/// # Errors
///
/// Returns [`Error::Database`](crate::Error::Database) if the query fails.
pub async fn recent(pool: &PgPool, limit: u64) -> Result<Vec<EventView>> {
    let payloads = sqlx::query_scalar!(
        "SELECT payload FROM event_outbox ORDER BY id DESC LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(pool)
    .await?;
    let views = payloads
        .into_iter()
        .filter_map(|payload| serde_json::from_value::<Envelope>(payload).ok())
        .map(|env| EventView::from(&env))
        .collect();
    Ok(views)
}

/// The relay poll: the oldest unpublished rows in id order, capped at
/// `limit`. `FOR UPDATE SKIP LOCKED` is what stops two relays shipping one
/// row (SEC-B6): rows held by another relay are skipped, and the rows
/// returned stay locked until the caller's transaction ends, so run this
/// in the transaction that marks them published.
///
/// # Errors
///
/// Returns [`Error::Database`](crate::Error::Database) if the query fails.
pub async fn unpublished(conn: &mut PgConnection, limit: u64) -> Result<Vec<OutboxRow>> {
    let rows = sqlx::query_as!(
        OutboxRow,
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
    Ok(rows)
}

/// The relay ack: stamp `published_at = now()` on the given ids after a
/// successful bus send. Returns the number of rows updated; a no-op on an
/// empty slice.
///
/// # Errors
///
/// Returns [`Error::Database`](crate::Error::Database) if the update fails.
pub async fn mark_published(conn: &mut PgConnection, ids: &[i64]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let done = sqlx::query!(
        "UPDATE event_outbox SET published_at = CURRENT_TIMESTAMP WHERE id = ANY($1)",
        ids,
    )
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Delete published rows whose `published_at` is older than `cutoff` (the
/// outbox is a short-lived hand-off buffer; durable history is the bus's
/// job). Returns the number of rows purged.
///
/// # Errors
///
/// Returns [`Error::Database`](crate::Error::Database) if the delete fails.
pub async fn purge_published(pool: &PgPool, cutoff: DateTime<Utc>) -> Result<u64> {
    let done = sqlx::query!(
        "DELETE FROM event_outbox WHERE published_at IS NOT NULL AND published_at < $1",
        cutoff,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// Every outbox row for one record pid and change kind, oldest first.
/// Read by the DB-gated tests that prove a write enqueued its event.
///
/// # Errors
///
/// Returns [`Error::Database`](crate::Error::Database) if the query fails.
pub async fn rows_for(pool: &PgPool, entity_pid: Uuid, kind: &str) -> Result<Vec<OutboxRow>> {
    let rows = sqlx::query_as!(
        OutboxRow,
        "SELECT id, event_id, entity, entity_pid, kind, occurred_at, actor,
                schema_version, payload, published_at
         FROM event_outbox
         WHERE entity_pid = $1 AND kind = $2
         ORDER BY id ASC",
        entity_pid,
        kind,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// A `LIMIT` bound: Postgres takes `BIGINT`, the callers count in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
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
            name: "Pride and Prejudice".to_string(),
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
        assert_eq!(row.entity, "thing");
        assert_eq!(row.entity_pid, Uuid::parse_str(pid).unwrap());
        assert_eq!(row.kind, "created");
        assert_eq!(row.occurred_at, occurred_at);
        assert_eq!(row.actor.as_deref(), Some("user-1"));
        assert_eq!(row.schema_version, 1);
        // The payload is the exact envelope (JSON) — every field survives.
        assert_eq!(row.payload["pid"], pid);
        assert_eq!(row.payload["seq"], 7);
        assert_eq!(row.payload["name"], "Pride and Prejudice");
        assert_eq!(row.payload["entity"], "thing");
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

/// DB-gated (`#[ignore]`) proof of the relay poll's lock behaviour:
/// rows one relay holds are invisible to a second relay polling at the
/// same time (SEC-B6). Needs a migrated `PostgreSQL` via `DATABASE_URL`.
#[cfg(test)]
mod relay_poll {
    use super::{OutboxInsert, unpublished};
    use crate::models::thing::Thing;
    use crate::streaming::envelope::EventKind;

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn the_poll_skips_rows_another_relay_holds() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        let pool = sqlx::PgPool::connect(&url).await.expect("connect");
        let thing = Thing::new("Relay Poll Lock Fixture");
        let row = OutboxInsert::for_event(&thing, EventKind::Created).unwrap();
        let mut conn = pool.acquire().await.unwrap();
        let inserted = row.insert_on(&mut conn).await.unwrap();
        drop(conn);

        // Relay one claims every unpublished row, and keeps them locked.
        let mut first = pool.begin().await.unwrap();
        let claimed = unpublished(&mut first, 10_000).await.unwrap();
        assert!(
            claimed.iter().any(|r| r.id == inserted.id),
            "relay one claims the row"
        );

        // Relay two, polling meanwhile, must not see that row.
        let mut second = pool.begin().await.unwrap();
        let seen = unpublished(&mut second, 10_000).await.unwrap();
        assert!(
            seen.iter().all(|r| r.id != inserted.id),
            "a row another relay holds is skipped"
        );
        second.rollback().await.unwrap();
        first.rollback().await.unwrap();
    }
}
