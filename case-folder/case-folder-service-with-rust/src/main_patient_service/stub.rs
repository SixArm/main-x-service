//! In-memory Main Patient Service for unit + integration tests, so
//! Case Tracking can run without a real service on hand.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, CreatePatient, Error, Patient};
use crate::nhs::{format_nhs_number, normalise_nhs_number};

/// In-memory test double for the Main Patient Service [`Client`]. Holds
/// patients in a `Mutex`-guarded `Vec`; no network involved.
#[derive(Default)]
pub struct StubClient {
    /// Stored patients, in insertion order.
    patients: Mutex<Vec<Patient>>,
}

impl StubClient {
    /// Creates an empty stub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-loads `patients`, skipping any whose `id` is already present
    /// (dedupe-by-id).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn seed(&self, patients: Vec<Patient>) {
        let mut guard = self.patients.lock().unwrap();
        for p in patients {
            if !guard.iter().any(|q| q.id == p.id) {
                guard.push(p);
            }
        }
    }

    /// Returns a clone of all stored patients (test inspection helper).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn snapshot(&self) -> Vec<Patient> {
        self.patients.lock().unwrap().clone()
    }

    /// Removes all stored patients.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn clear(&self) {
        self.patients.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
    /// Finds a patient whose normalised NHS Number matches `nhs_number`.
    /// Empty/blank input yields `Ok(None)`.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
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

    /// Finds a patient by UUID.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Patient>, Error> {
        let guard = self.patients.lock().unwrap();
        Ok(guard.iter().find(|p| p.id == id).cloned())
    }

    /// Registers a patient. If one already exists with the same NHS
    /// Number, returns it unchanged (idempotent create); otherwise stores
    /// a new patient with a fresh UUID and the formatted NHS Number.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
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
