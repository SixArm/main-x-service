//! Client for the Main Worker Service.
//!
//! The Main Worker Service is an external Rust service that owns the
//! registry of hospital staff who handle paper folders (clinicians,
//! doctors, nurses, administrators, secretaries, …). We talk to it over
//! HTTP/JSON (REST). The tracker keeps no worker tables of its own — move
//! events carry the worker's opaque UUID plus a snapshot of the name and
//! role so the audit log survives service outages and downstream renames.
//!
//! The [`Client`] trait keeps controllers and tests decoupled from any
//! particular transport. The HTTP implementation lives in
//! [`http`](self::http); tests use [`stub::StubClient`].

pub mod http;
pub mod stub;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Minimal worker projection used by Case Tracking. The Main
/// Worker Service returns much richer FHIR-style structures; we map them
/// into this flatter shape because it's all the tracker needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    /// Main Worker Service UUID (the worker's stable identifier).
    pub id: Uuid,
    /// Full display name (`given… family`).
    pub name: String,
    /// Worker classification (`doctor`, `nurse`, `carer`, `staff`,
    /// `employee`, `manager`, `supervisor`, `consultant`, `other`).
    /// Optional because the upstream service treats it as optional.
    pub role: Option<String>,
}

/// Errors raised by the Main Worker Service client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Network/transport failure, or a non-success HTTP status.
    #[error("Main Worker Service request failed: {0}")]
    Transport(String),
    /// The service replied but the body could not be parsed/mapped.
    #[error("Main Worker Service returned an unexpected response: {0}")]
    BadResponse(String),
    /// The service confirmed no matching worker exists.
    #[error("Main Worker Service did not find a worker")]
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
/// The worker-identity operations this crate needs from the upstream
/// main-worker-service. A trait so tests can substitute a fake without a
/// live HTTP dependency.
pub trait Client: Send + Sync {
    /// Free-text search by name. Returns matching workers (capped by the
    /// upstream service's `limit` query param — we default to 25).
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn search(&self, query: &str) -> Result<Vec<Worker>, Error>;

    /// Look up a single worker by service UUID.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Worker>, Error>;
}
