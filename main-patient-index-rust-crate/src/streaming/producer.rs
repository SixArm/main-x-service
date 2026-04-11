//! Event producer implementations

use std::sync::{Arc, Mutex};
use super::{EventProducer, PatientEvent};
use crate::Result;

/// In-memory event publisher for development/testing
/// In production, replace with Kafka, NATS, or Fluvio
#[derive(Clone)]
pub struct InMemoryEventPublisher {
    events: Arc<Mutex<Vec<PatientEvent>>>,
}

impl InMemoryEventPublisher {
    /// Create a new in-memory event publisher
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all published events (for testing)
    pub fn get_events(&self) -> Vec<PatientEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear all events (for testing)
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl Default for InMemoryEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProducer for InMemoryEventPublisher {
    fn publish(&self, event: PatientEvent) -> Result<()> {
        tracing::info!(
            "Publishing event: {} for patient {}",
            match &event {
                PatientEvent::Created { .. } => "Created",
                PatientEvent::Updated { .. } => "Updated",
                PatientEvent::Deleted { .. } => "Deleted",
                PatientEvent::Merged { .. } => "Merged",
                PatientEvent::Linked { .. } => "Linked",
                PatientEvent::Unlinked { .. } => "Unlinked",
            },
            event.patient_id()
        );

        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

/// Fluvio event producer (for production use)
pub struct FluvioProducer {
    // Fluvio producer will be initialized here
}

impl EventProducer for FluvioProducer {
    fn publish(&self, _event: PatientEvent) -> Result<()> {
        // TODO: Implement Fluvio event publishing
        todo!("Implement Fluvio event publishing")
    }
}
