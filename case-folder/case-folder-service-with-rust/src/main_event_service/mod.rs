//! Client for the Main Event Service.
//!
//! The Main Event Service is an external Rust service that owns the
//! registry of time-bounded `Event` records (schema.org/Event). Case
//! Tracking uses it to store **folder-move events** — each
//! recorded folder movement is an `Event` with
//! `event_type = Other("FolderMove")`.
//!
//! ## MoveEvent ↔ Event mapping
//!
//! | Tracker field            | Event field                                              |
//! | ------------------------ | -------------------------------------------------------- |
//! | `id`                     | `id`                                                     |
//! | `moved_at`               | `start_date` (also `end_date` — instantaneous)           |
//! | `folder_title`           | `name`                                                   |
//! | `reason`                 | `description`                                            |
//! | `folder_id`              | `keywords["folder_id=<uuid>"]`                           |
//! | `patient_id`             | `keywords["patient_id=<uuid>"]`                          |
//! | `nhs_number`             | `keywords["nhs_number=<XXX XXX XXXX>"]`                  |
//! | `patient_name`           | `keywords["patient_name=<name>"]`                        |
//! | `from_cabinet_id`        | `keywords["from_cabinet_id=<uuid>"]`                     |
//! | `to_cabinet_id`          | `keywords["to_cabinet_id=<uuid>"]`                       |
//! | `from_cabinet_label`     | `keywords["from_cabinet_label=<label>"]`                 |
//! | `to_cabinet_label`       | `keywords["to_cabinet_label=<label>"]`                   |
//! | `worker_id`              | `keywords["worker_id=<uuid>"]`                           |
//! | `moved_by`               | `keywords["moved_by=<name>"]`                            |
//! | `worker_role_snapshot`   | `keywords["worker_role=<role>"]`                         |
//!
//! Keywords are an unfortunate dumping ground but they're the only
//! string-bag field on `Event` that round-trips through the upstream
//! service without custom typed columns. The HTTP client packs and
//! parses them; controllers never see raw keywords.

pub mod http;
pub mod stub;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tracker-side projection of an `Event` representing a single
/// folder-move. The HTTP client serialises this to/from the upstream
/// `Event` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveEvent {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub patient_id: Uuid,
    pub nhs_number: String,
    pub patient_name: String,
    pub folder_title: String,
    pub from_cabinet_id: Option<Uuid>,
    pub to_cabinet_id: Option<Uuid>,
    pub from_cabinet_label: String,
    pub to_cabinet_label: String,
    pub worker_id: Option<Uuid>,
    pub moved_by: String,
    pub worker_role_snapshot: Option<String>,
    pub moved_at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordMove {
    pub folder_id: Uuid,
    pub patient_id: Uuid,
    pub nhs_number: String,
    pub patient_name: String,
    pub folder_title: String,
    pub from_cabinet_id: Option<Uuid>,
    pub to_cabinet_id: Option<Uuid>,
    pub from_cabinet_label: String,
    pub to_cabinet_label: String,
    pub worker_id: Option<Uuid>,
    pub moved_by: String,
    pub worker_role_snapshot: Option<String>,
    pub reason: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Main Event Service request failed: {0}")]
    Transport(String),
    #[error("Main Event Service returned an unexpected response: {0}")]
    BadResponse(String),
}

impl From<Error> for loco_rs::Error {
    fn from(value: Error) -> Self {
        loco_rs::Error::Message(value.to_string())
    }
}

#[async_trait]
pub trait Client: Send + Sync {
    /// Record a folder move. Returns the persisted event with its
    /// freshly-assigned UUID and `moved_at` timestamp.
    async fn record(&self, input: RecordMove) -> Result<MoveEvent, Error>;

    /// All folder moves, newest first.
    async fn list_all(&self) -> Result<Vec<MoveEvent>, Error>;

    /// The most recent `limit` folder moves.
    async fn list_recent(&self, limit: u32) -> Result<Vec<MoveEvent>, Error>;

    /// All moves for a specific folder, newest first.
    async fn list_for_folder(&self, folder_id: Uuid) -> Result<Vec<MoveEvent>, Error>;

    /// All moves for a specific patient (by Main Patient Service UUID).
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<MoveEvent>, Error>;
}
