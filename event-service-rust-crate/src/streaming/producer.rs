//! [`EventProducer`](crate::streaming::EventProducer) implementations.
//!
//! [`InMemoryEventPublisher`](crate::streaming::producer::InMemoryEventPublisher)
//! keeps published events in a `Mutex<Vec>` for development and tests;
//! [`FluvioProducer`](crate::streaming::producer::FluvioProducer) is the
//! production placeholder backed by Fluvio.

use std::sync::{Arc, Mutex};
use super::{EventProducer, EventEvent};
use crate::Result;

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
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a clone of every published event (test helper).
    pub fn get_events(&self) -> Vec<EventEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Drop all buffered events (test helper).
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Count of buffered events (test helper).
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

/// Production [`EventProducer`] backed by Fluvio (not yet implemented).
pub struct FluvioProducer {
    // Fluvio producer will be initialized here
}

impl EventProducer for FluvioProducer {
    /// Publish to Fluvio. Currently unimplemented (`todo!`).
    fn publish(&self, _event: EventEvent) -> Result<()> {
        // TODO: Implement Fluvio event publishing
        todo!("Implement Fluvio event publishing")
    }
}
