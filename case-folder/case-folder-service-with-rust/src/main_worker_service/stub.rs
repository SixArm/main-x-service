//! In-memory Main Worker Service for unit + integration tests.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, Error, Worker};

/// In-memory test double for the Main Worker Service [`Client`]. Holds
/// workers in a `Mutex`-guarded `Vec`; no network involved.
#[derive(Default)]
pub struct StubClient {
    /// Stored workers, in insertion order.
    workers: Mutex<Vec<Worker>>,
}

impl StubClient {
    /// Creates an empty stub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-loads `workers`, skipping any whose `id` is already present
    /// (dedupe-by-id).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn seed(&self, workers: Vec<Worker>) {
        let mut guard = self.workers.lock().unwrap();
        for w in workers {
            if !guard.iter().any(|q| q.id == w.id) {
                guard.push(w);
            }
        }
    }

    /// Returns a clone of all stored workers (test inspection helper).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn snapshot(&self) -> Vec<Worker> {
        self.workers.lock().unwrap().clone()
    }

    /// Removes all stored workers.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn clear(&self) {
        self.workers.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
    /// Case-insensitive substring name search. Empty query matches all.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn search(&self, query: &str) -> Result<Vec<Worker>, Error> {
        let q = query.trim().to_lowercase();
        let guard = self.workers.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|w| q.is_empty() || w.name.to_lowercase().contains(&q))
            .cloned()
            .collect())
    }

    /// Finds a worker by UUID.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Worker>, Error> {
        let guard = self.workers.lock().unwrap();
        Ok(guard.iter().find(|w| w.id == id).cloned())
    }
}
