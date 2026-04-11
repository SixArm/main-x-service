//! Event streaming with Fluvio

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::Event;
use crate::Result;

pub mod producer;
pub mod consumer;

/// Event event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum EventEvent {
    Created { event: Event, timestamp: DateTime<Utc> },
    Updated { event: Event, timestamp: DateTime<Utc> },
    Deleted { event_id: Uuid, timestamp: DateTime<Utc> },
    Merged { source_id: Uuid, target_id: Uuid, timestamp: DateTime<Utc> },
    Linked { event_id: Uuid, linked_id: Uuid, timestamp: DateTime<Utc> },
    Unlinked { event_id: Uuid, unlinked_id: Uuid, timestamp: DateTime<Utc> },
}

impl EventEvent {
    /// Get the timestamp of the event
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

    /// Get the event ID involved in the event
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

/// Event producer trait
pub trait EventProducer: Send + Sync {
    /// Publish a event event
    fn publish(&self, event: EventEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Event consumer trait
pub trait EventConsumer {
    /// Subscribe to event events
    fn subscribe(&mut self) -> Result<()>;

    /// Process the next event
    fn next_event(&mut self) -> Result<Option<EventEvent>>;
}
