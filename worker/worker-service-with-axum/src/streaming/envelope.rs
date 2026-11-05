//! The canonical, versioned event **envelope** for the durable event bus
//! (`agents/share/event-bus.md` §4) plus the transport selector (§7).
//!
//! This sits alongside — and does **not** replace — the legacy
//! [`WorkerEvent`](super::WorkerEvent) / [`EventProducer`](super::EventProducer)
//! in-memory ring buffer. The envelope is the shape that is persisted to
//! `event_outbox` (Phase 2) and later relayed to Fluvio (Phase 3); the
//! ring buffer stays exactly as before for the `memory` transport, so
//! shipping this module is behaviour-neutral until
//! `WORKER_EVENT_TRANSPORT=outbox` is set.
//!
//! [`Envelope::for_event`] builds the envelope from a domain
//! [`Worker`](crate::models::Worker); [`EventView`] is the flat operator
//! projection; [`EventTransport`] / [`transport`] read the configured
//! transport once from the environment.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Worker;

/// The entity this service publishes events for.
pub const ENTITY: &str = "worker";

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
    /// A cross-service outbound edge was asserted (LNK-1;
    /// `cross-service-linking.md` §4.2). The edge detail rides in
    /// [`Envelope::data`].
    Linked,
    /// A cross-service outbound edge was withdrawn (LNK-1).
    Unlinked,
}

impl EventKind {
    /// The wire token for this kind (the envelope's lowercase serde form).
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            EventKind::Created => "created",
            EventKind::Updated => "updated",
            EventKind::Deleted => "deleted",
            EventKind::Merged => "merged",
            EventKind::Linked => "linked",
            EventKind::Unlinked => "unlinked",
        }
    }
}

/// The canonical, versioned event envelope (event-bus.md §4).
///
/// One shape for every entity and every transport. The wall-clock
/// `occurred_at` and the full record `data` snapshot live on the durable
/// outbox row rather than in this envelope, keeping the projection small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Idempotency / dedup key, end to end (UUID v4).
    pub event_id: Uuid,
    /// Envelope schema version (see [`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// The entity name (`snake_case`), e.g. `"worker"`.
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
    /// For a `Merged` event, the absorbed duplicate's pid; `None`
    /// otherwise. Merge-repointing consumers key on this to move edges
    /// from the duplicate onto the survivor.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub merged_from: Option<String>,
    /// For a `Linked` / `Unlinked` event (LNK-1), the §4.2 edge detail
    /// (`edge_id` / `from_ref` / `to_ref` / `edge_kind` / `role` /
    /// `confidence` / `provenance` / `valid_from` / `valid_to`) the
    /// link-graph aggregator deserialises into its `LinkedEvent`; `None`
    /// for CRUD/merge events. Additive and `skip_serializing_if` so the
    /// existing CRUD/merge wire shape is byte-identical.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<serde_json::Value>,
}

/// The fixed entity name, used as the deserialize default for
/// [`Envelope::entity`] (a `&'static str` cannot borrow from input).
fn default_entity() -> &'static str {
    ENTITY
}

/// Process-wide monotonic event sequence, mirroring the per-process `seq`
/// the family's other services carry. Not durable — the outbox `id`
/// (`BIGSERIAL`) is the authoritative global order.
fn next_seq() -> u64 {
    static SEQ: AtomicU64 = AtomicU64::new(1);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

impl Envelope {
    /// Build an envelope describing `kind` applied to `worker`, minting a
    /// fresh `event_id` and a per-process `seq`. `actor` is left `None`
    /// (the repository write path does not currently thread the bearer
    /// `sub`); sourcing it is a follow-up.
    #[must_use]
    pub fn for_event(worker: &Worker, kind: EventKind) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            schema_version: SCHEMA_VERSION,
            entity: ENTITY,
            kind,
            pid: worker.id.to_string(),
            seq: next_seq(),
            actor: None,
            name: worker.full_name(),
            merged_from: None,
            data: None,
        }
    }

    /// Build a [`EventKind::Merged`] envelope for the surviving `survivor`
    /// record, carrying the absorbed `merged_from` duplicate's pid.
    /// Identical to [`Self::for_event(survivor, EventKind::Merged)`](Self::for_event)
    /// but with `merged_from` populated, so merge-repointing consumers can
    /// move edges off the duplicate.
    #[must_use]
    pub fn for_merge(survivor: &Worker, merged_from: &Uuid) -> Self {
        Self {
            merged_from: Some(merged_from.to_string()),
            ..Self::for_event(survivor, EventKind::Merged)
        }
    }

    /// Build a `Linked` / `Unlinked` cross-service link envelope (LNK-1):
    /// the envelope `pid` is the originating worker (the edge's `from`
    /// side), `name` its denormalised label, `actor` the causing bearer
    /// `sub` (when known), and `data` the §4.2 edge detail the aggregator
    /// consumes. `kind` must be [`EventKind::Linked`] or
    /// [`EventKind::Unlinked`].
    #[must_use]
    pub fn for_link(
        from_pid: &str,
        name: &str,
        actor: Option<&str>,
        kind: EventKind,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            schema_version: SCHEMA_VERSION,
            entity: ENTITY,
            kind,
            pid: from_pid.to_string(),
            seq: next_seq(),
            actor: actor.map(str::to_string),
            name: name.to_string(),
            merged_from: None,
            data: Some(data),
        }
    }
}

/// The flat operator-endpoint projection of an [`Envelope`] (`{kind, pid,
/// name, seq}`). Kept stable so a future `/events/recent` surface — and
/// any downstream consumer — sees one frozen shape.
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
/// shape, dropping the internal envelope fields.
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

/// Which event transport is active (event-bus.md §7). `memory` is the
/// default and today's behaviour; `outbox` durably enqueues each event on
/// the entity mutation's transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTransport {
    /// Process-wide ring buffer; no DB, no transaction (default).
    Memory,
    /// Transactional outbox: an `event_outbox` row per event, written on
    /// the entity mutation's transaction (Phase 2).
    Outbox,
}

impl EventTransport {
    /// Parse the `WORKER_EVENT_TRANSPORT` value. `outbox` selects the
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

/// The process-wide transport, read once from `WORKER_EVENT_TRANSPORT` and
/// cached (mirrors the auth `REQUIRE_AUTH` env pattern). Unset or
/// unrecognised ⇒ [`EventTransport::Memory`].
#[must_use]
pub fn transport() -> EventTransport {
    static TRANSPORT: OnceLock<EventTransport> = OnceLock::new();
    *TRANSPORT.get_or_init(|| {
        std::env::var("WORKER_EVENT_TRANSPORT")
            .map_or(EventTransport::Memory, |v| EventTransport::parse(&v))
    })
}

#[cfg(test)]
mod tests {
    use super::{ENTITY, EventKind, EventTransport, EventView, SCHEMA_VERSION, transport};
    use crate::models::{Gender, HumanName, Worker};

    fn a_worker() -> Worker {
        let name = HumanName {
            use_type: None,
            family: "Smith".into(),
            given: vec!["John".into()],
            prefix: vec![],
            suffix: vec![],
        };
        Worker::new(name, Gender::Male)
    }

    #[test]
    fn for_event_populates_the_envelope_from_the_record() {
        let worker = a_worker();
        let env = super::Envelope::for_event(&worker, EventKind::Created);
        assert_eq!(env.entity, ENTITY);
        assert_eq!(env.schema_version, SCHEMA_VERSION);
        assert_eq!(env.kind, EventKind::Created);
        assert_eq!(env.pid, worker.id.to_string());
        assert_eq!(env.name, worker.full_name());
        assert!(env.actor.is_none());
    }

    #[test]
    fn for_merge_sets_kind_merged_and_merged_from() {
        let survivor = a_worker();
        let dup = uuid::Uuid::new_v4();
        let env = super::Envelope::for_merge(&survivor, &dup);
        assert_eq!(env.kind, EventKind::Merged);
        assert_eq!(env.merged_from, Some(dup.to_string()));
        assert_eq!(env.pid, survivor.id.to_string());
    }

    #[test]
    fn for_event_leaves_merged_from_none() {
        let env = super::Envelope::for_event(&a_worker(), EventKind::Updated);
        assert!(env.merged_from.is_none());
    }

    #[test]
    fn seq_is_monotonic_across_envelopes() {
        let worker = a_worker();
        let a = super::Envelope::for_event(&worker, EventKind::Created);
        let b = super::Envelope::for_event(&worker, EventKind::Updated);
        assert!(b.seq > a.seq, "seq must strictly increase per process");
    }

    #[test]
    fn event_view_projects_the_frozen_shape() {
        let worker = a_worker();
        let env = super::Envelope::for_event(&worker, EventKind::Deleted);
        let view = EventView::from(&env);
        assert_eq!(view.kind, EventKind::Deleted);
        assert_eq!(view.pid, env.pid);
        assert_eq!(view.name, env.name);
        assert_eq!(view.seq, env.seq);
    }

    #[test]
    fn kind_tokens_match_the_serde_form() {
        assert_eq!(EventKind::Created.token(), "created");
        assert_eq!(EventKind::Updated.token(), "updated");
        assert_eq!(EventKind::Deleted.token(), "deleted");
        assert_eq!(EventKind::Merged.token(), "merged");
        assert_eq!(EventKind::Linked.token(), "linked");
        assert_eq!(EventKind::Unlinked.token(), "unlinked");
    }

    /// LNK-1: a CRUD/merge envelope has no `data`, and `skip_serializing_if`
    /// omits the key entirely, so the existing wire shape is byte-identical.
    #[test]
    fn crud_envelope_omits_data_on_the_wire() {
        let env = super::Envelope::for_event(&a_worker(), EventKind::Created);
        assert!(env.data.is_none());
        let json = serde_json::to_value(&env).unwrap();
        assert!(
            json.get("data").is_none(),
            "the `data` key must be absent for a CRUD event"
        );
    }

    /// LNK-1: a `for_link` envelope carries the §4.2 edge detail in `data`
    /// and uses the link kind — the shape the aggregator's `LinkedEvent`
    /// deserialises (the cross-service seam).
    #[test]
    fn for_link_carries_edge_detail_data() {
        let edge = serde_json::json!({
            "edge_id": "0c4f1e2a-0000-4000-8000-000000000010",
            "from_ref": "worker:0c4f1e2a-0000-4000-8000-000000000001",
            "to_ref": "person:0c4f1e2a-0000-4000-8000-000000000002",
            "edge_kind": "same_identity",
            "role": null, "confidence": 1.0, "provenance": "operator",
            "valid_from": null, "valid_to": null
        });
        let env = super::Envelope::for_link(
            "0c4f1e2a-0000-4000-8000-000000000001",
            "John Smith",
            Some("11111111-1111-1111-1111-111111111111"),
            EventKind::Linked,
            edge.clone(),
        );
        assert_eq!(env.kind, EventKind::Linked);
        assert_eq!(env.pid, "0c4f1e2a-0000-4000-8000-000000000001");
        assert_eq!(
            env.actor.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
        // The edge detail round-trips through the envelope's serialized form.
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["kind"], "linked");
        assert_eq!(json["data"]["edge_kind"], "same_identity");
        assert_eq!(
            json["data"]["to_ref"],
            "person:0c4f1e2a-0000-4000-8000-000000000002"
        );
    }

    #[test]
    fn transport_parse_defaults_to_memory() {
        assert_eq!(EventTransport::parse("outbox"), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("OUTBOX"), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("  outbox  "), EventTransport::Outbox);
        assert_eq!(EventTransport::parse("memory"), EventTransport::Memory);
        assert_eq!(EventTransport::parse("nonsense"), EventTransport::Memory);
        assert_eq!(EventTransport::parse(""), EventTransport::Memory);
    }

    #[test]
    fn is_outbox_reflects_the_variant() {
        assert!(EventTransport::Outbox.is_outbox());
        assert!(!EventTransport::Memory.is_outbox());
    }

    #[test]
    fn transport_reads_once_and_is_stable() {
        // Whatever the ambient env, the cached read is internally
        // consistent (a valid variant, and stable across calls).
        let first = transport();
        assert_eq!(first, transport());
    }
}
