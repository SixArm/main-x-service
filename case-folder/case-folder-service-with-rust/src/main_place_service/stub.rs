//! In-memory Main Place Service for unit + integration tests.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, CreatePlace, Error, Place};

#[derive(Default)]
pub struct StubClient {
    places: Mutex<Vec<Place>>,
}

impl StubClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, places: Vec<Place>) {
        let mut guard = self.places.lock().unwrap();
        for p in places {
            if !guard.iter().any(|q| q.id == p.id) {
                guard.push(p);
            }
        }
    }

    pub fn snapshot(&self) -> Vec<Place> {
        self.places.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.places.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
    async fn search(&self, query: &str, place_type: Option<&str>) -> Result<Vec<Place>, Error> {
        let q = query.trim().to_lowercase();
        let guard = self.places.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q))
            .filter(|p| place_type.is_none() || p.place_type.as_deref() == place_type)
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Place>, Error> {
        let guard = self.places.lock().unwrap();
        Ok(guard.iter().find(|p| p.id == id).cloned())
    }

    async fn create(&self, input: CreatePlace) -> Result<Place, Error> {
        let place = Place {
            id: Uuid::new_v4(),
            name: input.name,
            place_type: Some(input.place_type),
            description: input.description,
            contained_in_place: input.contained_in_place,
            capacity: input.capacity,
        };
        self.places.lock().unwrap().push(place.clone());
        Ok(place)
    }
}
