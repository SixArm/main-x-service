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

// The `KEY_*` constants are the keyword-bag prefixes. The upstream Event
// service has no typed columns for move-specific fields, so the tracker
// packs them as `"<prefix><value>"` strings into the Event's `keywords`
// array and parses them back out on the way in. Each prefix ends in `=`.

/// Keyword prefix for the moved folder's UUID (e.g. `"folder_id=<uuid>"`).
const KEY_FOLDER_ID: &str = "folder_id=";
/// Keyword prefix for the patient's UUID.
const KEY_PATIENT_ID: &str = "patient_id=";
/// Keyword prefix for the NHS Number snapshot.
const KEY_NHS: &str = "nhs_number=";
/// Keyword prefix for the patient-name snapshot.
const KEY_PATIENT_NAME: &str = "patient_name=";
/// Keyword prefix for the origin cabinet UUID.
const KEY_FROM_CABINET_ID: &str = "from_cabinet_id=";
/// Keyword prefix for the destination cabinet UUID.
const KEY_TO_CABINET_ID: &str = "to_cabinet_id=";
/// Keyword prefix for the origin cabinet label snapshot.
const KEY_FROM_CABINET_LABEL: &str = "from_cabinet_label=";
/// Keyword prefix for the destination cabinet label snapshot.
const KEY_TO_CABINET_LABEL: &str = "to_cabinet_label=";
/// Keyword prefix for the worker UUID.
const KEY_WORKER_ID: &str = "worker_id=";
/// Keyword prefix for the worker-name (who moved it) snapshot.
const KEY_MOVED_BY: &str = "moved_by=";
/// Keyword prefix for the worker-role snapshot.
const KEY_WORKER_ROLE: &str = "worker_role=";

/// The upstream `{success, data, error}` envelope. We only consume `data`.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    /// Payload, present on success; `None` when the service returns no data.
    data: Option<T>,
    /// Service-reported success flag. Unused by this client (status is
    /// authoritative), hence `#[allow(dead_code)]`.
    #[serde(default)]
    #[allow(dead_code)]
    success: bool,
}

/// The `data` shape returned by the search endpoint: a list of events.
#[derive(Debug, Deserialize, Default)]
struct SearchEnvelope {
    /// Raw event DTOs; non-folder-move events are filtered out downstream.
    #[serde(default)]
    events: Vec<EventDto>,
}

/// Wire shape of an upstream `Event`, trimmed to the fields the tracker
/// reads. Move-specific data is carried in `keywords` (see `KEY_*`).
#[derive(Debug, Serialize, Deserialize)]
struct EventDto {
    /// Event UUID.
    id: Uuid,
    /// Event name — the folder title snapshot.
    name: String,
    /// Event description — the move reason, if any.
    #[serde(default)]
    description: Option<String>,
    /// Raw `event_type`; may be a bare string or a `{"Other": "..."}` object.
    #[serde(default)]
    event_type: Option<serde_json::Value>,
    /// When the move happened.
    #[serde(default)]
    start_date: Option<DateTime<Utc>>,
    /// Packed move fields (see the `KEY_*` prefixes).
    #[serde(default)]
    keywords: Vec<String>,
}

/// Returns `true` when `event_type` marks the event as a folder move.
/// Handles both the bare-string (`"FolderMove"`) and tagged-enum
/// (`{"Other": "FolderMove"}`) JSON forms the upstream service may emit.
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
    /// Projects this DTO into a tracker [`MoveEvent`], or `None` when the
    /// event is not a folder move. Parses the `KEY_*`-prefixed keywords
    /// back into typed fields; missing UUID keywords fall back to nil.
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

/// Packs a [`RecordMove`] into the `KEY_*`-prefixed keyword strings the
/// upstream Event service round-trips. Always-present fields go first;
/// optional UUID/role fields are appended only when set.
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

/// REST [`Client`] for the Main Event Service.
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
    /// Panics if the underlying `reqwest` client cannot be built (only on
    /// a misconfigured TLS backend, not in normal operation).
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

    /// Runs a `GET /api/v1/events/search` with query params `q`, maps the
    /// folder-move events, and sorts them newest-first.
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status and
    /// [`Error::BadResponse`] when the body cannot be parsed.
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
        // `Reverse` sorts descending by timestamp: newest move first.
        events.sort_by_key(|e| std::cmp::Reverse(e.moved_at));
        Ok(events)
    }
}

#[async_trait]
impl Client for HttpClient {
    /// `POST /api/v1/events` — creates a `FolderMove` event. Builds the
    /// full schema.org `Event` payload (most arrays empty), packing the
    /// move fields into `keywords`. Returns the projected [`MoveEvent`].
    ///
    /// # Errors
    /// Returns [`Error::Transport`] on a non-success status and
    /// [`Error::BadResponse`] when the response has no usable `data`.
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

    /// `GET /api/v1/events/search?event_type=FolderMove&limit=500` — all
    /// folder moves (capped at 500), newest first.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_all(&self) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("limit", "500".to_string()),
            ("order", "moved_at_desc".to_string()),
        ];
        self.search_raw(&q).await
    }

    /// `GET /api/v1/events/search?event_type=FolderMove&limit=<limit>` —
    /// the most recent `limit` folder moves.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_recent(&self, limit: u32) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("limit", limit.to_string()),
            ("order", "moved_at_desc".to_string()),
        ];
        self.search_raw(&q).await
    }

    /// `GET /api/v1/events/search` filtered by the `folder_id=` keyword —
    /// all moves for one folder, newest first.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_for_folder(&self, folder_id: Uuid) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("keyword", format!("{KEY_FOLDER_ID}{folder_id}")),
            ("limit", "500".to_string()),
        ];
        self.search_raw(&q).await
    }

    /// `GET /api/v1/events/search` filtered by the `patient_id=` keyword —
    /// all moves for one patient, newest first.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<MoveEvent>, Error> {
        let q = vec![
            ("event_type", EVENT_TYPE.to_string()),
            ("keyword", format!("{KEY_PATIENT_ID}{patient_id}")),
            ("limit", "500".to_string()),
        ];
        self.search_raw(&q).await
    }
}
