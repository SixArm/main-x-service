//! Event streaming (CRUD/merge/link notifications).
//!
//! Every state-changing operation publishes an
//! [`EventEvent`](crate::streaming::EventEvent) onto a stream so
//! downstream consumers (indexers, caches, analytics) can react.
//! Publishing goes through the
//! [`EventProducer`](crate::streaming::EventProducer) trait (default
//! impl: [`InMemoryEventPublisher`](crate::streaming::producer::InMemoryEventPublisher));
//! consumption goes through
//! [`EventConsumer`](crate::streaming::EventConsumer). The backend is
//! intended to be Fluvio in production but is abstracted behind the
//! traits.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::Event;
use crate::Result;

/// Producer implementations (in-memory publisher).
pub mod producer;
/// Consumer implementations (stub).
pub mod consumer;

/// A streamed domain event describing one change to an event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum EventEvent {
    /// An event record was created.
    Created {
        /// The newly created event.
        event: Event,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// An event record was updated.
    Updated {
        /// The updated event.
        event: Event,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// An event record was (soft) deleted.
    Deleted {
        /// Id of the deleted event.
        event_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// Two records were merged.
    Merged {
        /// Id of the source (absorbed) event.
        source_id: Uuid,
        /// Id of the surviving target event.
        target_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// A link was created between two events.
    Linked {
        /// Id of the event a link was added to.
        event_id: Uuid,
        /// Id of the newly linked event.
        linked_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
    /// A link between two events was removed.
    Unlinked {
        /// Id of the event a link was removed from.
        event_id: Uuid,
        /// Id of the unlinked event.
        unlinked_id: Uuid,
        /// When the change occurred.
        timestamp: DateTime<Utc>,
    },
}

impl EventEvent {
    /// Return the timestamp carried by any variant.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            EventEvent::Created { timestamp, .. } => *timestamp,
            EventEvent::Updated { timestamp, .. } => *timestamp,
            EventEvent::Deleted { timestamp, .. } => *timestamp,
            EventEvent::Merged { timestamp, .. } => *timestamp,
            EventEvent::Linked { timestamp, .. } => *timestamp,
            EventEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Return the primary event id involved (for `Merged`, the source id).
    pub fn event_id(&self) -> Uuid {
        match self {
            EventEvent::Created { event, .. } => event.id,
            EventEvent::Updated { event, .. } => event.id,
            EventEvent::Deleted { event_id, .. } => *event_id,
            EventEvent::Merged { source_id, .. } => *source_id,
            EventEvent::Linked { event_id, .. } => *event_id,
            EventEvent::Unlinked { event_id, .. } => *event_id,
        }
    }
}

/// Strategy trait for publishing [`EventEvent`]s. Object-safe so it can
/// be held as `Arc<dyn EventProducer>` in shared application state.
pub trait EventProducer: Send + Sync {
    /// Publish one event onto the stream.
    fn publish(&self, event: EventEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Strategy trait for consuming [`EventEvent`]s from the stream.
pub trait EventConsumer {
    /// Begin a subscription to the stream.
    fn subscribe(&mut self) -> Result<()>;

    /// Pull the next event, or `None` when none is currently available.
    fn next_event(&mut self) -> Result<Option<EventEvent>>;
}
