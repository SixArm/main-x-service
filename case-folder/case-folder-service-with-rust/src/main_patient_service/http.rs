//! HTTP implementation of the Main Patient Service [`Client`].
//!
//! Talks to the service's REST surface at `{base_url}/api/persons/...`.
//! The service's `Person` JSON is FHIR-shaped; we project the fields we
//! care about into our flatter [`Patient`].

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::{Client, CreatePatient, Error, NHS_NUMBER_SYSTEM, Patient};
use crate::nhs::{format_nhs_number, normalise_nhs_number};

/// Wraps the service's `{success,data,error}` envelope. We only consume `data`.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    /// Payload, present on success.
    data: Option<T>,
    /// Service-reported success flag (checked on the search path).
    #[serde(default)]
    success: bool,
}

/// Wire shape of an upstream FHIR-style `Person`, trimmed to what the
/// tracker reads.
#[derive(Debug, Deserialize)]
struct PersonDto {
    /// Person UUID.
    id: Uuid,
    /// Identifier list; the NHS Number is found by matching `system`.
    #[serde(default)]
    identifiers: Vec<IdentifierDto>,
    /// Structured name (given parts + family).
    name: NameDto,
    /// Date of birth, if present.
    #[serde(default)]
    birth_date: Option<chrono::NaiveDate>,
}

/// A single identifier on a `Person` (e.g. the NHS Number).
#[derive(Debug, Deserialize)]
struct IdentifierDto {
    /// Identifier system URI (compared against [`NHS_NUMBER_SYSTEM`]).
    #[serde(default)]
    system: String,
    /// Identifier value (the raw NHS Number digits).
    #[serde(default)]
    value: String,
}

/// Structured person name as returned by the upstream service.
#[derive(Debug, Deserialize, Default)]
struct NameDto {
    /// Family (last) name.
    #[serde(default)]
    family: String,
    /// Given (first/middle) name parts.
    #[serde(default)]
    given: Vec<String>,
}

/// The `data` shape returned by the person search endpoint.
#[derive(Debug, Deserialize, Default)]
struct SearchEnvelope {
    /// Matching persons.
    #[serde(default)]
    persons: Vec<PersonDto>,
}

impl PersonDto {
    /// Projects this DTO into the flatter tracker [`Patient`]. Picks the
    /// NHS Number from the identifier whose `system` is [`NHS_NUMBER_SYSTEM`]
    /// and formats it; joins given parts with the family into a full name.
    fn into_patient(self) -> Patient {
        let nhs_number = self
            .identifiers
            .iter()
            .find(|i| i.system == NHS_NUMBER_SYSTEM)
            .map(|i| format_nhs_number(&i.value))
            .unwrap_or_default();
        let given = self.name.given.join(" ");
        let full = if given.is_empty() {
            self.name.family.clone()
        } else {
            format!("{} {}", given, self.name.family)
        };
        Patient {
            id: self.id,
            nhs_number,
            name: full,
            date_of_birth: self.birth_date,
        }
    }
}

/// REST [`Client`] for the Main Patient Service.
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
    /// `GET /api/persons/search?q=<digits>&limit=5&fuzzy=false` — looks
    /// up a patient by NHS Number. Normalises the input to digits, then
    /// re-checks each candidate's NHS Number identifier exactly (the search
    /// is a coarse text match, so we confirm an exact hit before returning).
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status,
    /// [`Error::BadResponse`] on a parse failure or `success=false`.
    async fn find_by_nhs_number(&self, nhs_number: &str) -> Result<Option<Patient>, Error> {
        let normalised = normalise_nhs_number(nhs_number);
        if normalised.is_empty() {
            return Ok(None);
        }
        let response = self
            .http
            .get(self.url("/api/persons/search"))
            .query(&[
                ("q", normalised.as_str()),
                ("limit", "5"),
                ("fuzzy", "false"),
            ])
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
        let envelope = body.data.unwrap_or_default();
        let target_digits = normalised.as_str();
        let matched = envelope.persons.into_iter().find(|p| {
            p.identifiers.iter().any(|i| {
                i.system == NHS_NUMBER_SYSTEM && normalise_nhs_number(&i.value) == target_digits
            })
        });
        Ok(matched.map(PersonDto::into_patient))
    }

    /// `GET /api/persons/{id}` — look up a patient by UUID. A `404`
    /// maps to `Ok(None)`.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on other non-success statuses and
    /// [`Error::BadResponse`] on a parse failure.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Patient>, Error> {
        let response = self
            .http
            .get(self.url(&format!("/api/persons/{id}")))
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
        let body: ApiResponse<PersonDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        Ok(body.data.map(PersonDto::into_patient))
    }

    /// `POST /api/persons` — registers a new patient. Sends the NHS
    /// Number as an identifier under [`NHS_NUMBER_SYSTEM`] and the name
    /// split into given/family parts; returns the projected [`Patient`].
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status and
    /// [`Error::BadResponse`] when the response has no usable `data`.
    async fn create(&self, input: CreatePatient) -> Result<Patient, Error> {
        let normalised = normalise_nhs_number(&input.nhs_number);
        let (given, family) = split_name(&input.name);
        let payload = json!({
            "id": Uuid::new_v4(),
            "identifiers": [{
                "identifier_type": "OTHER",
                "system": NHS_NUMBER_SYSTEM,
                "value": normalised,
            }],
            "active": true,
            "name": {
                "family": family,
                "given": given,
                "prefix": [],
                "suffix": [],
            },
            "additional_names": [],
            "telecom": [],
            "gender": "unknown",
            "birth_date": input.date_of_birth.format("%Y-%m-%d").to_string(),
            "deceased": false,
            "addresses": [],
            "photo": [],
            "links": [],
        });
        let response = self
            .http
            .post(self.url("/api/persons"))
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
        let body: ApiResponse<PersonDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .map(PersonDto::into_patient)
            .ok_or_else(|| Error::BadResponse("create returned empty data".into()))
    }
}

/// Splits a full name into `(given_parts, family)`. The last
/// whitespace-separated token is taken as the family name; everything
/// before it is the given parts. Returns `(vec![], "")` for empty input.
fn split_name(full: &str) -> (Vec<String>, String) {
    let mut parts: Vec<String> = full.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return (vec![], String::new());
    }
    let family = parts.pop().unwrap_or_default();
    (parts, family)
}
