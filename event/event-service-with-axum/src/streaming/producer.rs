//! [`EventProducer`](crate::streaming::EventProducer) implementations.
//!
//! [`InMemoryEventPublisher`](crate::streaming::producer::InMemoryEventPublisher)
//! keeps published events in a `Mutex<Vec>` for development and tests.
//! Production delivery to Fluvio is handled by the durable outbox +
//! relay (`src/relay.rs`, `EventSink`/`FluvioSink`) instead of this
//! trait — see spec §13 T-4.

use super::{EventEvent, EventProducer};
use crate::Result;
use std::sync::{Arc, Mutex};

/// In-memory event publisher for development and testing. In production,
/// replace with a durable backend (Fluvio / Kafka / NATS). The buffer is
/// `Arc<Mutex<…>>` so clones share the same event log.
#[derive(Clone)]
pub struct InMemoryEventPublisher {
    /// Shared, in-memory log of every published event.
    events: Arc<Mutex<Vec<EventEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create an empty in-memory publisher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a clone of every published event (test helper).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn get_events(&self) -> Vec<EventEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Drop all buffered events (test helper).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Count of buffered events (test helper).
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl Default for InMemoryEventPublisher {
    /// Same as [`InMemoryEventPublisher::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl EventProducer for InMemoryEventPublisher {
    /// Log the event at `info` level and append it to the buffer.
    fn publish(&self, event: EventEvent) -> Result<()> {
        tracing::info!(
            "Publishing event: {} for event {}",
            match &event {
                EventEvent::Created { .. } => "Created",
                EventEvent::Updated { .. } => "Updated",
                EventEvent::Deleted { .. } => "Deleted",
                EventEvent::Merged { .. } => "Merged",
                EventEvent::Linked { .. } => "Linked",
                EventEvent::Unlinked { .. } => "Unlinked",
            },
            event.event_id()
        );

        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
