//! Registers a shared Main Patient Service [`Client`] in the router as an
//! Axum extension.
//!
//! The router always sees a [`RoutingClient`] which, at request time,
//! delegates to either:
//!   - the test client (when `set_test_client` has been called — used by
//!     the integration tests to inject a `StubClient`), or
//!   - the HTTP client configured from `MAIN_PATIENT_SERVICE_BASE_URL`
//!     (default `http://localhost:5151`).
//!
//! Routing-at-request-time lets tests register their stub *after* the app
//! has booted, which is what Loco's `request::<App, …>` testing harness
//! forces.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Initializer as LocoInitializer},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::main_patient_service::{Client, CreatePatient, Error, Patient, http::HttpClient};

static TEST_CLIENT: Mutex<Option<Arc<dyn Client>>> = Mutex::new(None);

/// Replace the live Main Patient Service client with a test double for
/// the rest of the process. Calling this after a request is in flight is
/// fine — the next request will pick up the override.
///
/// # Panics
///
/// If the override slot's mutex is poisoned — i.e. a previous holder
/// panicked while swapping the client. Test-harness wiring only, so a
/// poisoned slot means the test run is already unsound.
pub fn set_test_client(client: Arc<dyn Client>) {
    *TEST_CLIENT.lock().unwrap() = Some(client);
}

/// Remove any test override and revert to the HTTP client.
///
/// # Panics
///
/// If the override slot's mutex is poisoned — i.e. a previous holder
/// panicked while swapping the client. Test-harness wiring only, so a
/// poisoned slot means the test run is already unsound.
pub fn clear_test_client() {
    *TEST_CLIENT.lock().unwrap() = None;
}

/// Client that the router always sees. Each call checks the override
/// slot first and otherwise routes to the HTTP `fallback`.
struct RoutingClient {
    /// Real HTTP client used whenever no test override is installed.
    fallback: HttpClient,
}

impl RoutingClient {
    /// Return the current test override (cloned `Arc`) if one is set.
    fn pick() -> Option<Arc<dyn Client>> {
        TEST_CLIENT.lock().unwrap().clone()
    }
}

#[async_trait]
impl Client for RoutingClient {
    /// Look up a patient by NHS number; override else HTTP fallback.
    async fn find_by_nhs_number(
        &self,
        nhs_number: &str,
    ) -> std::result::Result<Option<Patient>, Error> {
        if let Some(test) = Self::pick() {
            test.find_by_nhs_number(nhs_number).await
        } else {
            self.fallback.find_by_nhs_number(nhs_number).await
        }
    }

    /// Look up a patient by id; override else HTTP fallback.
    async fn find_by_id(&self, id: Uuid) -> std::result::Result<Option<Patient>, Error> {
        if let Some(test) = Self::pick() {
            test.find_by_id(id).await
        } else {
            self.fallback.find_by_id(id).await
        }
    }

    /// Create a patient; override else HTTP fallback.
    async fn create(&self, input: CreatePatient) -> std::result::Result<Patient, Error> {
        if let Some(test) = Self::pick() {
            test.create(input).await
        } else {
            self.fallback.create(input).await
        }
    }
}

/// Loco initializer that registers the Main Patient Service client.
pub struct ClientInitializer;

#[async_trait]
impl LocoInitializer for ClientInitializer {
    /// Stable identifier for this initializer in Loco's registry.
    fn name(&self) -> String {
        "main-patient-service-client".to_string()
    }

    /// Read `MAIN_PATIENT_SERVICE_BASE_URL` (default `http://localhost:5151`),
    /// build a [`RoutingClient`] over an HTTP fallback, and layer it as an
    /// Axum `Extension`.
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_PATIENT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5151".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
