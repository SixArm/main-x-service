//! Registers a shared Main Worker Service [`Client`] in the router as an
//! Axum extension. Mirrors the Main Patient Service initializer:
//! a [`RoutingClient`] checks an override slot at request time so tests
//! can inject a stub after the app has booted.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Initializer as LocoInitializer},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::main_worker_service::{Client, Error, Worker, http::HttpClient};

/// Process-wide override slot. When `Some`, the [`RoutingClient`]
/// delegates to it instead of the HTTP fallback. Read at request time.
static TEST_CLIENT: Mutex<Option<Arc<dyn Client>>> = Mutex::new(None);

/// Install a test double as the override. The next request picks it up.
///
/// # Panics
///
/// If the override slot's mutex is poisoned — i.e. a previous holder
/// panicked while swapping the client. Test-harness wiring only, so a
/// poisoned slot means the test run is already unsound.
pub fn set_test_client(client: Arc<dyn Client>) {
    *TEST_CLIENT.lock().unwrap() = Some(client);
}

/// Remove any test override and revert to the HTTP fallback.
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
    /// Search workers (staff directory); override else HTTP fallback.
    async fn search(&self, query: &str) -> std::result::Result<Vec<Worker>, Error> {
        if let Some(test) = Self::pick() {
            test.search(query).await
        } else {
            self.fallback.search(query).await
        }
    }

    /// Look up a worker by id; override else HTTP fallback.
    async fn find_by_id(&self, id: Uuid) -> std::result::Result<Option<Worker>, Error> {
        if let Some(test) = Self::pick() {
            test.find_by_id(id).await
        } else {
            self.fallback.find_by_id(id).await
        }
    }
}

/// Loco initializer that registers the Main Worker Service client.
pub struct ClientInitializer;

#[async_trait]
impl LocoInitializer for ClientInitializer {
    /// Stable identifier for this initializer in Loco's registry.
    fn name(&self) -> String {
        "main-worker-service-client".to_string()
    }

    /// Read `MAIN_WORKER_SERVICE_BASE_URL` (default `http://localhost:5152`),
    /// build a [`RoutingClient`] over an HTTP fallback, and layer it as an
    /// Axum `Extension`.
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_WORKER_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5152".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
