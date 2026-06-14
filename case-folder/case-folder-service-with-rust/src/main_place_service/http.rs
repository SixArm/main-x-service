//! HTTP implementation of the Main Place Service [`Client`].
//!
//! Talks to the service's REST surface at `{base_url}/api/v1/places/...`
//! (the endpoint shape documented by the upstream crate).

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::{Client, CreatePlace, Error, Place};

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

/// Wire shape of an upstream `Place`, trimmed to what the tracker reads.
#[derive(Debug, Deserialize)]
struct PlaceDto {
    /// Place UUID.
    id: Uuid,
    /// Place name.
    name: String,
    /// Raw `place_type`; may be a bare string or `{"Other": "..."}`.
    #[serde(default)]
    place_type: Option<serde_json::Value>,
    /// Optional free-text description.
    #[serde(default)]
    description: Option<String>,
    /// Parent place UUID, if nested.
    #[serde(default)]
    contained_in_place: Option<Uuid>,
    /// Some upstream representations carry capacity as
    /// `maximum_attendee_capacity` (schema.org); accept both.
    #[serde(default)]
    maximum_attendee_capacity: Option<i32>,
    /// Tracker-style capacity field; preferred over `maximum_attendee_capacity`.
    #[serde(default)]
    capacity: Option<i32>,
}

/// The `data` shape returned by the place search endpoint.
#[derive(Debug, Deserialize, Default)]
struct SearchEnvelope {
    /// Matching places.
    #[serde(default)]
    places: Vec<PlaceDto>,
}

impl PlaceDto {
    /// Projects this DTO into the tracker [`Place`], flattening the
    /// tagged `place_type` enum and choosing `capacity` if present,
    /// falling back to `maximum_attendee_capacity`.
    fn into_place(self) -> Place {
        // The upstream `PlaceType` is `enum { Landform, …, Other(String) }`.
        // It serialises either as the bare variant ("Hospital") or as
        // `{"Other": "FileCabinet"}`. Normalise both into a flat string.
        let place_type = match self.place_type {
            Some(serde_json::Value::String(s)) => Some(s),
            Some(serde_json::Value::Object(map)) => map
                .get("Other")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            _ => None,
        };
        Place {
            id: self.id,
            name: self.name,
            place_type,
            description: self.description,
            contained_in_place: self.contained_in_place,
            // Prefer the tracker `capacity`; fall back to schema.org's
            // `maximum_attendee_capacity` when only that is populated.
            capacity: self.capacity.or(self.maximum_attendee_capacity),
        }
    }
}

/// REST [`Client`] for the Main Place Service.
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
    /// `GET /api/v1/places/search?q=<query>&limit=100[&place_type=<pt>]` —
    /// free-text name search, optionally filtered by place type.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status and
    /// [`Error::BadResponse`] on a parse failure or `success=false`.
    async fn search(&self, query: &str, place_type: Option<&str>) -> Result<Vec<Place>, Error> {
        let mut q = vec![("q", query.to_string()), ("limit", "100".to_string())];
        if let Some(pt) = place_type {
            q.push(("place_type", pt.to_string()));
        }
        let response = self
            .http
            .get(self.url("/api/v1/places/search"))
            .query(&q)
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
            .places
            .into_iter()
            .map(PlaceDto::into_place)
            .collect())
    }

    /// `GET /api/v1/places/{id}` — look up a place by UUID. A `404` maps
    /// to `Ok(None)`.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on other non-success statuses and
    /// [`Error::BadResponse`] on a parse failure.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Place>, Error> {
        let response = self
            .http
            .get(self.url(&format!("/api/v1/places/{id}")))
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
        let body: ApiResponse<PlaceDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        Ok(body.data.map(PlaceDto::into_place))
    }

    /// `POST /api/v1/places` — registers a new place. When `capacity` is
    /// set it is written to both `maximum_attendee_capacity` and
    /// `capacity` so either upstream representation round-trips.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status and
    /// [`Error::BadResponse`] when the response has no usable `data`.
    async fn create(&self, input: CreatePlace) -> Result<Place, Error> {
        let mut payload = json!({
            "id": Uuid::new_v4(),
            "name": input.name,
            "place_type": input.place_type,
            "description": input.description,
            "contained_in_place": input.contained_in_place,
            "keywords": [],
            "identifiers": [],
            "amenity_features": [],
            "opening_hours": [],
        });
        if let Some(c) = input.capacity {
            payload["maximum_attendee_capacity"] = serde_json::json!(c);
            payload["capacity"] = serde_json::json!(c);
        }
        let response = self
            .http
            .post(self.url("/api/v1/places"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Transport(format!(
                "create returned non-success: {body}"
            )));
        }
        let body: ApiResponse<PlaceDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .map(PlaceDto::into_place)
            .ok_or_else(|| Error::BadResponse("create returned empty data".into()))
    }
}
