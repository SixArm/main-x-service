//! In-memory Main Worker Service for unit + integration tests.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, Error, Worker};

#[derive(Default)]
pub struct StubClient {
    workers: Mutex<Vec<Worker>>,
}

impl StubClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, workers: Vec<Worker>) {
        let mut guard = self.workers.lock().unwrap();
        for w in workers {
            if !guard.iter().any(|q| q.id == w.id) {
                guard.push(w);
            }
        }
    }

    pub fn snapshot(&self) -> Vec<Worker> {
        self.workers.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.workers.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
    async fn search(&self, query: &str) -> Result<Vec<Worker>, Error> {
        let q = query.trim().to_lowercase();
        let guard = self.workers.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|w| q.is_empty() || w.name.to_lowercase().contains(&q))
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Worker>, Error> {
        let guard = self.workers.lock().unwrap();
        Ok(guard.iter().find(|w| w.id == id).cloned())
    }
}
