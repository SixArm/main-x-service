//! In-memory event stream — Phase 1 of the durable event bus.
//!
//! Every CRUD/merge action publishes a canonical [`Envelope`] to a
//! process-wide ring buffer behind the [`EventPublisher`] seam. This is
//! Phase 1 of the family's durable-event-bus design
//! ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md)):
//! the versioned envelope (§4) and the publisher trait (§5), wired to an
//! in-memory implementation that reproduces today's behaviour exactly.
//! Phases 2–3 (transactional outbox → Fluvio) remain infra-gated roadmap.
//!
//! In loco there is no per-request shared state for this, so the buffer
//! is a `OnceLock`-initialised global.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Envelope schema version (§4). Bumped only on a breaking change to the
/// envelope shape; additive fields do not bump it.
pub const SCHEMA_VERSION: u32 = 1;

/// The entity name carried by every envelope (§4).
pub const ENTITY: &str = "work_item";

/// Serde default for [`Envelope::entity`] (it is never read from input).
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

/// The canonical, versioned event envelope (event-bus design §4).
///
/// One shape across every entity and every transport. Phase 1 carries
/// the identity, ordering, and operator-label fields; `occurred_at` and
/// the full-record `data` snapshot arrive at the outbox stage (Phase 2),
/// so they are intentionally absent here (adding them is additive and
/// does not bump [`SCHEMA_VERSION`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Idempotency / dedup key, end to end (UUID v4).
    pub event_id: Uuid,
    /// Envelope schema version; always [`SCHEMA_VERSION`] in Phase 1.
    pub schema_version: u32,
    /// The entity name; always [`ENTITY`] (`work_item`) for this crate.
    /// It is a `&'static str` (the compile-time constant), so on the wire
    /// it serializes verbatim and on the way back in it is filled from
    /// the constant rather than borrowed from the input — a `&'static
    /// str` cannot borrow a runtime string.
    #[serde(skip_deserializing, default = "default_entity")]
    pub entity: &'static str,
    /// The kind of change.
    pub kind: EventKind,
    /// The record's public id.
    pub pid: String,
    /// Per-process monotonic sequence number.
    pub seq: u64,
    /// The user pid from the bearer token, when known.
    pub actor: Option<String>,
    /// The record's denormalised label at the time of the event.
    pub name: String,
}

/// The operator-facing projection of an [`Envelope`] (§4).
///
/// This is the exact wire shape of `GET /api/v1/{collection}/events/recent`: the
/// flat `{kind, pid, name, seq}` an operator sees. It is deliberately a
/// strict subset of the envelope so the durable-bus internals can evolve
/// without changing the operator API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventView {
    /// The kind of change.
    pub kind: EventKind,
    /// The record's public id.
    pub pid: String,
    /// The record's denormalised label at the time of the event.
    pub name: String,
    /// Per-process monotonic sequence number.
    pub seq: u64,
}

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

/// The publisher seam (event-bus design §5).
///
/// Phase 1 ships the synchronous [`InMemoryPublisher`]; the durable
/// `OutboxPublisher` (Phase 2) and the Fluvio relay (Phase 3) slot in
/// behind the same trait without touching call sites.
pub trait EventPublisher: Send + Sync {
    /// Publish an event to the stream. Never fails the caller.
    fn publish(&self, env: Envelope);
    /// The most recent events (newest last), capped at `limit`, as the
    /// operator projection (§4).
    fn recent(&self, limit: usize) -> Vec<EventView>;
}

const CAPACITY: usize = 1000;

/// The in-memory ring-buffer publisher: today's behaviour, behind the
/// [`EventPublisher`] seam. Default for tests and single-node dev.
#[derive(Debug, Default)]
pub struct InMemoryPublisher {
    buffer: Mutex<VecDeque<Envelope>>,
}

impl InMemoryPublisher {
    /// Create an empty publisher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }
}

impl EventPublisher for InMemoryPublisher {
    /// Push an envelope; if the lock is poisoned the event is dropped
    /// (the audit log is the durable record).
    fn publish(&self, env: Envelope) {
        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() == CAPACITY {
                buf.pop_front();
            }
            buf.push_back(env);
        }
    }

    fn recent(&self, limit: usize) -> Vec<EventView> {
        self.buffer.lock().map_or_else(
            |_| Vec::new(),
            |buf| {
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

/// The process-wide publisher (Phase 1: in-memory).
fn publisher() -> &'static InMemoryPublisher {
    static PUB: OnceLock<InMemoryPublisher> = OnceLock::new();
    PUB.get_or_init(InMemoryPublisher::new)
}

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Build an [`Envelope`] for a change, assigning a fresh `event_id` and
/// the next per-process `seq`.
fn envelope(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) -> Envelope {
    Envelope {
        event_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        entity: ENTITY,
        kind,
        pid: pid.to_string(),
        seq: next_seq(),
        actor: actor.map(str::to_string),
        name: name.to_string(),
    }
}

/// Publish an event to the in-memory stream with no known actor.
///
/// Back-compat shim over [`publish_with_actor`]; never fails.
pub fn publish(kind: EventKind, pid: &str, name: &str) {
    publish_with_actor(kind, pid, name, None);
}

/// Publish an event to the in-memory stream, recording the `actor`
/// (the verified caller `sub`) when a bearer token was presented.
/// Never fails; if the lock is poisoned the event is dropped (the audit
/// log is the durable record).
pub fn publish_with_actor(kind: EventKind, pid: &str, name: &str, actor: Option<&str>) {
    publisher().publish(envelope(kind, pid, name, actor));
}

/// The most recent events (newest last), capped at `limit`, as the
/// operator projection consumed by `/events/recent`.
#[must_use]
pub fn recent(limit: usize) -> Vec<EventView> {
    publisher().recent(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the envelope wire contract: schema version is 1, `entity` is
    /// `"work_item"`, and a full serialize→deserialize round-trip preserves
    /// every field (including the `skip_deserializing` `entity`, refilled
    /// from the constant).
    #[test]
    fn envelope_serde_round_trip_and_schema_version() {
        let env = envelope(
            EventKind::Created,
            "pid-1",
            "Housing benefit appeal",
            Some("user-7"),
        );
        assert_eq!(env.schema_version, SCHEMA_VERSION);
        assert_eq!(env.schema_version, 1);
        assert_eq!(env.entity, "work_item");
        let json = serde_json::to_string(&env).expect("serialize envelope");
        let back: Envelope = serde_json::from_str(&json).expect("deserialize envelope");
        assert_eq!(back.event_id, env.event_id);
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.entity, "work_item");
        assert_eq!(back.kind, EventKind::Created);
        assert_eq!(back.pid, "pid-1");
        assert_eq!(back.actor.as_deref(), Some("user-7"));
        assert_eq!(back.name, "Housing benefit appeal");
    }

    /// Pins the frozen operator projection: `EventView` serializes to
    /// exactly `{kind, pid, name, seq}` — the envelope's internal fields
    /// (`event_id`, `schema_version`, `entity`, `actor`) are not exposed.
    #[test]
    fn projection_has_exactly_the_frozen_keys() {
        let env = envelope(
            EventKind::Updated,
            "pid-9",
            "Tax credit overpayment",
            Some("user-1"),
        );
        let view = EventView::from(&env);
        let value = serde_json::to_value(&view).expect("serialize view");
        let map = value.as_object().expect("view is a JSON object");
        let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
        keys.sort_unstable();
        // The wire shape is frozen: exactly kind, pid, name, seq — the
        // envelope's event_id / schema_version / entity / actor are not
        // exposed to operators.
        assert_eq!(keys, ["kind", "name", "pid", "seq"]);
        assert_eq!(value["kind"], "updated");
        assert_eq!(value["pid"], "pid-9");
        assert_eq!(value["name"], "Tax credit overpayment");
    }

    /// Pins the in-memory publisher: published envelopes read back newest
    /// last, projected to `EventView`, with strictly increasing `seq`.
    #[test]
    fn in_memory_publisher_publish_and_read_back() {
        let publisher = InMemoryPublisher::new();
        publisher.publish(envelope(
            EventKind::Created,
            "pid-1",
            "Housing benefit appeal",
            None,
        ));
        publisher.publish(envelope(
            EventKind::Updated,
            "pid-1",
            "Housing benefit appeal (rev 2)",
            None,
        ));
        let events = publisher.recent(10);
        assert!(events.len() >= 2);
        let last = events.last().unwrap();
        assert_eq!(last.kind, EventKind::Updated);
        assert_eq!(last.name, "Housing benefit appeal (rev 2)");
        // Sequence numbers are monotonic.
        assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
    }

    /// Pins actor capture on the envelope: `Some(actor)` when supplied,
    /// `None` otherwise.
    #[test]
    fn actor_is_recorded_or_none() {
        let with = envelope(
            EventKind::Created,
            "pid-1",
            "Housing benefit appeal",
            Some("user-42"),
        );
        assert_eq!(with.actor.as_deref(), Some("user-42"));
        let without = envelope(EventKind::Created, "pid-1", "Housing benefit appeal", None);
        assert_eq!(without.actor, None);
    }

    /// Pins the process-wide free functions (`publish_with_actor` /
    /// `publish` / `recent`): events for a pid come back in monotonic
    /// `seq` order via the global publisher.
    #[test]
    fn process_publish_is_monotonic_and_projects() {
        publish_with_actor(EventKind::Created, "proc-pid", "Initial", Some("user-3"));
        publish(EventKind::Updated, "proc-pid", "Revised");
        let events = recent(100);
        let mine: Vec<&EventView> = events.iter().filter(|e| e.pid == "proc-pid").collect();
        assert!(mine.len() >= 2);
        assert!(mine.windows(2).all(|w| w[0].seq < w[1].seq));
    }
}
