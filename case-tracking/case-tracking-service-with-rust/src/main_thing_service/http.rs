//! HTTP implementation of the Main Thing Service [`Client`].
//!
//! Talks to the upstream `/api/things/...` REST endpoints described in
//! the service's spec. As of writing the upstream crate has the schema
//! and routing scaffolding in place but the HTTP layer is not yet
//! wired; this client therefore matches the documented surface rather
//! than a running deployment. Once the upstream service exposes the
//! routes, this client should work unchanged.
//!
//! The folder ↔ Thing mapping is documented on [`super::Folder`].

use async_trait::async_trait;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::{Client, Error, Folder, NewFolder, NewVolume, Volume};

const KEY_PATIENT_ID: &str = "patient_id=";
const KEY_PATIENT_NAME: &str = "patient_name=";
const KEY_CABINET_PATH: &str = "cabinet_path=";
const KEY_VOLUME_ID: &str = "volume_id=";
const KEY_VOLUME_TITLE: &str = "volume_title=";
/// NHS Number `propertyID` we register with the Thing service so we
/// can find folders by patient.
const NHS_PROPERTY_ID: &str = "nhs-number";
/// Marks the Thing as a case-file folder. Anything else is ignored by
/// the tracker's search/list operations.
const CASE_FILE_TYPE: &str = "CaseFile";
/// Marks the Thing as a volume (a bundle of folders).
const VOLUME_TYPE: &str = "Volume";

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: Option<T>,
    #[serde(default)]
    #[allow(dead_code)]
    success: bool,
}

#[derive(Debug, Deserialize, Default)]
struct SearchEnvelope {
    #[serde(default)]
    things: Vec<ThingDto>,
}

#[derive(Debug, Deserialize)]
struct ThingDto {
    id: Uuid,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    thing_type: Option<serde_json::Value>,
    #[serde(default)]
    contained_in_thing: Option<Uuid>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    identifiers: Vec<IdentifierDto>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IdentifierDto {
    property_id: serde_json::Value,
    value: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

impl ThingDto {
    fn into_folder(self) -> Option<Folder> {
        if !is_case_file(&self.thing_type) {
            return None;
        }
        let (patient_id, patient_name, cabinet_path, volume_id, volume_title) =
            parse_thing_keywords(&self.keywords);
        let nhs_number = self
            .identifiers
            .iter()
            .find(|i| identifier_is_nhs_number(&i.property_id))
            .map(|i| i.value.clone())
            .unwrap_or_default();
        Some(Folder {
            id: self.id,
            title: self.name,
            patient_id: patient_id.unwrap_or_else(Uuid::nil),
            nhs_number_snapshot: nhs_number,
            patient_name_snapshot: patient_name.unwrap_or_default(),
            cabinet_id: self.contained_in_thing,
            cabinet_path_snapshot: cabinet_path,
            notes: self.description,
            volume_id,
            volume_title_snapshot: volume_title,
        })
    }

    fn into_volume(self) -> Option<Volume> {
        if !is_volume(&self.thing_type) {
            return None;
        }
        let (patient_id, patient_name, cabinet_path, _, _) = parse_thing_keywords(&self.keywords);
        let nhs_number = self
            .identifiers
            .iter()
            .find(|i| identifier_is_nhs_number(&i.property_id))
            .map(|i| i.value.clone())
            .unwrap_or_default();
        Some(Volume {
            id: self.id,
            title: self.name,
            patient_id: patient_id.unwrap_or_else(Uuid::nil),
            nhs_number_snapshot: nhs_number,
            patient_name_snapshot: patient_name.unwrap_or_default(),
            cabinet_id: self.contained_in_thing,
            cabinet_path_snapshot: cabinet_path,
        })
    }
}

#[allow(clippy::type_complexity)]
fn parse_thing_keywords(
    keywords: &[String],
) -> (
    Option<Uuid>,
    Option<String>,
    Option<String>,
    Option<Uuid>,
    Option<String>,
) {
    let mut patient_id = None;
    let mut patient_name = None;
    let mut cabinet_path = None;
    let mut volume_id = None;
    let mut volume_title = None;
    for kw in keywords {
        if let Some(v) = kw.strip_prefix(KEY_PATIENT_ID) {
            patient_id = Uuid::parse_str(v).ok();
        } else if let Some(v) = kw.strip_prefix(KEY_PATIENT_NAME) {
            patient_name = Some(v.to_string());
        } else if let Some(v) = kw.strip_prefix(KEY_CABINET_PATH) {
            cabinet_path = Some(v.to_string());
        } else if let Some(v) = kw.strip_prefix(KEY_VOLUME_ID) {
            volume_id = Uuid::parse_str(v).ok();
        } else if let Some(v) = kw.strip_prefix(KEY_VOLUME_TITLE) {
            volume_title = Some(v.to_string());
        }
    }
    (
        patient_id,
        patient_name,
        cabinet_path,
        volume_id,
        volume_title,
    )
}

fn is_volume(value: &Option<serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::String(s)) => s == VOLUME_TYPE,
        Some(serde_json::Value::Object(map)) => map
            .get("Other")
            .and_then(|v| v.as_str())
            .map(|s| s == VOLUME_TYPE)
            .unwrap_or(false),
        _ => false,
    }
}

fn is_case_file(value: &Option<serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::String(s)) => s == CASE_FILE_TYPE,
        Some(serde_json::Value::Object(map)) => map
            .get("Other")
            .and_then(|v| v.as_str())
            .map(|s| s == CASE_FILE_TYPE)
            .unwrap_or(false),
        _ => false,
    }
}

fn identifier_is_nhs_number(property_id: &serde_json::Value) -> bool {
    match property_id {
        serde_json::Value::Object(map) => map
            .get("Custom")
            .and_then(|v| v.as_str())
            .map(|s| s == NHS_PROPERTY_ID)
            .unwrap_or(false),
        serde_json::Value::String(s) => s == NHS_PROPERTY_ID,
        _ => false,
    }
}

fn build_keywords(input: &NewFolder) -> Vec<String> {
    let mut out = vec![
        format!("{KEY_PATIENT_ID}{}", input.patient_id),
        format!("{KEY_PATIENT_NAME}{}", input.patient_name_snapshot),
    ];
    if let Some(p) = &input.cabinet_path_snapshot {
        out.push(format!("{KEY_CABINET_PATH}{p}"));
    }
    if let Some(v) = &input.volume_id {
        out.push(format!("{KEY_VOLUME_ID}{v}"));
    }
    if let Some(t) = &input.volume_title_snapshot {
        out.push(format!("{KEY_VOLUME_TITLE}{t}"));
    }
    out
}

fn build_volume_payload(id: Uuid, input: &NewVolume) -> serde_json::Value {
    let mut keywords = vec![
        format!("{KEY_PATIENT_ID}{}", input.patient_id),
        format!("{KEY_PATIENT_NAME}{}", input.patient_name_snapshot),
    ];
    if let Some(p) = &input.cabinet_path_snapshot {
        keywords.push(format!("{KEY_CABINET_PATH}{p}"));
    }
    json!({
        "id": id,
        "name": input.title,
        "thing_type": { "Other": VOLUME_TYPE },
        "contained_in_thing": input.cabinet_id,
        "keywords": keywords,
        "identifiers": [
            {
                "property_id": { "Custom": NHS_PROPERTY_ID },
                "value": input.nhs_number_snapshot,
                "name": "NHS Number",
                "url": "https://fhir.nhs.uk/Id/nhs-number",
            }
        ],
        "amenity_features": [],
        "opening_hours": [],
    })
}

fn build_payload(id: Uuid, input: &NewFolder) -> serde_json::Value {
    json!({
        "id": id,
        "name": input.title,
        "description": input.description_value(),
        "thing_type": { "Other": CASE_FILE_TYPE },
        "contained_in_thing": input.cabinet_id,
        "keywords": build_keywords(input),
        "identifiers": [
            {
                "property_id": { "Custom": NHS_PROPERTY_ID },
                "value": input.nhs_number_snapshot,
                "name": "NHS Number",
                "url": "https://fhir.nhs.uk/Id/nhs-number",
            }
        ],
        "amenity_features": [],
        "opening_hours": [],
    })
}

impl NewFolder {
    fn description_value(&self) -> serde_json::Value {
        match self.notes.as_deref() {
            Some(n) => serde_json::Value::String(n.to_string()),
            None => serde_json::Value::Null,
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

    async fn search_raw(&self, q: &[(&str, String)]) -> Result<Vec<Folder>, Error> {
        let response = self
            .http
            .get(self.url("/api/things/search"))
            .query(q)
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
        Ok(body
            .data
            .unwrap_or_default()
            .things
            .into_iter()
            .filter_map(ThingDto::into_folder)
            .collect())
    }

    async fn patch_volume(&self, id: Uuid, patch: serde_json::Value) -> Result<Volume, Error> {
        let response = self
            .http
            .patch(self.url(&format!("/api/things/{id}")))
            .json(&patch)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "update returned HTTP {}",
                response.status()
            )));
        }
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .and_then(ThingDto::into_volume)
            .ok_or_else(|| Error::BadResponse("update returned empty data".into()))
    }
}

#[async_trait]
impl Client for HttpClient {
    async fn search(&self, query: &str) -> Result<Vec<Folder>, Error> {
        let q = vec![
            ("q", query.to_string()),
            ("thing_type", CASE_FILE_TYPE.to_string()),
            ("limit", "100".to_string()),
        ];
        self.search_raw(&q).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Folder>, Error> {
        let response = self
            .http
            .get(self.url(&format!("/api/things/{id}")))
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
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        Ok(body.data.and_then(ThingDto::into_folder))
    }

    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<Folder>, Error> {
        let q = vec![
            ("keyword", format!("{KEY_PATIENT_ID}{patient_id}")),
            ("thing_type", CASE_FILE_TYPE.to_string()),
            ("limit", "200".to_string()),
        ];
        self.search_raw(&q).await
    }

    async fn list_for_nhs_number(&self, nhs_number: &str) -> Result<Vec<Folder>, Error> {
        let q = vec![
            ("identifier_type", NHS_PROPERTY_ID.to_string()),
            ("identifier_value", nhs_number.to_string()),
            ("thing_type", CASE_FILE_TYPE.to_string()),
            ("limit", "200".to_string()),
        ];
        self.search_raw(&q).await
    }

    async fn create(&self, input: NewFolder) -> Result<Folder, Error> {
        let id = Uuid::new_v4();
        let payload = build_payload(id, &input);
        let response = self
            .http
            .post(self.url("/api/things"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if response.status() == StatusCode::CONFLICT {
            return Err(Error::DuplicateTitle);
        }
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Transport(format!(
                "create returned non-success: {body}"
            )));
        }
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .and_then(ThingDto::into_folder)
            .ok_or_else(|| Error::BadResponse("create returned empty data".into()))
    }

    async fn update_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Folder, Error> {
        let mut patch = json!({
            "contained_in_thing": cabinet_id,
        });
        if let Some(p) = cabinet_path_snapshot.as_ref() {
            // The Thing service replaces the full keyword list on PATCH; we
            // include the cabinet_path keyword so the snapshot stays in sync.
            patch["keywords"] = json!([format!("{KEY_CABINET_PATH}{p}")]);
        }
        let response = self
            .http
            .patch(self.url(&format!("/api/things/{id}")))
            .json(&patch)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "update returned HTTP {}",
                response.status()
            )));
        }
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .and_then(ThingDto::into_folder)
            .ok_or_else(|| Error::BadResponse("update returned empty data".into()))
    }

    async fn create_volume(&self, input: NewVolume) -> Result<Volume, Error> {
        let id = Uuid::new_v4();
        let payload = build_volume_payload(id, &input);
        let response = self
            .http
            .post(self.url("/api/things"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if response.status() == StatusCode::CONFLICT {
            return Err(Error::DuplicateTitle);
        }
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Transport(format!(
                "create volume returned non-success: {body}"
            )));
        }
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .and_then(ThingDto::into_volume)
            .ok_or_else(|| Error::BadResponse("create volume returned empty data".into()))
    }

    async fn find_volume_by_id(&self, id: Uuid) -> Result<Option<Volume>, Error> {
        let response = self
            .http
            .get(self.url(&format!("/api/things/{id}")))
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
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        Ok(body.data.and_then(ThingDto::into_volume))
    }

    async fn list_volumes(&self) -> Result<Vec<Volume>, Error> {
        let response = self
            .http
            .get(self.url("/api/things/search"))
            .query(&[("thing_type", VOLUME_TYPE), ("limit", "200")])
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
        Ok(body
            .data
            .unwrap_or_default()
            .things
            .into_iter()
            .filter_map(ThingDto::into_volume)
            .collect())
    }

    async fn rename_volume(&self, id: Uuid, title: String) -> Result<Volume, Error> {
        self.patch_volume(id, json!({ "name": title })).await
    }

    async fn update_volume_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Volume, Error> {
        let mut patch = json!({ "contained_in_thing": cabinet_id });
        if let Some(p) = cabinet_path_snapshot.as_ref() {
            patch["keywords"] = json!([format!("{KEY_CABINET_PATH}{p}")]);
        }
        self.patch_volume(id, patch).await
    }

    async fn set_folder_volume(
        &self,
        folder_id: Uuid,
        volume_id: Option<Uuid>,
        volume_title_snapshot: Option<String>,
    ) -> Result<Folder, Error> {
        let mut keywords: Vec<String> = Vec::new();
        if let Some(v) = volume_id {
            keywords.push(format!("{KEY_VOLUME_ID}{v}"));
        }
        if let Some(t) = &volume_title_snapshot {
            keywords.push(format!("{KEY_VOLUME_TITLE}{t}"));
        }
        let patch = json!({ "keywords": keywords });
        let response = self
            .http
            .patch(self.url(&format!("/api/things/{folder_id}")))
            .json(&patch)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "update returned HTTP {}",
                response.status()
            )));
        }
        let body: ApiResponse<ThingDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .and_then(ThingDto::into_folder)
            .ok_or_else(|| Error::BadResponse("update returned empty data".into()))
    }
}
