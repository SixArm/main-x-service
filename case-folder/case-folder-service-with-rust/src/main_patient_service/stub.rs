//! In-memory Main Patient Service for unit + integration tests, so
//! Case Tracking can run without a real service on hand.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, CreatePatient, Error, Patient};
use crate::nhs::{format_nhs_number, normalise_nhs_number};

#[derive(Default)]
pub struct StubClient {
    patients: Mutex<Vec<Patient>>,
}

impl StubClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, patients: Vec<Patient>) {
        let mut guard = self.patients.lock().unwrap();
        for p in patients {
            if !guard.iter().any(|q| q.id == p.id) {
                guard.push(p);
            }
        }
    }

    pub fn snapshot(&self) -> Vec<Patient> {
        self.patients.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.patients.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
    async fn find_by_nhs_number(&self, nhs_number: &str) -> Result<Option<Patient>, Error> {
        let needle = normalise_nhs_number(nhs_number);
        if needle.is_empty() {
            return Ok(None);
        }
        let guard = self.patients.lock().unwrap();
        Ok(guard
            .iter()
            .find(|p| normalise_nhs_number(&p.nhs_number) == needle)
            .cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Patient>, Error> {
        let guard = self.patients.lock().unwrap();
        Ok(guard.iter().find(|p| p.id == id).cloned())
    }

    async fn create(&self, input: CreatePatient) -> Result<Patient, Error> {
        let formatted = format_nhs_number(&input.nhs_number);
        let mut guard = self.patients.lock().unwrap();
        if let Some(existing) = guard.iter().find(|p| {
            normalise_nhs_number(&p.nhs_number) == normalise_nhs_number(&input.nhs_number)
        }) {
            return Ok(existing.clone());
        }
        let patient = Patient {
            id: Uuid::new_v4(),
            nhs_number: formatted,
            name: input.name,
            date_of_birth: Some(input.date_of_birth),
        };
        guard.push(patient.clone());
        Ok(patient)
    }
}
