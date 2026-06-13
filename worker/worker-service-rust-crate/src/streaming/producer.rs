//! Concrete [`EventProducer`] implementations.
//!
//! [`InMemoryEventPublisher`] buffers events in a shared `Vec` for
//! development and tests (and lets tests assert on what was published).
//! [`FluvioProducer`] is the production transport placeholder. Both implement
//! the [`EventProducer`] trait from the parent module.

use super::{EventProducer, WorkerEvent};
use crate::Result;
use std::sync::{Arc, Mutex};

/// An [`EventProducer`] that records events in an in-memory buffer.
///
/// Intended for development and testing; in production this is swapped for a
/// durable broker (Kafka, NATS, or Fluvio). Cheaply cloneable — clones share
/// the same underlying buffer via `Arc<Mutex<…>>`.
#[derive(Clone)]
pub struct InMemoryEventPublisher {
    /// Shared, mutex-guarded buffer of every published event.
    events: Arc<Mutex<Vec<WorkerEvent>>>,
}

impl InMemoryEventPublisher {
    /// Creates a publisher with an empty event buffer.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a clone of all events published so far (test helper).
    pub fn get_events(&self) -> Vec<WorkerEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Empties the event buffer (test helper).
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Returns the number of buffered events.
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
    /// Logs the event at `info` level and appends it to the in-memory buffer.
    /// Never fails.
    fn publish(&self, event: WorkerEvent) -> Result<()> {
        tracing::info!(
            "Publishing event: {} for worker {}",
            match &event {
                WorkerEvent::Created { .. } => "Created",
                WorkerEvent::Updated { .. } => "Updated",
                WorkerEvent::Deleted { .. } => "Deleted",
                WorkerEvent::Merged { .. } => "Merged",
                WorkerEvent::Linked { .. } => "Linked",
                WorkerEvent::Unlinked { .. } => "Unlinked",
            },
            event.worker_id()
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
    /// Not yet implemented — panics via `todo!`. Wire up the Fluvio client
    /// before using this producer in production.
    fn publish(&self, _event: WorkerEvent) -> Result<()> {
        // TODO: Implement Fluvio event publishing
        todo!("Implement Fluvio event publishing")
    }
}
