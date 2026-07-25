//! Client for the Main Patient Service.
//!
//! The Main Patient Service is an external Rust service that owns the
//! patient master index. We talk to it over HTTP/JSON (REST). This crate
//! keeps no patient tables of its own — folders carry an opaque patient
//! UUID plus a denormalised snapshot of the NHS Number and patient name
//! so the UI can render lists without a network round-trip per row.
//!
//! The [`Client`] trait keeps controllers and tests decoupled from any
//! particular transport. The HTTP implementation lives in
//! [`http`](self::http); tests use [`stub::StubClient`].

pub mod http;
pub mod stub;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// NHS England's canonical identifier system URI for the NHS Number.
pub const NHS_NUMBER_SYSTEM: &str = "https://fhir.nhs.uk/Id/nhs-number";

/// Minimal patient projection used by Case Tracking. The Main
/// Patient Service returns much richer FHIR-style structures; we map them
/// into this flatter shape because it's all the tracker needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    /// Main Patient Service UUID (the patient's stable identifier).
    pub id: Uuid,
    /// NHS Number, formatted `XXX XXX XXXX` for display.
    pub nhs_number: String,
    /// Full display name (`given… family`).
    pub name: String,
    /// Date of birth, if the service has one on record.
    pub date_of_birth: Option<chrono::NaiveDate>,
}

/// Input used when registering a new patient with the Main Patient
/// Service from inside the "add folder" workflow.
#[derive(Debug, Clone)]
pub struct CreatePatient {
    /// NHS Number in any format; normalised to digits before sending.
    pub nhs_number: String,
    /// Full display name; split into given/family parts on the way out.
    pub name: String,
    /// Date of birth (required when creating a patient).
    pub date_of_birth: chrono::NaiveDate,
}

/// Errors raised by the Main Patient Service client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Network/transport failure, or a non-success HTTP status.
    #[error("Main Patient Service request failed: {0}")]
    Transport(String),
    /// The service replied but the body could not be parsed/mapped.
    #[error("Main Patient Service returned an unexpected response: {0}")]
    BadResponse(String),
    /// The service confirmed no matching patient exists.
    #[error("Main Patient Service did not find a patient")]
    NotFound,
}

/// Flattens our typed [`Error`] into a `loco_rs::Error::Message` for
/// `?`-propagation in controllers.
impl From<Error> for loco_rs::Error {
    fn from(value: Error) -> Self {
        loco_rs::Error::Message(value.to_string())
    }
}

#[async_trait]
/// The patient-identity operations this crate needs from the upstream
/// main-patient-service. A trait so tests can substitute a fake without a
/// live HTTP dependency.
pub trait Client: Send + Sync {
    /// Look up a patient by NHS Number. Returns `Ok(None)` when the
    /// service confirms there is no matching record.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn find_by_nhs_number(&self, nhs_number: &str) -> Result<Option<Patient>, Error>;

    /// Look up a patient by service UUID.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Patient>, Error>;

    /// Register a new patient with the service. Returns the assigned UUID.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn create(&self, input: CreatePatient) -> Result<Patient, Error>;
}

/// Find an existing patient by NHS Number or create one. Convenience used
/// by the "add folder" controller.
///
/// # Errors
/// Returns [`Error`] if the lookup or create call fails.
pub async fn find_or_create(client: &dyn Client, input: CreatePatient) -> Result<Patient, Error> {
    if let Some(existing) = client.find_by_nhs_number(&input.nhs_number).await? {
        return Ok(existing);
    }
    client.create(input).await
}
