//! Event streaming for worker lifecycle changes.
//!
//! Every CRUD/merge/link operation publishes a
//! [`WorkerEvent`](crate::streaming::WorkerEvent) so downstream consumers
//! (audit pipelines, projections, other services) can react. The
//! [`EventProducer`](crate::streaming::EventProducer) trait abstracts the
//! transport; the bundled
//! [`InMemoryEventPublisher`](crate::streaming::InMemoryEventPublisher) is the
//! default in-process implementation. The real production broker path is
//! the Phase-3 durable-bus outbox relay (`crate::relay`,
//! `WORKER_EVENT_TRANSPORT=outbox`) — see `AGENTS.md` "Durable event bus
//! relay". An earlier consumer-side stub interface (pre-outbox
//! scaffolding, every method panicking, zero callers) was removed rather
//! than filled in (spec §13 T-16): the relay superseded that shape
//! entirely before it was ever implemented.
//!
//! Events are serde-tagged on `event_type`, so the JSON wire form carries a
//! discriminator field naming the variant.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Result;
use crate::models::Worker;

/// Canonical durable-event-bus envelope + transport selector (Phase 2).
pub mod envelope;
pub mod producer;

pub use envelope::{
    ENTITY, Envelope, EventKind, EventTransport, EventView, SCHEMA_VERSION, transport,
};

/// A lifecycle event emitted when a worker record changes.
///
/// Serialized with an internal `event_type` tag identifying the variant.
/// Use [`timestamp`](WorkerEvent::timestamp) and
/// [`worker_id`](WorkerEvent::worker_id) to read the common fields without
/// matching every variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum WorkerEvent {
    /// A new worker was created; carries the full record.
    Created {
        /// The newly created worker record.
        worker: Worker,
        /// When the event occurred.
        timestamp: DateTime<Utc>,
    },
    /// An existing worker was updated; carries the new record state.
    Updated {
        /// The updated worker record.
        worker: Worker,
        /// When the event occurred.
        timestamp: DateTime<Utc>,
    },
    /// A worker was (soft) deleted; carries only the affected ID.
    Deleted {
        /// The ID of the deleted worker.
        worker_id: Uuid,
        /// When the event occurred.
        timestamp: DateTime<Utc>,
    },
    /// Two workers were merged: `source_id` was merged into `target_id`.
    Merged {
        /// The ID of the worker that was merged away (the duplicate).
        source_id: Uuid,
        /// The ID of the surviving worker (the main record).
        target_id: Uuid,
        /// When the event occurred.
        timestamp: DateTime<Utc>,
    },
    /// A link was created from `worker_id` to `linked_id`.
    Linked {
        /// The originating worker ID.
        worker_id: Uuid,
        /// The worker ID that was linked to.
        linked_id: Uuid,
        /// When the event occurred.
        timestamp: DateTime<Utc>,
    },
    /// A link from `worker_id` to `unlinked_id` was removed.
    Unlinked {
        /// The originating worker ID.
        worker_id: Uuid,
        /// The worker ID that was unlinked.
        unlinked_id: Uuid,
        /// When the event occurred.
        timestamp: DateTime<Utc>,
    },
}

impl WorkerEvent {
    /// Returns the event's timestamp regardless of variant.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            WorkerEvent::Created { timestamp, .. }
            | WorkerEvent::Updated { timestamp, .. }
            | WorkerEvent::Deleted { timestamp, .. }
            | WorkerEvent::Merged { timestamp, .. }
            | WorkerEvent::Linked { timestamp, .. }
            | WorkerEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the primary worker ID involved in the event. For `Merged` this
    /// is the merge source; for `Linked`/`Unlinked` it is the originating
    /// worker.
    #[must_use]
    pub fn worker_id(&self) -> Uuid {
        match self {
            WorkerEvent::Created { worker, .. } | WorkerEvent::Updated { worker, .. } => worker.id,
            WorkerEvent::Deleted { worker_id, .. }
            | WorkerEvent::Linked { worker_id, .. }
            | WorkerEvent::Unlinked { worker_id, .. } => *worker_id,
            WorkerEvent::Merged { source_id, .. } => *source_id,
        }
    }
}

/// Publishing side of the event stream. Implementations deliver a
/// [`WorkerEvent`] to the configured transport. Must be `Send + Sync` so it
/// can live in shared application state behind an `Arc`.
pub trait EventProducer: Send + Sync {
    /// Publishes a single worker event.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying transport fails to deliver the
    /// event. The in-memory implementation never fails.
    fn publish(&self, event: WorkerEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;
