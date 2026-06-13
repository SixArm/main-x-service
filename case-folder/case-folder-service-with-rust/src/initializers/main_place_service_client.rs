//! Registers a shared Main Place Service [`Client`] in the router as an
//! Axum extension. Same `RoutingClient` pattern as the other external
//! services.

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    app::{AppContext, Initializer as LocoInitializer},
    Result,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::main_place_service::{http::HttpClient, Client, CreatePlace, Error, Place};

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
    async fn search(
        &self,
        query: &str,
        place_type: Option<&str>,
    ) -> std::result::Result<Vec<Place>, Error> {
        if let Some(test) = self.pick() {
            test.search(query, place_type).await
        } else {
            self.fallback.search(query, place_type).await
        }
    }

    async fn find_by_id(&self, id: Uuid) -> std::result::Result<Option<Place>, Error> {
        if let Some(test) = self.pick() {
            test.find_by_id(id).await
        } else {
            self.fallback.find_by_id(id).await
        }
    }

    async fn create(&self, input: CreatePlace) -> std::result::Result<Place, Error> {
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
        "main-place-service-client".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_PLACE_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5153".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
