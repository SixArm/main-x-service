//! Event streaming with Fluvio

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::Person;
use crate::Result;

pub mod producer;
pub mod consumer;

/// Person event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum PersonEvent {
    Created { person: Person, timestamp: DateTime<Utc> },
    Updated { person: Person, timestamp: DateTime<Utc> },
    Deleted { person_id: Uuid, timestamp: DateTime<Utc> },
    Merged { source_id: Uuid, target_id: Uuid, timestamp: DateTime<Utc> },
    Linked { person_id: Uuid, linked_id: Uuid, timestamp: DateTime<Utc> },
    Unlinked { person_id: Uuid, unlinked_id: Uuid, timestamp: DateTime<Utc> },
}

impl PersonEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            PersonEvent::Created { timestamp, .. } => *timestamp,
            PersonEvent::Updated { timestamp, .. } => *timestamp,
            PersonEvent::Deleted { timestamp, .. } => *timestamp,
            PersonEvent::Merged { timestamp, .. } => *timestamp,
            PersonEvent::Linked { timestamp, .. } => *timestamp,
            PersonEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Get the person ID involved in the event
    pub fn person_id(&self) -> Uuid {
        match self {
            PersonEvent::Created { person, .. } => person.id,
            PersonEvent::Updated { person, .. } => person.id,
            PersonEvent::Deleted { person_id, .. } => *person_id,
            PersonEvent::Merged { source_id, .. } => *source_id,
            PersonEvent::Linked { person_id, .. } => *person_id,
            PersonEvent::Unlinked { person_id, .. } => *person_id,
        }
    }
}

/// Event producer trait
pub trait EventProducer: Send + Sync {
    /// Publish a person event
    fn publish(&self, event: PersonEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Event consumer trait
pub trait EventConsumer {
    /// Subscribe to person events
    fn subscribe(&mut self) -> Result<()>;

    /// Process the next event
    fn next_event(&mut self) -> Result<Option<PersonEvent>>;
}
