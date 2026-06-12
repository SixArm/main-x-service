//! Concrete [`EventProducer`](crate::streaming::EventProducer) implementations.
//!
//! [`InMemoryEventPublisher`] buffers events in a shared `Vec` for
//! development and testing — it lets tests assert on what was emitted.
//! [`FluvioProducer`](crate::streaming::producer::FluvioProducer) is the production transport stub. Both implement
//! the [`EventProducer`](crate::streaming::EventProducer) trait from the parent module.

use super::{EventProducer, PersonEvent};
use crate::Result;
use std::sync::{Arc, Mutex};

/// An [`EventProducer`] that records events in memory.
///
/// Intended for development and tests; in production swap in a durable
/// transport such as Kafka, NATS, or Fluvio. Cloning is cheap and shares
/// the same buffer (the `Vec` lives behind an `Arc<Mutex<_>>`), so all
/// clones observe the same events.
#[derive(Clone)]
pub struct InMemoryEventPublisher {
    /// Shared, mutex-guarded buffer of every published event.
    events: Arc<Mutex<Vec<PersonEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create a publisher with an empty event buffer.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a snapshot clone of every event published so far.
    pub fn get_events(&self) -> Vec<PersonEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Discard all buffered events (test convenience).
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Return the number of events currently buffered.
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl Default for InMemoryEventPublisher {
    /// Equivalent to [`InMemoryEventPublisher::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl EventProducer for InMemoryEventPublisher {
    /// Log the event at `info` level, then append it to the buffer.
    ///
    /// Always succeeds (returns `Ok`) since the buffer never fails.
    fn publish(&self, event: PersonEvent) -> Result<()> {
        tracing::info!(
            "Publishing event: {} for person {}",
            match &event {
                PersonEvent::Created { .. } => "Created",
                PersonEvent::Updated { .. } => "Updated",
                PersonEvent::Deleted { .. } => "Deleted",
                PersonEvent::Merged { .. } => "Merged",
                PersonEvent::Linked { .. } => "Linked",
                PersonEvent::Unlinked { .. } => "Unlinked",
            },
            event.person_id()
        );

        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

/// Production [`EventProducer`] backed by Fluvio (not yet implemented).
///
/// Placeholder for a durable, partitioned stream. The Fluvio client
/// handle will live in this struct once wired up.
pub struct FluvioProducer {
    // Fluvio producer will be initialized here
}

impl EventProducer for FluvioProducer {
    /// Unimplemented; panics via `todo!` until the Fluvio client lands.
    fn publish(&self, _event: PersonEvent) -> Result<()> {
        // TODO: Implement Fluvio event publishing
        todo!("Implement Fluvio event publishing")
    }
}
