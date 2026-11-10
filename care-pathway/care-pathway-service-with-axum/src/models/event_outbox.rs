//! `event_outbox` model — the transactional-outbox write + relay surface
//! for the durable event bus (Phase 2; `agents/share/event-bus.md` §3, §5).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` — it takes an [`axa::sqlx::Db`] so a request
//! handler can pass its own open transaction, giving the entity write and
//! the event the same commit boundary. [`Model::recent`] projects the last
//! N rows to the frozen operator [`EventView`] shape.
//! [`Model::unpublished`] / [`Model::mark_published`] /
//! [`Model::purge_published`] are the Phase-3 relay worker's poll, ack and
//! retention sweep.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{ModelError, ModelResult};
use crate::streaming::{Envelope, EventKind, EventView};

/// One persisted outbox row: a canonical envelope awaiting relay to the
/// durable bus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTime<FixedOffset>,
    /// Row creation timestamp too: the column default, never bumped.
    pub updated_at: DateTime<FixedOffset>,
    /// Auto-increment pk; also the global relay order.
    pub id: i32,
    /// Envelope id — the consumer dedup key.
    pub event_id: Uuid,
    /// The entity name (e.g. `care_pathway`).
    pub entity: String,
    /// The record pid — the bus partition key.
    pub entity_pid: Uuid,
    /// The change kind: `created` / `updated` / `deleted` / `merged`.
    pub kind: String,
    /// When the change occurred (stamped at enqueue).
    pub occurred_at: DateTime<FixedOffset>,
    /// The acting user's pid, or `None` when unauthenticated.
    pub actor: Option<String>,
    /// The envelope schema version.
    pub schema_version: i32,
    /// The full canonical envelope as JSONB.
    pub payload: serde_json::Value,
    /// `None` until the relay ships the row to the bus.
    pub published_at: Option<DateTime<FixedOffset>>,
}

/// A `LIMIT` bound: Postgres takes `BIGINT`, the callers count in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

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
    pub occurred_at: DateTime<FixedOffset>,
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
    pub fn from_envelope(env: &Envelope, occurred_at: DateTime<FixedOffset>) -> ModelResult<Self> {
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

    /// Durably enqueue this event **on the given connection** — pass the
    /// open transaction to share the entity mutation's commit boundary
    /// (the outbox guarantee: no committed change without its event, and
    /// vice versa). `published_at` starts `NULL`.
    ///
    /// # Errors
    ///
    /// When the insert fails (e.g. a duplicate `event_id`).
    pub async fn insert_on(&self, db: impl Into<Db<'_>>) -> ModelResult<Model> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Model,
            r#"INSERT INTO event_outbox
                   (event_id, entity, entity_pid, kind, occurred_at, actor,
                    schema_version, payload, published_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, event_id, entity, entity_pid, kind,
                         occurred_at AS "occurred_at: _", actor, schema_version, payload,
                         published_at AS "published_at: _""#,
            self.event_id,
            self.entity,
            self.entity_pid,
            self.kind,
            self.occurred_at,
            self.actor,
            self.schema_version,
            self.payload,
        )
        .fetch_one(&mut *conn)
        .await?;
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
    pub async fn recent(db: &PgPool, limit: u64) -> ModelResult<Vec<EventView>> {
        let payloads = sqlx::query_scalar!(
            "SELECT payload FROM event_outbox ORDER BY id DESC LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        // Each `payload` is a full canonical envelope; project it to the
        // frozen view shape. A row whose payload cannot be parsed is
        // skipped rather than failing the whole read.
        let views = payloads
            .into_iter()
            .filter_map(|payload| serde_json::from_value::<Envelope>(payload).ok())
            .map(|env| EventView::from(&env))
            .collect();
        Ok(views)
    }

    /// The Phase-3 relay poll: the oldest unpublished rows in id order,
    /// capped at `limit`. `FOR UPDATE SKIP LOCKED` is what stops two
    /// relays shipping one row (SEC-B6): rows held by another relay are
    /// skipped, and the rows returned stay locked until the caller's
    /// transaction ends, so run this in the transaction that marks them
    /// published.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn unpublished(conn: &mut PgConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, event_id, entity, entity_pid, kind,
                      occurred_at AS "occurred_at: _", actor, schema_version, payload,
                      published_at AS "published_at: _"
               FROM event_outbox
               WHERE published_at IS NULL
               ORDER BY id ASC
               LIMIT $1
               FOR UPDATE SKIP LOCKED"#,
            to_i64(limit),
        )
        .fetch_all(conn)
        .await?;
        Ok(rows)
    }

    /// The Phase-3 relay ack: stamp `published_at` (this process's clock) on
    /// the given ids after a successful bus send. Returns the number of
    /// rows updated; a no-op on an empty slice.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn mark_published(conn: &mut PgConnection, ids: &[i32]) -> ModelResult<u64> {
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

    /// Delete published rows whose `published_at` is older than `cutoff`
    /// (the outbox is a short-lived hand-off buffer; durable history is
    /// the bus's job). Returns the number of rows purged.
    ///
    /// # Errors
    ///
    /// When the delete fails.
    pub async fn purge_published(db: &PgPool, cutoff: DateTime<Utc>) -> ModelResult<u64> {
        let done = sqlx::query!(
            "DELETE FROM event_outbox WHERE published_at IS NOT NULL AND published_at < $1",
            cutoff.fixed_offset(),
        )
        .execute(db)
        .await?;
        Ok(done.rows_affected())
    }

    /// Every outbox row for one record pid, oldest first. Read by the
    /// DB-gated tests that prove a write enqueued its event.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: &PgPool, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, event_id, entity, entity_pid, kind,
                      occurred_at AS "occurred_at: _", actor, schema_version, payload,
                      published_at AS "published_at: _"
               FROM event_outbox
               WHERE entity_pid = $1
               ORDER BY id ASC"#,
            entity_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// How many outbox rows there are.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_all(db: &PgPool) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM event_outbox"#)
            .fetch_one(db)
            .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }
}

/// DB-free pins for the pure envelope→row mapping: every field maps, the
/// full envelope survives as the payload, a non-UUID pid is rejected, and
/// the kind tokens match the envelope's lowercase serde form.
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

/// DB-gated (`#[ignore]`) proof of the relay poll's lock behaviour: rows
/// one relay holds are invisible to a second relay polling at the same
/// time (SEC-B6).
/// Needs a migrated `PostgreSQL` via `DB_URL`.
#[cfg(test)]
mod relay_poll {
    use super::{Model, OutboxInsert};
    use crate::streaming::{ENTITY, Envelope, EventKind, SCHEMA_VERSION};
    use uuid::Uuid;

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
    async fn the_poll_skips_rows_another_relay_holds() {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for DB tests");
        let pool = sqlx::PgPool::connect(&url).await.expect("connect");
        crate::models::MIGRATOR.run(&pool).await.expect("migrate");
        let env = Envelope {
            event_id: Uuid::new_v4(),
            schema_version: SCHEMA_VERSION,
            entity: ENTITY,
            kind: EventKind::Created,
            pid: Uuid::new_v4().to_string(),
            seq: 1,
            actor: None,
            name: "Relay Poll Lock Fixture".to_string(),
            data: None,
        };
        let inserted = OutboxInsert::from_envelope(&env, chrono::Utc::now().into())
            .unwrap()
            .insert_on(&pool)
            .await
            .unwrap();

        // Relay one claims every unpublished row, and keeps them locked.
        let mut first = pool.begin().await.unwrap();
        let claimed = Model::unpublished(&mut first, 10_000).await.unwrap();
        assert!(
            claimed.iter().any(|r| r.id == inserted.id),
            "relay one claims the row"
        );

        // Relay two, polling meanwhile, must not see that row.
        let mut second = pool.begin().await.unwrap();
        let seen = Model::unpublished(&mut second, 10_000).await.unwrap();
        assert!(
            seen.iter().all(|r| r.id != inserted.id),
            "a row another relay holds is skipped"
        );
        second.rollback().await.unwrap();
        first.rollback().await.unwrap();
    }
}
