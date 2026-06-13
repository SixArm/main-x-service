//! HTTP implementation of the Main Patient Service [`Client`].
//!
//! Talks to the service's REST surface at `{base_url}/api/v1/persons/...`.
//! The service's `Person` JSON is FHIR-shaped; we project the fields we
//! care about into our flatter [`Patient`].

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::{Client, CreatePatient, Error, Patient, NHS_NUMBER_SYSTEM};
use crate::nhs::{format_nhs_number, normalise_nhs_number};

/// Wraps the service's `{success,data,error}` envelope. We only consume `data`.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: Option<T>,
    #[serde(default)]
    success: bool,
}

#[derive(Debug, Deserialize)]
struct PersonDto {
    id: Uuid,
    #[serde(default)]
    identifiers: Vec<IdentifierDto>,
    name: NameDto,
    #[serde(default)]
    birth_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize)]
struct IdentifierDto {
    #[serde(default)]
    system: String,
    #[serde(default)]
    value: String,
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
    persons: Vec<PersonDto>,
}

impl PersonDto {
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
    async fn find_by_nhs_number(&self, nhs_number: &str) -> Result<Option<Patient>, Error> {
        let normalised = normalise_nhs_number(nhs_number);
        if normalised.is_empty() {
            return Ok(None);
        }
        let response = self
            .http
            .get(self.url("/api/v1/persons/search"))
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

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Patient>, Error> {
        let response = self
            .http
            .get(self.url(&format!("/api/v1/persons/{id}")))
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
            .post(self.url("/api/v1/persons"))
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

fn split_name(full: &str) -> (Vec<String>, String) {
    let mut parts: Vec<String> = full.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return (vec![], String::new());
    }
    let family = parts.pop().unwrap_or_default();
    (parts, family)
}
