//! Event streaming with Fluvio

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::Worker;
use crate::Result;

pub mod producer;
pub mod consumer;

/// Worker event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum WorkerEvent {
    Created { worker: Worker, timestamp: DateTime<Utc> },
    Updated { worker: Worker, timestamp: DateTime<Utc> },
    Deleted { worker_id: Uuid, timestamp: DateTime<Utc> },
    Merged { source_id: Uuid, target_id: Uuid, timestamp: DateTime<Utc> },
    Linked { worker_id: Uuid, linked_id: Uuid, timestamp: DateTime<Utc> },
    Unlinked { worker_id: Uuid, unlinked_id: Uuid, timestamp: DateTime<Utc> },
}

impl WorkerEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            WorkerEvent::Created { timestamp, .. } => *timestamp,
            WorkerEvent::Updated { timestamp, .. } => *timestamp,
            WorkerEvent::Deleted { timestamp, .. } => *timestamp,
            WorkerEvent::Merged { timestamp, .. } => *timestamp,
            WorkerEvent::Linked { timestamp, .. } => *timestamp,
            WorkerEvent::Unlinked { timestamp, .. } => *timestamp,
        }
    }

    /// Get the worker ID involved in the event
    pub fn worker_id(&self) -> Uuid {
        match self {
            WorkerEvent::Created { worker, .. } => worker.id,
            WorkerEvent::Updated { worker, .. } => worker.id,
            WorkerEvent::Deleted { worker_id, .. } => *worker_id,
            WorkerEvent::Merged { source_id, .. } => *source_id,
            WorkerEvent::Linked { worker_id, .. } => *worker_id,
            WorkerEvent::Unlinked { worker_id, .. } => *worker_id,
        }
    }
}

/// Event producer trait
pub trait EventProducer: Send + Sync {
    /// Publish a worker event
    fn publish(&self, event: WorkerEvent) -> Result<()>;
}

pub use producer::InMemoryEventPublisher;

/// Event consumer trait
pub trait EventConsumer {
    /// Subscribe to worker events
    fn subscribe(&mut self) -> Result<()>;

    /// Process the next event
    fn next_event(&mut self) -> Result<Option<WorkerEvent>>;
}
