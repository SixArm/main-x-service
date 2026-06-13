//! Registers a shared Main Thing Service [`Client`] in the router as
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

use crate::main_thing_service::{
    http::HttpClient, Client, Error, Folder, NewFolder, NewVolume, Volume,
};

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
    async fn search(&self, query: &str) -> std::result::Result<Vec<Folder>, Error> {
        match self.pick() {
            Some(t) => t.search(query).await,
            None => self.fallback.search(query).await,
        }
    }

    async fn find_by_id(&self, id: Uuid) -> std::result::Result<Option<Folder>, Error> {
        match self.pick() {
            Some(t) => t.find_by_id(id).await,
            None => self.fallback.find_by_id(id).await,
        }
    }

    async fn list_for_patient(&self, patient_id: Uuid) -> std::result::Result<Vec<Folder>, Error> {
        match self.pick() {
            Some(t) => t.list_for_patient(patient_id).await,
            None => self.fallback.list_for_patient(patient_id).await,
        }
    }

    async fn list_for_nhs_number(
        &self,
        nhs_number: &str,
    ) -> std::result::Result<Vec<Folder>, Error> {
        match self.pick() {
            Some(t) => t.list_for_nhs_number(nhs_number).await,
            None => self.fallback.list_for_nhs_number(nhs_number).await,
        }
    }

    async fn create(&self, input: NewFolder) -> std::result::Result<Folder, Error> {
        match self.pick() {
            Some(t) => t.create(input).await,
            None => self.fallback.create(input).await,
        }
    }

    async fn update_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> std::result::Result<Folder, Error> {
        match self.pick() {
            Some(t) => {
                t.update_cabinet(id, cabinet_id, cabinet_path_snapshot)
                    .await
            }
            None => {
                self.fallback
                    .update_cabinet(id, cabinet_id, cabinet_path_snapshot)
                    .await
            }
        }
    }

    async fn create_volume(&self, input: NewVolume) -> std::result::Result<Volume, Error> {
        match self.pick() {
            Some(t) => t.create_volume(input).await,
            None => self.fallback.create_volume(input).await,
        }
    }

    async fn find_volume_by_id(&self, id: Uuid) -> std::result::Result<Option<Volume>, Error> {
        match self.pick() {
            Some(t) => t.find_volume_by_id(id).await,
            None => self.fallback.find_volume_by_id(id).await,
        }
    }

    async fn list_volumes(&self) -> std::result::Result<Vec<Volume>, Error> {
        match self.pick() {
            Some(t) => t.list_volumes().await,
            None => self.fallback.list_volumes().await,
        }
    }

    async fn rename_volume(&self, id: Uuid, title: String) -> std::result::Result<Volume, Error> {
        match self.pick() {
            Some(t) => t.rename_volume(id, title).await,
            None => self.fallback.rename_volume(id, title).await,
        }
    }

    async fn update_volume_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> std::result::Result<Volume, Error> {
        match self.pick() {
            Some(t) => {
                t.update_volume_cabinet(id, cabinet_id, cabinet_path_snapshot)
                    .await
            }
            None => {
                self.fallback
                    .update_volume_cabinet(id, cabinet_id, cabinet_path_snapshot)
                    .await
            }
        }
    }

    async fn set_folder_volume(
        &self,
        folder_id: Uuid,
        volume_id: Option<Uuid>,
        volume_title_snapshot: Option<String>,
    ) -> std::result::Result<Folder, Error> {
        match self.pick() {
            Some(t) => {
                t.set_folder_volume(folder_id, volume_id, volume_title_snapshot)
                    .await
            }
            None => {
                self.fallback
                    .set_folder_volume(folder_id, volume_id, volume_title_snapshot)
                    .await
            }
        }
    }
}

pub struct ClientInitializer;

#[async_trait]
impl LocoInitializer for ClientInitializer {
    fn name(&self) -> String {
        "main-thing-service-client".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let base_url = std::env::var("MAIN_THING_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:5154".to_string());
        let client: Arc<dyn Client> = Arc::new(RoutingClient {
            fallback: HttpClient::new(base_url),
        });
        Ok(router.layer(axum::Extension(client)))
    }
}
