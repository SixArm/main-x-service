//! Event stream — Phases 1 & 2 of the durable event bus.
//!
//! Every CRUD action publishes a canonical [`Envelope`]. Two transports
//! sit behind the [`EventTransport`] selector (config
//! `ORGANIZATION_EVENT_TRANSPORT`, default `memory`; see
//! `agents/share/event-bus.md` §7):
//!
//! - **`memory`** (Phase 1, default) — a process-wide ring buffer via the
//!   [`EventPublisher`] seam / [`InMemoryPublisher`]. Behaviour is
//!   identical to the original free-function ring buffer; no DB, no tx.
//! - **`outbox`** (Phase 2) — the [`OutboxPublisher`] inserts one
//!   `event_outbox` row **on the handler's transaction**, so the entity
//!   write and its event commit or roll back together (no committed
//!   change without its event, and vice versa). The transaction-aware
//!   write+emit path is exposed as [`create_and_emit`] /
//!   [`update_and_emit`] / [`delete_and_emit`] / [`merge_and_emit`], which
//!   both the native and FHIR controllers call so neither has to know the
//!   transport. Phase 3 (the Fluvio relay worker) is roadmap.
//!
//! The operator endpoint `GET /api/organizations/events/recent` returns
//! the flat [`EventView`] projection (`{kind, pid, name, seq}`), a frozen
//! wire shape consumed by the front-end — served from the ring buffer
//! (`memory`) or the outbox (`outbox`) via [`recent_events`], identically.
//!
//! In loco there is no per-request shared state for this, so the
//! ring-buffer publisher is a `OnceLock`-initialised global.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use loco_rs::prelude::ModelResult;
use organization_matcher::Organization;
use sea_orm::{ConnectionTrait, DatabaseConnection, IntoActiveModel, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::event_outbox::{Model as OutboxRow, OutboxInsert};
use crate::models::organizations::Model as OrgModel;

/// The entity this service publishes events for.
pub const ENTITY: &str = "organization";

/// Current envelope schema version. Bumped on any breaking envelope
/// change; additive fields do not bump it (see event-bus.md §4).
pub const SCHEMA_VERSION: u32 = 1;

/// The kind of change that occurred.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventKind {
    /// A record was created.
    Created,
    /// A record was updated.
    Updated,
    /// A record was soft-deleted.
    Deleted,
    /// A duplicate was merged into this (surviving) record.
    Merged,
}

/// The canonical, versioned event envelope (event-bus.md §4).
///
/// One shape for every entity and every transport. Phase 1 omits
/// `occurred_at` and `data`: those are added at the outbox stage
/// (Phase 2), where the handler transaction supplies an authoritative
/// timestamp and the full record snapshot. No timestamp is included here
/// because Phase 1 keeps the in-memory envelope minimal — the wall-clock
/// `occurred_at` belongs with the durable outbox row, not the ring buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Idempotency / dedup key, end to end (UUID v4).
    pub event_id: Uuid,
    /// Envelope schema version (see [`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// The entity name (`snake_case`), e.g. `"organization"`.
    ///
    /// Serialized verbatim; on deserialize it is fixed to [`ENTITY`]
    /// (a `&'static str` cannot borrow from the input), which is correct
    /// because this service only ever emits its own entity.
    #[serde(skip_deserializing, default = "default_entity")]
    pub entity: &'static str,
    /// The kind of change.
    pub kind: EventKind,
    /// The record's public id.
    pub pid: String,
    /// Monotonic sequence number (per process).
    pub seq: u64,
    /// The user `pid` (bearer `sub`) that caused the change, when known.
    pub actor: Option<String>,
    /// The record's denormalised label (its name) at event time.
    pub name: String,
}

/// The fixed entity name, used as the deserialize default for
/// [`Envelope::entity`] (a `&'static str` cannot borrow from input).
fn default_entity() -> &'static str {
    ENTITY
}

/// The flat operator-endpoint projection of an [`Envelope`].
///
/// This is the **frozen** `/events/recent` wire shape (`{kind, pid,
/// name, seq}`); the front-end consumes it. Do not add or rename fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventView {
    /// The kind of change.
    pub kind: EventKind,
    /// The record's public id.
    pub pid: String,
    /// The record's name at the time of the event.
    pub name: String,
    /// Monotonic sequence number (per process).
    pub seq: u64,
}

/// Project a full [`Envelope`] down to the frozen [`EventView`] wire
/// shape, dropping the internal envelope fields the front-end never sees.
impl From<&Envelope> for EventView {
    fn from(env: &Envelope) -> Self {
        Self {
            kind: env.kind,
            pid: env.pid.clone(),
            name: env.name.clone(),
            seq: env.seq,
        }
    }
}

/// The publisher seam (event-bus.md §5). In Phase 1 the only
/// implementation is [`InMemoryPublisher`]; the outbox / Fluvio
/// implementations arrive in Phases 2–3.
pub trait EventPublisher: Send + Sync {
    /// Enqueue an event onto the stream. Never fails; an in-memory
    /// implementation that cannot push (e.g. a poisoned lock) drops the
    /// event silently — the audit log is the durable record.
    fn publish(&self, env: Envelope);

    /// The most recent events (newest last), projected to [`EventView`]
    /// and capped at `limit`.
    fn recent(&self, limit: usize) -> Vec<EventView>;
}

/// Ring-buffer capacity: the most recent `CAPACITY` events are retained;
/// older ones are evicted from the front. Bounds memory for an in-process
/// buffer that is not meant to be a durable log (the audit table is).
const CAPACITY: usize = 1000;

/// The in-memory ring-buffer publisher: today's behaviour, behind the
/// [`EventPublisher`] trait. Default for tests and single-node dev;
/// keeps `cargo test` DB-free.
#[derive(Debug, Default)]
pub struct InMemoryPublisher {
    /// The bounded FIFO of recent envelopes, `Mutex`-guarded for the
    /// cross-thread (multi-request) publish/read access.
    buffer: Mutex<VecDeque<Envelope>>,
}

impl InMemoryPublisher {
    /// Create an empty publisher with the configured capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }
}

impl EventPublisher for InMemoryPublisher {
    fn publish(&self, env: Envelope) {
        // A poisoned lock drops the event (the trait contract: never
        // fail); the audit log remains the durable record.
        if let Ok(mut buf) = self.buffer.lock() {
            // Enforce the cap by evicting the oldest before pushing.
            if buf.len() == CAPACITY {
                buf.pop_front();
            }
            buf.push_back(env);
        }
    }

    fn recent(&self, limit: usize) -> Vec<EventView> {
        // A poisoned lock yields an empty result rather than panicking.
        self.buffer.lock().map_or_else(
            |_| Vec::new(),
            |buf| {
                // Take the newest `limit` (iterate from the back), then
                // re-reverse so the result is oldest-to-newest.
                buf.iter()
                    .rev()
                    .take(limit)
                    .rev()
                    .map(EventView::from)
                    .collect()
            },
        )
    }
}

/// The process-wide publisher instance, lazily initialised on first use.
/// In loco there is no per-request shared state for this, so a global is
/// the natural home (see the module docs).
fn publisher() -> &'static InMemoryPublisher {
    static PUBLISHER: OnceLock<InMemoryPublisher> = OnceLock::new();
    PUBLISHER.get_or_init(InMemoryPublisher::new)
}

/// Hand out the next monotonic per-process sequence number (starting at
/// 1). `Relaxed` is sufficient: `seq` only needs to be unique and
/// increasing, not synchronised with other memory.
fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Build a fresh [`Envelope`] with a new `event_id` and the next `seq`.
fn envelope(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) -> Envelope {
    Envelope {
        event_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        entity: ENTITY,
        kind,
        pid: pid.to_string(),
        seq: next_seq(),
        actor: actor.map(ToString::to_string),
        name: name.to_string(),
    }
}

/// Publish an event to the in-memory stream (no known actor).
///
/// Back-compat shim over [`publish_with_actor`]; never fails.
pub fn publish(kind: EventKind, pid: &str, name: &str) {
    publish_with_actor(kind, pid, name, None);
}

/// Publish an event, stamping the caller identity when known.
///
/// `actor` is the verified bearer `sub` (user `pid`) from
/// `MaybeAuthUser`, or `None` when no token was presented.
pub fn publish_with_actor(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) {
    publisher().publish(envelope(kind, pid, name, actor));
}

/// The most recent events (newest last), projected to [`EventView`] and
/// capped at `limit`. Drives the frozen `/events/recent` endpoint for the
/// `memory` transport.
#[must_use]
pub fn recent(limit: usize) -> Vec<EventView> {
    publisher().recent(limit)
}

/// Which event transport is active (event-bus.md §7). `memory` is the
/// default and today's behaviour; `outbox` durably enqueues each event on
/// the entity mutation's transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTransport {
    /// Process-wide ring buffer; no DB, no transaction (default).
    Memory,
    /// Transactional outbox: an `event_outbox` row per event, written on
    /// the handler's transaction (Phase 2).
    Outbox,
}

impl EventTransport {
    /// Parse the `ORGANIZATION_EVENT_TRANSPORT` value. `outbox` selects the
    /// outbox; `memory` and **any unrecognised value** fall back to
    /// `memory` (fail-safe to today's behaviour), case-insensitively.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "outbox" => Self::Outbox,
            _ => Self::Memory,
        }
    }

    /// Whether the outbox transport is selected.
    #[must_use]
    pub const fn is_outbox(self) -> bool {
        matches!(self, Self::Outbox)
    }
}

/// The process-wide transport, read once from `ORGANIZATION_EVENT_TRANSPORT`
/// and cached (mirrors the auth `REQUIRE_AUTH` env pattern). Unset or
/// unrecognised ⇒ [`EventTransport::Memory`].
#[must_use]
pub fn transport() -> EventTransport {
    static TRANSPORT: OnceLock<EventTransport> = OnceLock::new();
    *TRANSPORT.get_or_init(|| {
        std::env::var("ORGANIZATION_EVENT_TRANSPORT")
            .map_or(EventTransport::Memory, |v| EventTransport::parse(&v))
    })
}

/// The transactional-outbox publisher (event-bus.md §5), sitting alongside
/// [`InMemoryPublisher`]. Unlike the ring buffer, its `publish` is async
/// and takes a connection, so it can run **inside the handler's
/// transaction** — the outbox guarantee. It holds no state (the durable
/// store is Postgres), so it is a unit type.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutboxPublisher;

impl OutboxPublisher {
    /// Insert one `event_outbox` row for `env` **on `conn`**. Pass a
    /// `&DatabaseTransaction` to share the entity mutation's commit
    /// boundary. `occurred_at` is stamped now (the Phase-1 envelope has no
    /// wall-clock time).
    ///
    /// # Errors
    ///
    /// When the envelope pid is not a UUID, or the insert fails.
    pub async fn publish<C: ConnectionTrait>(self, conn: &C, env: &Envelope) -> ModelResult<()> {
        let insert = OutboxInsert::from_envelope(env, chrono::Utc::now().into())?;
        insert.insert_on(conn).await?;
        Ok(())
    }

    /// The most recent outbox rows, newest first, projected to
    /// [`EventView`] (drives `/events/recent` under the `outbox`
    /// transport).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(self, db: &DatabaseConnection, limit: usize) -> ModelResult<Vec<EventView>> {
        OutboxRow::recent(db, u64::try_from(limit).unwrap_or(u64::MAX)).await
    }
}

/// Recent events for the operator endpoint, from whichever transport is
/// active: the ring buffer (`memory`) or the outbox table (`outbox`). The
/// wire shape — a `Vec<EventView>` — is identical either way.
///
/// # Errors
///
/// When the outbox query fails (`memory` never errors).
pub async fn recent_events(db: &DatabaseConnection, limit: usize) -> ModelResult<Vec<EventView>> {
    match transport() {
        EventTransport::Memory => Ok(recent(limit)),
        EventTransport::Outbox => OutboxPublisher.recent(db, limit).await,
    }
}

/// Create an organization and emit its `Created` event, atomically under
/// the active transport. `memory`: insert on `db`, then push to the ring
/// buffer (today's behaviour). `outbox`: open one transaction, insert the
/// row **and** the `event_outbox` row on it, then commit — so a crash can
/// never persist one without the other.
///
/// # Errors
///
/// When the insert (or, for `outbox`, the transaction / outbox insert)
/// fails; the transaction rolls back both writes on any error.
pub async fn create_and_emit(
    db: &DatabaseConnection,
    org: &Organization,
    actor: Option<&str>,
) -> ModelResult<OrgModel> {
    match transport() {
        EventTransport::Memory => {
            let model = OrgModel::create(db, org).await?;
            publish_with_actor(
                EventKind::Created,
                &model.pid.to_string(),
                &model.name,
                actor,
            );
            Ok(model)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let model = OrgModel::create(&txn, org).await?;
            let env = envelope(EventKind::Created, &model.pid.to_string(), &model.name, actor);
            OutboxPublisher.publish(&txn, &env).await?;
            txn.commit().await?;
            Ok(model)
        }
    }
}

/// Replace an organization's payload and emit its `Updated` event,
/// atomically under the active transport (see [`create_and_emit`] for the
/// two paths). Consumes the fetched `model`.
///
/// # Errors
///
/// When the update (or, for `outbox`, the transaction / outbox insert)
/// fails.
pub async fn update_and_emit(
    db: &DatabaseConnection,
    model: OrgModel,
    org: &Organization,
    actor: Option<&str>,
) -> ModelResult<OrgModel> {
    match transport() {
        EventTransport::Memory => {
            let updated = model.into_active_model().update_data(db, org).await?;
            publish_with_actor(
                EventKind::Updated,
                &updated.pid.to_string(),
                &updated.name,
                actor,
            );
            Ok(updated)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let updated = model.into_active_model().update_data(&txn, org).await?;
            let env = envelope(
                EventKind::Updated,
                &updated.pid.to_string(),
                &updated.name,
                actor,
            );
            OutboxPublisher.publish(&txn, &env).await?;
            txn.commit().await?;
            Ok(updated)
        }
    }
}

/// Soft-delete an organization and emit its `Deleted` event, atomically
/// under the active transport. Returns the record's `(pid, name)` — captured
/// before the delete — so the caller can audit and respond.
///
/// # Errors
///
/// When the soft-delete (or, for `outbox`, the transaction / outbox
/// insert) fails.
pub async fn delete_and_emit(
    db: &DatabaseConnection,
    model: OrgModel,
    actor: Option<&str>,
) -> ModelResult<(Uuid, String)> {
    let (pid, name) = (model.pid, model.name.clone());
    match transport() {
        EventTransport::Memory => {
            model.into_active_model().soft_delete(db).await?;
            publish_with_actor(EventKind::Deleted, &pid.to_string(), &name, actor);
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            model.into_active_model().soft_delete(&txn).await?;
            let env = envelope(EventKind::Deleted, &pid.to_string(), &name, actor);
            OutboxPublisher.publish(&txn, &env).await?;
            txn.commit().await?;
        }
    }
    Ok((pid, name))
}

/// Fold a duplicate into a survivor and emit the pair of events (`Merged`
/// on the survivor, `Deleted` on the duplicate), atomically under the
/// active transport. Under `outbox`, both writes and both outbox rows
/// share one transaction. Returns `(merged_survivor, duplicate_pid,
/// duplicate_name)` for the caller's merge-record / audit follow-up.
///
/// # Errors
///
/// When either write (or, for `outbox`, the transaction / outbox inserts)
/// fails; the transaction rolls back the whole merge on any error.
pub async fn merge_and_emit(
    db: &DatabaseConnection,
    main: OrgModel,
    duplicate: OrgModel,
    merged_org: &Organization,
    actor: Option<&str>,
) -> ModelResult<(OrgModel, Uuid, String)> {
    let (dup_pid, dup_name) = (duplicate.pid, duplicate.name.clone());
    match transport() {
        EventTransport::Memory => {
            let merged = main.into_active_model().update_data(db, merged_org).await?;
            duplicate.into_active_model().soft_delete(db).await?;
            publish_with_actor(
                EventKind::Merged,
                &merged.pid.to_string(),
                &merged.name,
                actor,
            );
            publish_with_actor(EventKind::Deleted, &dup_pid.to_string(), &dup_name, actor);
            Ok((merged, dup_pid, dup_name))
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let merged = main
                .into_active_model()
                .update_data(&txn, merged_org)
                .await?;
            duplicate.into_active_model().soft_delete(&txn).await?;
            let merged_env = envelope(
                EventKind::Merged,
                &merged.pid.to_string(),
                &merged.name,
                actor,
            );
            OutboxPublisher.publish(&txn, &merged_env).await?;
            let deleted_env = envelope(EventKind::Deleted, &dup_pid.to_string(), &dup_name, actor);
            OutboxPublisher.publish(&txn, &deleted_env).await?;
            txn.commit().await?;
            Ok((merged, dup_pid, dup_name))
        }
    }
}

/// DB-free pins for the event stream: global round-trip + monotonic
/// `seq`, isolated-publisher behaviour, envelope serde, optional actor,
/// and the frozen `/events/recent` projection shape.
#[cfg(test)]
mod tests {
    use super::*;

    /// The global publish/recent path round-trips through the ring
    /// buffer (retargeted from the original `publish_and_read_back`),
    /// and `seq` is monotonic.
    #[test]
    fn publish_and_read_back() {
        publish(EventKind::Created, "pid-1", "Acme");
        publish(EventKind::Updated, "pid-1", "Acme Inc");
        let events = recent(10);
        assert!(events.len() >= 2);
        let last = events.last().unwrap();
        assert_eq!(last.kind, EventKind::Updated);
        assert_eq!(last.name, "Acme Inc");
        // Sequence numbers are monotonic.
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }

    /// An isolated `InMemoryPublisher` round-trips an envelope to its
    /// `EventView` projection.
    #[test]
    fn in_memory_publisher_publish_then_recent() {
        let pub_ = InMemoryPublisher::new();
        pub_.publish(envelope(
            EventKind::Created,
            "pid-A",
            "Acme",
            Some("user-1"),
        ));
        pub_.publish(envelope(EventKind::Deleted, "pid-A", "Acme", None));
        let views = pub_.recent(10);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].kind, EventKind::Created);
        assert_eq!(views[0].pid, "pid-A");
        assert_eq!(views[1].kind, EventKind::Deleted);
        assert!(views[0].seq < views[1].seq);
    }

    /// The envelope serializes and deserializes losslessly, carrying
    /// `schema_version == 1`.
    #[test]
    fn envelope_serde_round_trip() {
        let env = envelope(EventKind::Updated, "pid-X", "Acme Inc", Some("user-9"));
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["entity"], "organization");
        assert_eq!(json["kind"], "updated");
        assert_eq!(json["actor"], "user-9");
        let back: Envelope = serde_json::from_value(json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert_eq!(back.event_id, env.event_id);
        assert_eq!(back.seq, env.seq);
        assert_eq!(back.actor.as_deref(), Some("user-9"));
    }

    /// `actor` is `None` on the back-compat path and populated otherwise.
    #[test]
    fn actor_is_optional() {
        let with = envelope(EventKind::Created, "pid-1", "Acme", Some("user-7"));
        assert_eq!(with.actor.as_deref(), Some("user-7"));
        let without = envelope(EventKind::Created, "pid-1", "Acme", None);
        assert_eq!(without.actor, None);
    }

    /// The transport selector parses `outbox` (case-insensitively, with
    /// surrounding whitespace), and falls back to `memory` for `memory`,
    /// the empty string, and any unrecognised value (fail-safe to today's
    /// behaviour). The `is_outbox` helper agrees.
    #[test]
    fn transport_parse_defaults_to_memory() {
        assert_eq!(EventTransport::parse("outbox"), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("  OutBox "), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("memory"), EventTransport::Memory);
        assert_eq!(EventTransport::parse(""), EventTransport::Memory);
        assert_eq!(EventTransport::parse("junk"), EventTransport::Memory);
        assert!(EventTransport::parse("outbox").is_outbox());
        assert!(!EventTransport::parse("memory").is_outbox());
    }

    /// The `EventView` projection serializes to EXACTLY the frozen
    /// `/events/recent` shape: keys are `kind, pid, name, seq` and
    /// nothing else (front-end contract).
    #[test]
    fn event_view_projection_is_frozen_shape() {
        let env = envelope(EventKind::Created, "pid-1", "Acme", Some("user-1"));
        let view = EventView::from(&env);
        let json = serde_json::to_value(&view).unwrap();
        let obj = json.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["kind", "name", "pid", "seq"]);
        assert_eq!(obj["kind"], "created");
        assert_eq!(obj["pid"], "pid-1");
        assert_eq!(obj["name"], "Acme");
        assert!(obj["seq"].is_u64());
    }
}
