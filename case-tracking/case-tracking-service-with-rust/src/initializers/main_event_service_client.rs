//! Registers a shared Main Event Service [`Client`] in the router as
//! an Axum extension. Same `RoutingClient` pattern as the other
//! external services.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    app::{AppContext, Initializer as LocoInitializer},
    Result,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::main_event_service::{http::HttpClient, Client, Error, MoveEvent, RecordMove};

static TEST_CLIENT: Mutex<Option<Arc<dyn Client>>> = Mutex::new(None);

pub fn set_test_client(client: Arc<dyn Client>) {
    *TEST_CLIENT.lock().unwrap() = Some(client);
}

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
    async fn record(&self, input: RecordMove) -> std::result::Result<MoveEvent, Error> {
        match self.pick() {
            Some(t) => t.record(input).await,
            None => self.fallback.record(input).await,
        }
    }

    async fn list_all(&self) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_all().await,
            None => self.fallback.list_all().await,
        }
    }

    async fn list_recent(&self, limit: u32) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_recent(limit).await,
            None => self.fallback.list_recent(limit).await,
        }
    }

    async fn list_for_folder(&self, folder_id: Uuid) -> std::result::Result<Vec<MoveEvent>, Error> {
        match self.pick() {
            Some(t) => t.list_for_folder(folder_id).await,
            None => self.fallback.list_for_folder(folder_id).await,
        }
    }

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

pub struct ClientInitializer;

#[async_trait]
impl LocoInitializer for ClientInitializer {
    fn name(&self) -> String {
        "main-event-service-client".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_EVENT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5155".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
