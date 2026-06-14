//! Event-streaming publisher.
//!
//! Every CRUD/merge operation on a Place emits an event. The MVP keeps an
//! in-memory `Vec` so tests can observe; a Fluvio adapter is planned under
//! a feature flag.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One event in the Place stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceEvent {
    /// Unique event id.
    pub id: Uuid,
    /// CRUD operation discriminator.
    pub kind: EventKind,
    /// The place the event refers to.
    pub entity_id: Uuid,
    /// Operation-specific JSON payload captured by the handler.
    pub payload: serde_json::Value,
    /// When the event was emitted.
    pub emitted_at: DateTime<Utc>,
}

impl PlaceEvent {
    /// Build a place event of the given kind.
    pub fn new(kind: EventKind, place_id: Uuid, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            entity_id: place_id,
            payload,
            emitted_at: Utc::now(),
        }
    }
}

/// The CRUD operation a [`PlaceEvent`] represents. Serialises in PascalCase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventKind {
    /// A place was created.
    PlaceCreated,
    /// A place was updated.
    PlaceUpdated,
    /// A place was (soft-)deleted.
    PlaceDeleted,
    /// A place absorbed a duplicate via merge.
    PlaceMerged,
}

/// Object-safe trait so `AppState` can carry `Arc<dyn EventPublisher>`.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish one [`PlaceEvent`] to the stream.
    async fn publish(&self, event: PlaceEvent) -> crate::Result<()>;
}

/// MVP implementation — captures every event in a `Mutex<Vec<_>>`.
#[derive(Default)]
pub struct InMemoryEventPublisher {
    /// Thread-safe buffer of every event published so far.
    events: Arc<Mutex<Vec<PlaceEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create an empty in-memory publisher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all captured events in publish order.
    pub fn events(&self) -> Vec<PlaceEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }

    /// Number of events captured so far.
    pub fn count(&self) -> usize {
        self.events.lock().expect("events mutex poisoned").len()
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventPublisher {
    /// Log the event and append it to the in-memory buffer.
    async fn publish(&self, event: PlaceEvent) -> crate::Result<()> {
        tracing::debug!(?event.kind, ?event.entity_id, "place event emitted");
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
        let e = PlaceEvent::new(
            EventKind::PlaceCreated,
            Uuid::new_v4(),
            serde_json::json!({"name": "Central Park"}),
        );
        publisher.publish(e).await.unwrap();
        assert_eq!(publisher.count(), 1);
        assert_eq!(publisher.events()[0].kind, EventKind::PlaceCreated);
    }

    /// `EventKind` serialises in PascalCase.
    #[test]
    fn event_kind_serialises_pascal_case() {
        let s = serde_json::to_string(&EventKind::PlaceMerged).unwrap();
        assert_eq!(s, "\"PlaceMerged\"");
    }
}
