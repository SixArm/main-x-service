//! Event streaming with Fluvio

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::Patient;
use crate::Result;

pub mod producer;
pub mod consumer;

/// Patient event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum PatientEvent {
    Created { patient: Patient, timestamp: DateTime<Utc> },
    Updated { patient: Patient, timestamp: DateTime<Utc> },
    Deleted { patient_id: Uuid, timestamp: DateTime<Utc> },
    Merged { source_id: Uuid, target_id: Uuid, timestamp: DateTime<Utc> },
    Linked { patient_id: Uuid, linked_id: Uuid, timestamp: DateTime<Utc> },
    Unlinked { patient_id: Uuid, unlinked_id: Uuid, timestamp: DateTime<Utc> },
}

impl PatientEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            PatientEvent::Created { timestamp, .. } => *timestamp,
            PatientEvent::Updated { timestamp, .. } => *timestamp,
            PatientEvent::Deleted { timestamp, .. } => *timestamp,
            PatientEvent::Merged { timestamp, .. } => *timestamp,
            PatientEvent::Linked { timestamp, .. } => *timestamp,
            PatientEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Get the patient ID involved in the event
    pub fn patient_id(&self) -> Uuid {
        match self {
            PatientEvent::Created { patient, .. } => patient.id,
            PatientEvent::Updated { patient, .. } => patient.id,
            PatientEvent::Deleted { patient_id, .. } => *patient_id,
            PatientEvent::Merged { source_id, .. } => *source_id,
            PatientEvent::Linked { patient_id, .. } => *patient_id,
            PatientEvent::Unlinked { patient_id, .. } => *patient_id,
        }
    }
}

/// Event producer trait
pub trait EventProducer: Send + Sync {
    /// Publish a patient event
    fn publish(&self, event: PatientEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Event consumer trait
pub trait EventConsumer {
    /// Subscribe to patient events
    fn subscribe(&mut self) -> Result<()>;

    /// Process the next event
    fn next_event(&mut self) -> Result<Option<PatientEvent>>;
}
