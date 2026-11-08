//! `event_outbox` — the transactional-outbox write surface for the
//! durable event bus (Phase 2; `agents/share/event-bus.md` §3).
//!
//! [`OutboxInsert::from_envelope`] is the **pure** envelope→row mapping
//! (unit-testable DB-free). [`OutboxInsert::insert_on`] performs the
//! in-transaction `INSERT` — it takes a [`Db`], so the handler passes its
//! own transaction, giving the entity write and the event one commit
//! boundary. [`Model::unpublished`] / [`Model::mark_published`] are the
//! Phase-3 relay's poll + ack (roadmap; the relay itself is not built
//! yet).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};
use crate::streaming::{Envelope, EventView};

/// One persisted outbox row: a canonical envelope awaiting relay to the
/// durable bus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTime<FixedOffset>,
    /// Row last-update timestamp.
    pub updated_at: DateTime<FixedOffset>,
    /// Auto-increment pk (`SERIAL`, so `i32`); also the global relay
    /// order.
    pub id: i32,
    /// Envelope id — the consumer dedup key.
    pub event_id: Uuid,
    /// The entity name (e.g. `stay`).
    pub entity: String,
    /// The record pid — the bus partition key.
    pub entity_pid: Uuid,
    /// The change kind: `created` / `updated` / `deleted` / ….
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

/// The column values for one outbox row, derived from a canonical
/// [`Envelope`] plus the enqueue timestamp. A plain struct so the
/// derivation is unit-testable DB-free.
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
    pub occurred_at: DateTime<FixedOffset>,
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
    pub fn from_envelope(env: &Envelope, occurred_at: DateTime<FixedOffset>) -> ModelResult<Self> {
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

    /// Durably enqueue this event **on the given connection** — pass the
    /// mutation's transaction to share its commit boundary.
    /// `published_at` starts `NULL`.
    ///
    /// # Errors
    ///
    /// When the insert fails (e.g. a duplicate `event_id`).
    pub async fn insert_on(&self, db: impl Into<Db<'_>>) -> ModelResult<Model> {
        let mut conn = db.into().acquire().await?;
        // `id`, `created_at` and `updated_at` are column defaults.
        let row = sqlx::query_as!(
            Model,
            r#"INSERT INTO event_outbox (event_id, entity, entity_pid, kind, occurred_at,
                                         actor, schema_version, payload, published_at)
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
    /// The most recent outbox rows, newest first, projected to the
    /// frozen operator [`EventView`] shape. Drives `/events/recent`
    /// when the transport is `outbox`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: impl Into<Db<'_>>, limit: u64) -> ModelResult<Vec<EventView>> {
        let mut conn = db.into().acquire().await?;
        let payloads = sqlx::query_scalar!(
            "SELECT payload FROM event_outbox ORDER BY id DESC LIMIT $1",
            super::to_i64(limit),
        )
        .fetch_all(&mut *conn)
        .await?;
        let views = payloads
            .into_iter()
            .filter_map(|payload| serde_json::from_value::<Envelope>(payload).ok())
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
    pub async fn unpublished(db: impl Into<Db<'_>>, limit: u64) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, event_id, entity, entity_pid, kind,
                      occurred_at AS "occurred_at: _", actor, schema_version, payload,
                      published_at AS "published_at: _"
               FROM event_outbox
               WHERE published_at IS NULL
               ORDER BY id
               LIMIT $1
               FOR UPDATE SKIP LOCKED"#,
            super::to_i64(limit),
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// The Phase-3 relay ack: stamp `published_at = now()` on `ids`.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn mark_published(db: impl Into<Db<'_>>, ids: &[i32]) -> ModelResult<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut conn = db.into().acquire().await?;
        let done = sqlx::query!(
            "UPDATE event_outbox SET updated_at = now(), published_at = CURRENT_TIMESTAMP WHERE id = ANY($1)",
            ids,
        )
        .execute(&mut *conn)
        .await?;
        Ok(done.rows_affected())
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
        assert!(
            OutboxInsert::from_envelope(&an_envelope("created", "not-a-uuid"), an_instant())
                .is_err()
        );
    }
}
