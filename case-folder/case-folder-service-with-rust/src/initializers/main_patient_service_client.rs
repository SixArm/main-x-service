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
    app::{AppContext, Initializer as LocoInitializer},
    Result,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::main_patient_service::{http::HttpClient, Client, CreatePatient, Error, Patient};

static TEST_CLIENT: Mutex<Option<Arc<dyn Client>>> = Mutex::new(None);

/// Replace the live Main Patient Service client with a test double for
/// the rest of the process. Calling this after a request is in flight is
/// fine — the next request will pick up the override.
pub fn set_test_client(client: Arc<dyn Client>) {
    *TEST_CLIENT.lock().unwrap() = Some(client);
}

/// Remove any test override and revert to the HTTP client.
pub fn clear_test_client() {
    *TEST_CLIENT.lock().unwrap() = None;
}

struct RoutingClient {
    fallback: HttpClient,
}

impl RoutingClient {
    fn pick(&self) -> Option<Arc<dyn Client>> {
        TEST_CLIENT.lock().unwrap().clone()
    }
}

#[async_trait]
impl Client for RoutingClient {
    async fn find_by_nhs_number(
        &self,
        nhs_number: &str,
    ) -> std::result::Result<Option<Patient>, Error> {
        if let Some(test) = self.pick() {
            test.find_by_nhs_number(nhs_number).await
        } else {
            self.fallback.find_by_nhs_number(nhs_number).await
        }
    }

    async fn find_by_id(&self, id: Uuid) -> std::result::Result<Option<Patient>, Error> {
        if let Some(test) = self.pick() {
            test.find_by_id(id).await
        } else {
            self.fallback.find_by_id(id).await
        }
    }

    async fn create(&self, input: CreatePatient) -> std::result::Result<Patient, Error> {
        if let Some(test) = self.pick() {
            test.create(input).await
        } else {
            self.fallback.create(input).await
        }
    }
}

pub struct ClientInitializer;

#[async_trait]
impl LocoInitializer for ClientInitializer {
    fn name(&self) -> String {
        "main-patient-service-client".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_PATIENT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5151".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
