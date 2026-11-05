//! Event-streaming publisher.
//!
//! Every CRUD/merge operation on a Thing emits an event. The MVP keeps an
//! in-memory `Vec` so tests can observe; a Fluvio adapter is planned under
//! a feature flag.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Canonical durable-bus envelope + transport selector (event-bus.md
/// §4/§7). Sits alongside the legacy [`ThingEvent`] in-memory buffer.
/// Its `EventKind` (`Created`/…) is deliberately **not** re-exported here
/// (it would clash with the legacy [`EventKind`] below); reach it as
/// `crate::streaming::envelope::EventKind`.
pub mod envelope;

pub use envelope::{ENTITY, Envelope, EventTransport, EventView, SCHEMA_VERSION, transport};

/// One event in the Thing stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThingEvent {
    /// Unique event id.
    pub id: Uuid,
    /// CRUD operation discriminator.
    pub kind: EventKind,
    /// The thing the event refers to.
    pub entity_id: Uuid,
    /// Operation-specific JSON payload captured by the handler.
    pub payload: serde_json::Value,
    /// When the event was emitted.
    pub emitted_at: DateTime<Utc>,
}

impl ThingEvent {
    /// Build a thing event of the given kind.
    #[must_use]
    pub fn new(kind: EventKind, thing_id: Uuid, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            entity_id: thing_id,
            payload,
            emitted_at: Utc::now(),
        }
    }
}

/// The CRUD operation a [`ThingEvent`] represents. Serialises in `PascalCase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventKind {
    /// A thing was created.
    ThingCreated,
    /// A thing was updated.
    ThingUpdated,
    /// A thing was (soft-)deleted.
    ThingDeleted,
    /// A thing absorbed a duplicate via merge.
    ThingMerged,
}

/// Object-safe trait so `AppState` can carry `Arc<dyn EventPublisher>`.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish one [`ThingEvent`] to the stream.
    async fn publish(&self, event: ThingEvent) -> crate::Result<()>;
}

/// MVP implementation — captures every event in a `Mutex<Vec<_>>`.
#[derive(Default)]
pub struct InMemoryEventPublisher {
    /// Thread-safe buffer of every event published so far.
    events: Arc<Mutex<Vec<ThingEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create an empty in-memory publisher.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all captured events in publish order.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned by a panic in another
    /// thread while holding the lock.
    #[must_use]
    pub fn events(&self) -> Vec<ThingEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }

    /// Number of events captured so far.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned by a panic in another
    /// thread while holding the lock.
    #[must_use]
    pub fn count(&self) -> usize {
        self.events.lock().expect("events mutex poisoned").len()
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    /// Log the event and append it to the in-memory buffer.
    async fn publish(&self, event: ThingEvent) -> crate::Result<()> {
        tracing::debug!(?event.kind, ?event.entity_id, "thing event emitted");
        self.events
            .lock()
            .map_err(|_| crate::Error::Streaming("events mutex poisoned".into()))?
            .push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A published event is observable via `events()`.
    #[tokio::test]
    async fn publishes_and_observes() {
        let publisher = InMemoryEventPublisher::new();
        let e = ThingEvent::new(
            EventKind::ThingCreated,
            Uuid::new_v4(),
            serde_json::json!({"name": "Widget"}),
        );
        publisher.publish(e).await.unwrap();
        assert_eq!(publisher.count(), 1);
        assert_eq!(publisher.events()[0].kind, EventKind::ThingCreated);
    }

    /// `EventKind` serialises in `PascalCase`.
    #[test]
    fn event_kind_serialises_pascal_case() {
        let s = serde_json::to_string(&EventKind::ThingMerged).unwrap();
        assert_eq!(s, "\"ThingMerged\"");
    }
}
