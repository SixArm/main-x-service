//! Event stream — Phases 1 & 2 of the durable event bus.
//!
//! Every CRUD action publishes a canonical [`Envelope`]. Two transports
//! sit behind the [`EventTransport`] selector (config
//! `CARE_PATHWAY_EVENT_TRANSPORT`, default `memory`; see
//! [`agents/share/event-bus.md`](../../../../agents/share/event-bus.md) §7):
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
//! `occurred_at` and `data` are omitted from the in-memory [`Envelope`]:
//! the design (§4) places them at the outbox stage, where the handler
//! transaction supplies an authoritative timestamp.
//!
//! The operator endpoint `GET /api/care-pathways/events/recent` returns
//! the flat [`EventView`] projection (`{kind, pid, name, seq}`), a frozen
//! wire shape consumed by the front-end — served from the ring buffer
//! (`memory`) or the outbox (`outbox`) via [`recent_events`], identically.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use care_pathway_matcher::CarePathway;
use loco_rs::prelude::ModelResult;
use sea_orm::{ConnectionTrait, DatabaseConnection, IntoActiveModel, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::audit_logs::Model as AuditModel;
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::event_outbox::{Model as OutboxRow, OutboxInsert};

/// Current envelope schema version. Bumped only on a breaking change to
/// the [`Envelope`] shape; additive fields do not bump it.
pub const SCHEMA_VERSION: u32 = 1;

/// The `snake_case` entity name carried by every envelope from this service.
pub const ENTITY: &str = "care_pathway";

/// Deserialize default for [`Envelope::entity`]: a `&'static str` cannot
/// borrow from the input, so on deserialize the field is fixed to
/// [`ENTITY`] (correct — this service only ever emits its own entity).
/// Without this the derived `Deserialize` would only hold for `'static`,
/// breaking the owned `serde_json::from_value` used to project outbox rows.
fn default_entity() -> &'static str {
    ENTITY
}

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

/// The canonical, versioned event envelope (design §4).
///
/// One shape for every entity and transport. Phase 1 omits `occurred_at`
/// and `data`; those arrive at the outbox stage (Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Idempotency / dedup key for consumers (UUID v4).
    pub event_id: Uuid,
    /// Envelope schema version (currently [`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// The `snake_case` entity name, e.g. `"care_pathway"`.
    ///
    /// Serialized verbatim; on deserialize it is fixed to [`ENTITY`] (a
    /// `&'static str` cannot borrow from the input), which is correct
    /// because this service only ever emits its own entity.
    #[serde(skip_deserializing, default = "default_entity")]
    pub entity: &'static str,
    /// The kind of change.
    pub kind: EventKind,
    /// The care pathway's public id.
    pub pid: String,
    /// Monotonic sequence number (per process).
    pub seq: u64,
    /// The user pid from the bearer token, when known.
    pub actor: Option<String>,
    /// The care pathway's name at the time of the event.
    pub name: String,
}

/// Operator-facing projection of an [`Envelope`] (design §4).
///
/// Serializes to the **frozen** `/events/recent` wire shape — exactly the
/// keys `kind`, `pid`, `name`, `seq`, byte-identical to the previous
/// `PathwayEvent`. The front-end recent-activity view depends on this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventView {
    /// The kind of change.
    pub kind: EventKind,
    /// The care pathway's public id.
    pub pid: String,
    /// The care pathway's name at the time of the event.
    pub name: String,
    /// Monotonic sequence number (per process).
    pub seq: u64,
}

impl From<&Envelope> for EventView {
    /// Project an [`Envelope`] down to the frozen operator wire shape,
    /// dropping the internal fields (`event_id`, `schema_version`,
    /// `entity`, `actor`) that the `/events/recent` view never exposes.
    fn from(env: &Envelope) -> Self {
        Self {
            kind: env.kind,
            pid: env.pid.clone(),
            name: env.name.clone(),
            seq: env.seq,
        }
    }
}

/// The publisher seam (design §5).
///
/// In Phase 1 the only implementation is [`InMemoryPublisher`]; Phase 2
/// adds `OutboxPublisher` (in-transaction `event_outbox` insert) behind
/// the same trait. The publish path never silently drops on the durable
/// implementation.
pub trait EventPublisher: Send + Sync {
    /// Publish an event to the stream.
    fn publish(&self, env: Envelope);
    /// Recent events for the operator endpoint, newest last, capped at
    /// `limit` (projection of §4).
    fn recent(&self, limit: usize) -> Vec<EventView>;
}

/// Ring-buffer capacity: the most recent 1000 events are retained;
/// publishing past this evicts the oldest. Bounds memory on a long-lived
/// process while keeping enough history for the operator view.
const CAPACITY: usize = 1000;

/// Today's ring buffer behind the [`EventPublisher`] seam. Default for
/// tests and single-node dev; keeps `cargo test` DB-free.
#[derive(Debug, Default)]
pub struct InMemoryPublisher {
    buffer: Mutex<VecDeque<Envelope>>,
}

impl InMemoryPublisher {
    /// Construct an empty in-memory publisher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }
}

impl EventPublisher for InMemoryPublisher {
    /// Push to the ring buffer. Never fails; if the lock is poisoned the
    /// event is dropped (the audit log is the durable record).
    fn publish(&self, env: Envelope) {
        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() == CAPACITY {
                buf.pop_front();
            }
            buf.push_back(env);
        }
    }

    /// Return the newest `limit` events in chronological order. Takes the
    /// last `limit` from the back of the buffer, then re-reverses so the
    /// result is oldest-to-newest. A poisoned lock yields an empty vec.
    fn recent(&self, limit: usize) -> Vec<EventView> {
        self.buffer.lock().map_or_else(
            |_| Vec::new(),
            |buf| buf.iter().rev().take(limit).rev().map(Into::into).collect(),
        )
    }
}

/// The process-wide publisher global, lazily initialised on first use.
fn publisher() -> &'static InMemoryPublisher {
    static PUBLISHER: OnceLock<InMemoryPublisher> = OnceLock::new();
    PUBLISHER.get_or_init(InMemoryPublisher::new)
}

/// Allocate the next per-process event sequence number (starts at 1).
/// `Relaxed` is sufficient: we only need monotonic uniqueness, not
/// ordering relative to other memory operations.
fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Build a canonical [`Envelope`] for a change, allocating the next
/// per-process sequence number.
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

/// Publish an event to the in-memory stream (actor unknown).
///
/// Back-compatible call surface; equivalent to
/// [`publish_with_actor`] with `actor == None`.
pub fn publish(kind: EventKind, pid: &str, name: &str) {
    publish_with_actor(kind, pid, name, None);
}

/// Publish an event to the in-memory stream, stamping the bearer-token
/// `actor` when the caller is known. Never fails.
pub fn publish_with_actor(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) {
    publisher().publish(envelope(kind, pid, name, actor));
}

/// The most recent events (newest last), capped at `limit`. Returns the
/// frozen [`EventView`] projection consumed by `/events/recent`.
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
    /// Parse the `CARE_PATHWAY_EVENT_TRANSPORT` value. `outbox` selects the
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

/// The process-wide transport, read once from `CARE_PATHWAY_EVENT_TRANSPORT`
/// and cached (mirrors the auth `REQUIRE_AUTH` env pattern). Unset or
/// unrecognised ⇒ [`EventTransport::Memory`].
#[must_use]
pub fn transport() -> EventTransport {
    static TRANSPORT: OnceLock<EventTransport> = OnceLock::new();
    *TRANSPORT.get_or_init(|| {
        std::env::var("CARE_PATHWAY_EVENT_TRANSPORT")
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
    pub async fn recent(
        self,
        db: &DatabaseConnection,
        limit: usize,
    ) -> ModelResult<Vec<EventView>> {
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

/// Best-effort audit on the pooled connection — the `memory`-transport
/// side channel. A failure is logged, never fatal: the request already
/// succeeded and audit is secondary here. (Under `outbox` the audit row
/// is written *inside* the transaction instead — `AuditModel::record(&txn,
/// …)?` — so entity + event + audit commit or roll back together, per
/// `event-bus.md` §3.)
async fn audit_best_effort(
    db: &DatabaseConnection,
    entity_pid: Uuid,
    action: &str,
    actor: Option<&str>,
    snapshot: Option<serde_json::Value>,
) {
    if let Err(err) = AuditModel::record(db, entity_pid, action, actor, snapshot).await {
        tracing::warn!(error = %err, action, "failed to write audit log");
    }
}

/// Create a care pathway and emit its `Created` event, atomically under
/// the active transport. `memory`: insert on `db`, then push to the ring
/// buffer (today's behaviour). `outbox`: open one transaction, insert the
/// row, the `event_outbox` row, **and** the audit row on it, then commit —
/// so a crash can never persist one without the others.
///
/// # Errors
///
/// When the insert (or, for `outbox`, the transaction / outbox / audit
/// insert) fails; the transaction rolls back all writes on any error.
pub async fn create_and_emit(
    db: &DatabaseConnection,
    pathway: &CarePathway,
    actor: Option<&str>,
) -> ModelResult<PathwayModel> {
    match transport() {
        EventTransport::Memory => {
            let model = PathwayModel::create(db, pathway).await?;
            publish_with_actor(
                EventKind::Created,
                &model.pid.to_string(),
                &model.name,
                actor,
            );
            audit_best_effort(db, model.pid, "created", actor, Some(model.data.clone())).await;
            Ok(model)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let model = PathwayModel::create(&txn, pathway).await?;
            let env = envelope(
                EventKind::Created,
                &model.pid.to_string(),
                &model.name,
                actor,
            );
            OutboxPublisher.publish(&txn, &env).await?;
            // Audit joins the same transaction — atomic with entity + event.
            AuditModel::record(&txn, model.pid, "created", actor, Some(model.data.clone())).await?;
            txn.commit().await?;
            Ok(model)
        }
    }
}

/// Replace a care pathway's payload and emit its `Updated` event,
/// atomically under the active transport (see [`create_and_emit`] for the
/// two paths). Consumes the fetched `model`.
///
/// # Errors
///
/// When the update (or, for `outbox`, the transaction / outbox insert)
/// fails.
pub async fn update_and_emit(
    db: &DatabaseConnection,
    model: PathwayModel,
    pathway: &CarePathway,
    actor: Option<&str>,
) -> ModelResult<PathwayModel> {
    match transport() {
        EventTransport::Memory => {
            let updated = model.into_active_model().update_data(db, pathway).await?;
            publish_with_actor(
                EventKind::Updated,
                &updated.pid.to_string(),
                &updated.name,
                actor,
            );
            audit_best_effort(
                db,
                updated.pid,
                "updated",
                actor,
                Some(updated.data.clone()),
            )
            .await;
            Ok(updated)
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let updated = model.into_active_model().update_data(&txn, pathway).await?;
            let env = envelope(
                EventKind::Updated,
                &updated.pid.to_string(),
                &updated.name,
                actor,
            );
            OutboxPublisher.publish(&txn, &env).await?;
            AuditModel::record(
                &txn,
                updated.pid,
                "updated",
                actor,
                Some(updated.data.clone()),
            )
            .await?;
            txn.commit().await?;
            Ok(updated)
        }
    }
}

/// Soft-delete a care pathway and emit its `Deleted` event, atomically
/// under the active transport. Returns the record's `(pid, name)` — captured
/// before the delete — so the caller can audit and respond.
///
/// # Errors
///
/// When the soft-delete (or, for `outbox`, the transaction / outbox
/// insert) fails.
pub async fn delete_and_emit(
    db: &DatabaseConnection,
    model: PathwayModel,
    actor: Option<&str>,
) -> ModelResult<(Uuid, String)> {
    let (pid, name) = (model.pid, model.name.clone());
    match transport() {
        EventTransport::Memory => {
            model.into_active_model().soft_delete(db).await?;
            publish_with_actor(EventKind::Deleted, &pid.to_string(), &name, actor);
            audit_best_effort(db, pid, "deleted", actor, None).await;
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            model.into_active_model().soft_delete(&txn).await?;
            let env = envelope(EventKind::Deleted, &pid.to_string(), &name, actor);
            OutboxPublisher.publish(&txn, &env).await?;
            AuditModel::record(&txn, pid, "deleted", actor, None).await?;
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
    main: PathwayModel,
    duplicate: PathwayModel,
    merged_pathway: &CarePathway,
    actor: Option<&str>,
) -> ModelResult<(PathwayModel, Uuid, String)> {
    let (dup_pid, dup_name) = (duplicate.pid, duplicate.name.clone());
    match transport() {
        EventTransport::Memory => {
            let merged = main
                .into_active_model()
                .update_data(db, merged_pathway)
                .await?;
            duplicate.into_active_model().soft_delete(db).await?;
            publish_with_actor(
                EventKind::Merged,
                &merged.pid.to_string(),
                &merged.name,
                actor,
            );
            publish_with_actor(EventKind::Deleted, &dup_pid.to_string(), &dup_name, actor);
            audit_best_effort(db, merged.pid, "merged", actor, Some(merged.data.clone())).await;
            audit_best_effort(db, dup_pid, "merged_into", actor, None).await;
            Ok((merged, dup_pid, dup_name))
        }
        EventTransport::Outbox => {
            let txn = db.begin().await?;
            let merged = main
                .into_active_model()
                .update_data(&txn, merged_pathway)
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
            // Both audit rows join the same transaction as the merge.
            AuditModel::record(&txn, merged.pid, "merged", actor, Some(merged.data.clone()))
                .await?;
            AuditModel::record(&txn, dup_pid, "merged_into", actor, None).await?;
            txn.commit().await?;
            Ok((merged, dup_pid, dup_name))
        }
    }
}

/// Tests for the Phase-1 in-memory event path: publish/read-back,
/// monotonic sequencing, the envelope's Serde round-trip, and the frozen
/// `EventView` wire shape. All DB-free.
#[cfg(test)]
mod tests {
    use super::*;

    /// The `InMemoryPublisher` round-trips a published envelope to its
    /// projection (retargets the previous `publish_and_read_back`).
    #[test]
    fn publish_and_read_back() {
        publish(EventKind::Created, "pid-1", "Acute Stroke Care Pathway");
        publish(EventKind::Updated, "pid-1", "Acute Stroke Pathway");
        let events = recent(10);
        assert!(events.len() >= 2);
        let last = events.last().unwrap();
        assert_eq!(last.kind, EventKind::Updated);
        assert_eq!(last.name, "Acute Stroke Pathway");
        // Sequence numbers are monotonic.
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }

    /// A fresh `InMemoryPublisher` (not the global) publishes and reads
    /// back its own buffer, independent of process-wide state.
    #[test]
    fn in_memory_publisher_is_self_contained() {
        let pub_ = InMemoryPublisher::new();
        assert!(pub_.recent(10).is_empty());
        pub_.publish(envelope(EventKind::Created, "p", "Name", None));
        let got = pub_.recent(10);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].pid, "p");
        assert_eq!(got[0].name, "Name");
    }

    /// The envelope round-trips through Serde and carries `schema_version` 1.
    ///
    /// `entity` is `&'static str`, so deserialization borrows from a
    /// `'static` JSON buffer (the in-memory transport never deserializes
    /// envelopes; this only guards the wire shape).
    #[test]
    fn envelope_serde_round_trip() {
        let env = envelope(EventKind::Merged, "pid-x", "Stroke", Some("user-7"));
        let json: &'static str = Box::leak(serde_json::to_string(&env).unwrap().into_boxed_str());
        let back: Envelope = serde_json::from_str(json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.entity, ENTITY);
        assert_eq!(back.kind, EventKind::Merged);
        assert_eq!(back.pid, "pid-x");
        assert_eq!(back.name, "Stroke");
        assert_eq!(back.actor.as_deref(), Some("user-7"));
        assert_eq!(back.event_id, env.event_id);
    }

    /// The `EventView` projection serializes to EXACTLY the frozen
    /// `/events/recent` keys: `kind`, `pid`, `name`, `seq` — and nothing
    /// else. The front-end recent-activity view depends on this shape.
    #[test]
    fn event_view_has_exactly_the_frozen_keys() {
        let env = envelope(EventKind::Created, "pid-1", "Acute Stroke", Some("user-1"));
        let view = EventView::from(&env);
        let value = serde_json::to_value(&view).unwrap();
        let obj = value.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["kind", "name", "pid", "seq"]);
        // Frozen lowercase kind rendering.
        assert_eq!(obj["kind"], serde_json::json!("created"));
        assert_eq!(obj["pid"], serde_json::json!("pid-1"));
        assert_eq!(obj["name"], serde_json::json!("Acute Stroke"));
        // The actor is intentionally NOT projected to the operator view.
        assert!(!obj.contains_key("actor"));
    }

    /// `actor` is populated when supplied and `None` via the bare
    /// `publish` surface.
    #[test]
    fn actor_is_populated_or_none() {
        let with = envelope(EventKind::Updated, "p", "n", Some("user-9"));
        assert_eq!(with.actor.as_deref(), Some("user-9"));
        let without = envelope(EventKind::Updated, "p", "n", None);
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

    /// Sequence numbers are monotonic across publishes on one publisher.
    #[test]
    fn seq_is_monotonic() {
        let pub_ = InMemoryPublisher::new();
        for i in 0..5 {
            pub_.publish(envelope(EventKind::Created, &format!("p{i}"), "n", None));
        }
        let events = pub_.recent(10);
        assert_eq!(events.len(), 5);
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }
}
