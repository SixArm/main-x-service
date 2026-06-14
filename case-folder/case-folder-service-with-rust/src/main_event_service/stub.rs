//! In-memory Main Event Service for unit + integration tests.

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, Error, MoveEvent, RecordMove};

/// In-memory test double for the Main Event Service [`Client`]. Holds
/// recorded moves in a `Mutex`-guarded `Vec`; no network involved.
#[derive(Default)]
pub struct StubClient {
    /// Recorded moves, in insertion order.
    events: Mutex<Vec<MoveEvent>>,
}

impl StubClient {
    /// Creates an empty stub.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-loads `events`, skipping any whose `id` is already present
    /// (dedupe-by-id), so tests can seed a fixed dataset idempotently.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn seed(&self, events: Vec<MoveEvent>) {
        let mut guard = self.events.lock().unwrap();
        for e in events {
            if !guard.iter().any(|g| g.id == e.id) {
                guard.push(e);
            }
        }
    }

    /// Returns a clone of all stored moves (test inspection helper).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn snapshot(&self) -> Vec<MoveEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Removes all stored moves.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

/// Sorts `events` newest-first by `moved_at`, mirroring the HTTP client's
/// `Reverse`-keyed ordering.
fn sort_desc(mut events: Vec<MoveEvent>) -> Vec<MoveEvent> {
    events.sort_by_key(|e| std::cmp::Reverse(e.moved_at));
    events
}

#[async_trait]
impl Client for StubClient {
    /// Assigns a fresh UUID and `moved_at = now`, stores the move, and
    /// returns it — mirroring the real service's create semantics.
    ///
    /// # Errors
    /// Infallible in practice; signature returns `Result` for trait parity.
    async fn record(&self, input: RecordMove) -> Result<MoveEvent, Error> {
        let event = MoveEvent {
            id: Uuid::new_v4(),
            folder_id: input.folder_id,
            patient_id: input.patient_id,
            nhs_number: input.nhs_number,
            patient_name: input.patient_name,
            folder_title: input.folder_title,
            from_cabinet_id: input.from_cabinet_id,
            to_cabinet_id: input.to_cabinet_id,
            from_cabinet_label: input.from_cabinet_label,
            to_cabinet_label: input.to_cabinet_label,
            worker_id: input.worker_id,
            moved_by: input.moved_by,
            worker_role_snapshot: input.worker_role_snapshot,
            moved_at: Utc::now(),
            reason: input.reason,
        };
        self.events.lock().unwrap().push(event.clone());
        Ok(event)
    }

    /// All stored moves, newest first.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_all(&self) -> Result<Vec<MoveEvent>, Error> {
        Ok(sort_desc(self.events.lock().unwrap().clone()))
    }

    /// The most recent `limit` moves (sort then truncate).
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_recent(&self, limit: u32) -> Result<Vec<MoveEvent>, Error> {
        let mut events = sort_desc(self.events.lock().unwrap().clone());
        events.truncate(limit as usize);
        Ok(events)
    }

    /// All moves whose `folder_id` matches, newest first.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_for_folder(&self, folder_id: Uuid) -> Result<Vec<MoveEvent>, Error> {
        let guard = self.events.lock().unwrap();
        Ok(sort_desc(
            guard
                .iter()
                .filter(|e| e.folder_id == folder_id)
                .cloned()
                .collect(),
        ))
    }

    /// All moves whose `patient_id` matches, newest first.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<MoveEvent>, Error> {
        let guard = self.events.lock().unwrap();
        Ok(sort_desc(
            guard
                .iter()
                .filter(|e| e.patient_id == patient_id)
                .cloned()
                .collect(),
        ))
    }
}
