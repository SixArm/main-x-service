//! Event streaming for worker lifecycle changes.
//!
//! Every CRUD/merge/link operation publishes a
//! [`WorkerEvent`](crate::streaming::WorkerEvent) so downstream consumers
//! (audit pipelines, projections, other services) can react. The
//! [`EventProducer`](crate::streaming::EventProducer) trait abstracts the
//! transport; the bundled
//! [`InMemoryEventPublisher`](crate::streaming::InMemoryEventPublisher) is the
//! default in-process implementation, with Fluvio intended as the production
//! broker. [`EventConsumer`](crate::streaming::EventConsumer) is the (stubbed)
//! read side.
//!
//! Events are serde-tagged on `event_type`, so the JSON wire form carries a
//! discriminator field naming the variant.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use jiff::Timestamp;

use crate::models::Worker;
use crate::Result;

pub mod producer;
pub mod consumer;

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
        timestamp: Timestamp,
    },
    /// An existing worker was updated; carries the new record state.
    Updated {
        /// The updated worker record.
        worker: Worker,
        /// When the event occurred.
        timestamp: Timestamp,
    },
    /// A worker was (soft) deleted; carries only the affected ID.
    Deleted {
        /// The ID of the deleted worker.
        worker_id: Uuid,
        /// When the event occurred.
        timestamp: Timestamp,
    },
    /// Two workers were merged: `source_id` was merged into `target_id`.
    Merged {
        /// The ID of the worker that was merged away (the duplicate).
        source_id: Uuid,
        /// The ID of the surviving worker (the main record).
        target_id: Uuid,
        /// When the event occurred.
        timestamp: Timestamp,
    },
    /// A link was created from `worker_id` to `linked_id`.
    Linked {
        /// The originating worker ID.
        worker_id: Uuid,
        /// The worker ID that was linked to.
        linked_id: Uuid,
        /// When the event occurred.
        timestamp: Timestamp,
    },
    /// A link from `worker_id` to `unlinked_id` was removed.
    Unlinked {
        /// The originating worker ID.
        worker_id: Uuid,
        /// The worker ID that was unlinked.
        unlinked_id: Uuid,
        /// When the event occurred.
        timestamp: Timestamp,
    },
}

impl WorkerEvent {
    /// Returns the event's timestamp regardless of variant.
    pub fn timestamp(&self) -> Timestamp {
        match self {
            WorkerEvent::Created { timestamp, .. } => *timestamp,
            WorkerEvent::Updated { timestamp, .. } => *timestamp,
            WorkerEvent::Deleted { timestamp, .. } => *timestamp,
            WorkerEvent::Merged { timestamp, .. } => *timestamp,
            WorkerEvent::Linked { timestamp, .. } => *timestamp,
            WorkerEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the primary worker ID involved in the event. For `Merged` this
    /// is the merge source; for `Linked`/`Unlinked` it is the originating
    /// worker.
    pub fn worker_id(&self) -> Uuid {
        match self {
            WorkerEvent::Created { worker, .. } => worker.id,
            WorkerEvent::Updated { worker, .. } => worker.id,
            WorkerEvent::Deleted { worker_id, .. } => *worker_id,
            WorkerEvent::Merged { source_id, .. } => *source_id,
            WorkerEvent::Linked { worker_id, .. } => *worker_id,
            WorkerEvent::Unlinked { worker_id, .. } => *worker_id,
        }
    }
}

/// Publishing side of the event stream. Implementations deliver a
/// [`WorkerEvent`] to the configured transport. Must be `Send + Sync` so it
/// can live in shared application state behind an `Arc`.
pub trait EventProducer: Send + Sync {
    /// Publishes a single worker event, returning an error if delivery fails.
    fn publish(&self, event: WorkerEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Consuming side of the event stream (currently a stub interface).
pub trait EventConsumer {
    /// Begins a subscription to the worker event topic.
    fn subscribe(&mut self) -> Result<()>;

    /// Returns the next available event, or `None` when none is pending.
    fn next_event(&mut self) -> Result<Option<WorkerEvent>>;
}
