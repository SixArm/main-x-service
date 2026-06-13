//! HTTP implementation of the Main Worker Service [`Client`].
//!
//! Talks to the service's REST surface at `{base_url}/api/v1/workers/...`.
//! The service's `Worker` JSON is FHIR-shaped; we project the fields we
//! care about into our flatter [`Worker`].

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use super::{Client, Error, Worker};

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: Option<T>,
    #[serde(default)]
    success: bool,
}

#[derive(Debug, Deserialize)]
struct WorkerDto {
    id: Uuid,
    name: NameDto,
    #[serde(default)]
    worker_type: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct NameDto {
    #[serde(default)]
    family: String,
    #[serde(default)]
    given: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SearchEnvelope {
    #[serde(default)]
    workers: Vec<WorkerDto>,
}

impl WorkerDto {
    fn into_worker(self) -> Worker {
        let given = self.name.given.join(" ");
        let full = if given.is_empty() {
            self.name.family.clone()
        } else {
            format!("{} {}", given, self.name.family)
        };
        Worker {
            id: self.id,
            name: full,
            role: self.worker_type,
        }
    }
}

#[derive(Clone)]
pub struct HttpClient {
    base_url: String,
    http: reqwest::Client,
}

impl HttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("build reqwest client"),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[async_trait]
impl Client for HttpClient {
    async fn search(&self, query: &str) -> Result<Vec<Worker>, Error> {
        let response = self
            .http
            .get(self.url("/api/v1/workers/search"))
            .query(&[("q", query), ("limit", "25"), ("fuzzy", "true")])
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "search returned HTTP {}",
                response.status()
            )));
        }
        let body: ApiResponse<SearchEnvelope> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        if !body.success {
            return Err(Error::BadResponse("search returned success=false".into()));
        }
        Ok(body
            .data
            .unwrap_or_default()
            .workers
            .into_iter()
            .map(WorkerDto::into_worker)
            .collect())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Worker>, Error> {
        let response = self
            .http
            .get(self.url(&format!("/api/v1/workers/{id}")))
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "get returned HTTP {}",
                response.status()
            )));
        }
        let body: ApiResponse<WorkerDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        Ok(body.data.map(WorkerDto::into_worker))
    }
}
