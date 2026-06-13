//! HTTP implementation of the Main Event Service [`Client`].
//!
//! Talks to the upstream REST surface at `/api/v1/events[/...]`. The
//! upstream service exposes a real working API (unlike Thing). The
//! tracker stores folder-move events as schema.org Event records with
//! `event_type = Other("FolderMove")` and packs the move-specific
//! fields into the Event's `keywords` array.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::{Client, Error, MoveEvent, RecordMove};

/// Marks the Event as a folder move. Anything else is ignored.
const EVENT_TYPE: &str = "FolderMove";

const KEY_FOLDER_ID: &str = "folder_id=";
const KEY_PATIENT_ID: &str = "patient_id=";
const KEY_NHS: &str = "nhs_number=";
const KEY_PATIENT_NAME: &str = "patient_name=";
const KEY_FROM_CABINET_ID: &str = "from_cabinet_id=";
const KEY_TO_CABINET_ID: &str = "to_cabinet_id=";
const KEY_FROM_CABINET_LABEL: &str = "from_cabinet_label=";
const KEY_TO_CABINET_LABEL: &str = "to_cabinet_label=";
const KEY_WORKER_ID: &str = "worker_id=";
const KEY_MOVED_BY: &str = "moved_by=";
const KEY_WORKER_ROLE: &str = "worker_role=";

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
    events: Vec<EventDto>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventDto {
    id: Uuid,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    event_type: Option<serde_json::Value>,
    #[serde(default)]
    start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    keywords: Vec<String>,
}

fn is_folder_move(value: &Option<serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::String(s)) => s == EVENT_TYPE,
        Some(serde_json::Value::Object(map)) => map
            .get("Other")
            .and_then(|v| v.as_str())
            .map(|s| s == EVENT_TYPE)
            .unwrap_or(false),
        _ => false,
    }
}

impl EventDto {
    fn into_move_event(self) -> Option<MoveEvent> {
        if !is_folder_move(&self.event_type) {
            return None;
        }
        let moved_at = self.start_date.unwrap_or_else(Utc::now);
        let mut folder_id: Option<Uuid> = None;
        let mut patient_id: Option<Uuid> = None;
        let mut nhs_number = String::new();
        let mut patient_name = String::new();
        let mut from_cabinet_id: Option<Uuid> = None;
        let mut to_cabinet_id: Option<Uuid> = None;
        let mut from_cabinet_label = String::new();
        let mut to_cabinet_label = String::new();
        let mut worker_id: Option<Uuid> = None;
        let mut moved_by = String::new();
        let mut worker_role: Option<String> = None;
        for kw in &self.keywords {
            if let Some(v) = kw.strip_prefix(KEY_FOLDER_ID) {
                folder_id = Uuid::parse_str(v).ok();
            } else if let Some(v) = kw.strip_prefix(KEY_PATIENT_ID) {
                patient_id = Uuid::parse_str(v).ok();
            } else if let Some(v) = kw.strip_prefix(KEY_NHS) {
                nhs_number = v.to_string();
            } else if let Some(v) = kw.strip_prefix(KEY_PATIENT_NAME) {
                patient_name = v.to_string();
            } else if let Some(v) = kw.strip_prefix(KEY_FROM_CABINET_ID) {
                from_cabinet_id = Uuid::parse_str(v).ok();
            } else if let Some(v) = kw.strip_prefix(KEY_TO_CABINET_ID) {
                to_cabinet_id = Uuid::parse_str(v).ok();
            } else if let Some(v) = kw.strip_prefix(KEY_FROM_CABINET_LABEL) {
                from_cabinet_label = v.to_string();
            } else if let Some(v) = kw.strip_prefix(KEY_TO_CABINET_LABEL) {
                to_cabinet_label = v.to_string();
            } else if let Some(v) = kw.strip_prefix(KEY_WORKER_ID) {
                worker_id = Uuid::parse_str(v).ok();
            } else if let Some(v) = kw.strip_prefix(KEY_MOVED_BY) {
                moved_by = v.to_string();
            } else if let Some(v) = kw.strip_prefix(KEY_WORKER_ROLE) {
                worker_role = Some(v.to_string());
            }
        }
        Some(MoveEvent {
            id: self.id,
            folder_id: folder_id.unwrap_or_else(Uuid::nil),
            patient_id: patient_id.unwrap_or_else(Uuid::nil),
            nhs_number,
            patient_name,
            folder_title: self.name,
            from_cabinet_id,
            to_cabinet_id,
            from_cabinet_label,
            to_cabinet_label,
            worker_id,
            moved_by,
            worker_role_snapshot: worker_role,
            moved_at,
            reason: self.description,
        })
    }
}

fn build_keywords(input: &RecordMove) -> Vec<String> {
    let mut out = vec![
        format!("{KEY_FOLDER_ID}{}", input.folder_id),
        format!("{KEY_PATIENT_ID}{}", input.patient_id),
        format!("{KEY_NHS}{}", input.nhs_number),
        format!("{KEY_PATIENT_NAME}{}", input.patient_name),
        format!("{KEY_FROM_CABINET_LABEL}{}", input.from_cabinet_label),
        format!("{KEY_TO_CABINET_LABEL}{}", input.to_cabinet_label),
        format!("{KEY_MOVED_BY}{}", input.moved_by),
    ];
    if let Some(id) = input.from_cabinet_id {
        out.push(format!("{KEY_FROM_CABINET_ID}{id}"));
    }
    if let Some(id) = input.to_cabinet_id {
        out.push(format!("{KEY_TO_CABINET_ID}{id}"));
    }
    if let Some(id) = input.worker_id {
        out.push(format!("{KEY_WORKER_ID}{id}"));
    }
    if let Some(role) = &input.worker_role_snapshot {
        out.push(format!("{KEY_WORKER_ROLE}{role}"));
    }
    out
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

    async fn search_raw(&self, q: &[(&str, String)]) -> Result<Vec<MoveEvent>, Error> {
        let response = self
            .http
            .get(self.url("/api/v1/events/search"))
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
        let mut events: Vec<MoveEvent> = body
            .data
            .unwrap_or_default()
            .events
            .into_iter()
            .filter_map(EventDto::into_move_event)
            .collect();
        events.sort_by_key(|e| std::cmp::Reverse(e.moved_at));
        Ok(events)
    }
}

#[async_trait]
impl Client for HttpClient {
    async fn record(&self, input: RecordMove) -> Result<MoveEvent, Error> {
        let now = Utc::now();
        let payload = json!({
            "id": Uuid::new_v4(),
            "name": input.folder_title,
            "description": input.reason,
            "event_type": { "Other": EVENT_TYPE },
            "start_date": now,
            "end_date": now,
            "event_status": "Scheduled",
            "event_attendance_mode": "Offline",
            "keywords": build_keywords(&input),
            "identifiers": [],
            "alternate_names": [],
            "image": [],
            "same_as": [],
            "location": [],
            "organizers": [],
            "performers": [],
            "attendees": [],
            "sponsors": [],
            "funders": [],
            "contributors": [],
            "about": [],
            "works": [],
            "sub_events": [],
            "offers": [],
            "links": [],
            "in_language": [],
            "active": true,
        });
        let response = self
            .http
            .post(self.url("/api/v1/events"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Transport(format!(
                "record returned non-success: {body}"
            )));
        }
        let body: ApiResponse<EventDto> = response
            .json()
            .await
            .map_err(|e| Error::BadResponse(e.to_string()))?;
        body.data
            .and_then(EventDto::into_move_event)
            .ok_or_else(|| Error::BadResponse("record returned empty data".into()))
    }

    async fn list_all(&self) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("limit", "500".to_string()),
            ("order", "moved_at_desc".to_string()),
        ];
        self.search_raw(&q).await
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("limit", limit.to_string()),
            ("order", "moved_at_desc".to_string()),
        ];
        self.search_raw(&q).await
    }

    async fn list_for_folder(&self, folder_id: Uuid) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("keyword", format!("{KEY_FOLDER_ID}{folder_id}")),
            ("limit", "500".to_string()),
        ];
        self.search_raw(&q).await
    }

    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("keyword", format!("{KEY_PATIENT_ID}{patient_id}")),
            ("limit", "500".to_string()),
        ];
        self.search_raw(&q).await
    }
}
