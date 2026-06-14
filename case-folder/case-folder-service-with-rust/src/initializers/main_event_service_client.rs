//! Registers a shared Main Event Service [`Client`] in the router as
//! an Axum extension. Same `RoutingClient` pattern as the other
//! external services.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Initializer as LocoInitializer},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::main_event_service::{Client, Error, MoveEvent, RecordMove, http::HttpClient};

/// Process-wide override slot. When `Some`, the [`RoutingClient`]
/// delegates to it instead of the HTTP fallback. Read at request time
/// (not boot time) so tests can inject a stub after the app has started.
static TEST_CLIENT: Mutex<Option<Arc<dyn Client>>> = Mutex::new(None);

/// Install a test double as the override. The next request picks it up.
pub fn set_test_client(client: Arc<dyn Client>) {
    *TEST_CLIENT.lock().unwrap() = Some(client);
}

/// Remove any test override and revert to the HTTP fallback.
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
    fn pick(&self) -> Option<Arc<dyn Client>> {
        TEST_CLIENT.lock().unwrap().clone()
    }
}

#[async_trait]
impl Client for RoutingClient {
    /// Record a move event; delegates to the override else the HTTP fallback.
    async fn record(&self, input: RecordMove) -> std::result::Result<MoveEvent, Error> {
        match self.pick() {
            Some(t) => t.record(input).await,
            None => self.fallback.record(input).await,
        }
    }

    /// List every move event; delegates to the override else the HTTP fallback.
    async fn list_all(&self) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_all().await,
            None => self.fallback.list_all().await,
        }
    }

    /// List the `limit` most recent move events; override else HTTP fallback.
    async fn list_recent(&self, limit: u32) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_recent(limit).await,
            None => self.fallback.list_recent(limit).await,
        }
    }

    /// List move events for one folder; override else HTTP fallback.
    async fn list_for_folder(&self, folder_id: Uuid) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_for_folder(folder_id).await,
            None => self.fallback.list_for_folder(folder_id).await,
        }
    }

    /// List move events for one patient; override else HTTP fallback.
    async fn list_for_patient(
        &self,
        patient_id: Uuid,
    ) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_for_patient(patient_id).await,
            None => self.fallback.list_for_patient(patient_id).await,
        }
    }
}

/// Loco initializer that registers the Main Event Service client.
pub struct ClientInitializer;

#[async_trait]
impl LocoInitializer for ClientInitializer {
    /// Stable identifier for this initializer in Loco's registry.
    fn name(&self) -> String {
        "main-event-service-client".to_string()
    }

    /// Read `MAIN_EVENT_SERVICE_BASE_URL` (default `http://localhost:5155`),
    /// build a [`RoutingClient`] over an HTTP fallback, and layer it as an
    /// Axum `Extension` so controllers can resolve `Arc<dyn Client>`.
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_EVENT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5155".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
