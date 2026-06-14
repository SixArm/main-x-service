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

/// The upstream `{success, data, error}` envelope. We consume `data` and,
/// on search, `success`.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    /// Payload, present on success.
    data: Option<T>,
    /// Service-reported success flag (checked on the search path).
    #[serde(default)]
    success: bool,
}

/// Wire shape of an upstream FHIR-style `Worker`, trimmed to what the
/// tracker reads.
#[derive(Debug, Deserialize)]
struct WorkerDto {
    /// Worker UUID.
    id: Uuid,
    /// Structured name (given parts + family).
    name: NameDto,
    /// Worker classification, mapped to [`Worker::role`].
    #[serde(default)]
    worker_type: Option<String>,
}

/// Structured worker name as returned by the upstream service.
#[derive(Debug, Deserialize, Default)]
struct NameDto {
    /// Family (last) name.
    #[serde(default)]
    family: String,
    /// Given (first/middle) name parts.
    #[serde(default)]
    given: Vec<String>,
}

/// The `data` shape returned by the worker search endpoint.
#[derive(Debug, Deserialize, Default)]
struct SearchEnvelope {
    /// Matching workers.
    #[serde(default)]
    workers: Vec<WorkerDto>,
}

impl WorkerDto {
    /// Projects this DTO into the flatter tracker [`Worker`], joining the
    /// given parts with the family into a single display name.
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

/// REST [`Client`] for the Main Worker Service.
#[derive(Clone)]
pub struct HttpClient {
    /// Service base URL with any trailing slash trimmed.
    base_url: String,
    /// Shared `reqwest` client (connection pool + 5s timeout).
    http: reqwest::Client,
}

impl HttpClient {
    /// Builds a client for the service at `base_url`. Trims a trailing
    /// slash and applies a 5-second request timeout.
    ///
    /// # Panics
    /// Panics if the underlying `reqwest` client cannot be built.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("build reqwest client"),
        }
    }

    /// Joins `path` onto the base URL to form an absolute request URL.
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[async_trait]
impl Client for HttpClient {
    /// `GET /api/v1/workers/search?q=<query>&limit=25&fuzzy=true` —
    /// fuzzy free-text name search, capped at 25 results.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status and
    /// [`Error::BadResponse`] on a parse failure or `success=false`.
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

    /// `GET /api/v1/workers/{id}` — look up a worker by UUID. A `404` maps
    /// to `Ok(None)`.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on other non-success statuses and
    /// [`Error::BadResponse`] on a parse failure.
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
