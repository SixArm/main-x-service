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
    /// Upstream `Event` UUID (the move's stable identifier).
    pub id: Uuid,
    /// Folder that moved (a Main Thing Service `Thing` UUID).
    pub folder_id: Uuid,
    /// Patient the folder belongs to (a Main Patient Service UUID).
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot, formatted `XXX XXX XXXX`.
    pub nhs_number: String,
    /// Patient name snapshot at the time of the move.
    pub patient_name: String,
    /// Folder title snapshot (the upstream `Event.name`).
    pub folder_title: String,
    /// Cabinet the folder moved *from* (a Main Place Service UUID), if known.
    pub from_cabinet_id: Option<Uuid>,
    /// Cabinet the folder moved *to* (a Main Place Service UUID), if known.
    pub to_cabinet_id: Option<Uuid>,
    /// Human-readable label of the origin cabinet, snapshotted for display.
    pub from_cabinet_label: String,
    /// Human-readable label of the destination cabinet, snapshotted for display.
    pub to_cabinet_label: String,
    /// Worker who performed the move (a Main Worker Service UUID), if recorded.
    pub worker_id: Option<Uuid>,
    /// Worker name snapshot — who moved it, in plain text.
    pub moved_by: String,
    /// Worker role snapshot (e.g. `nurse`) at move time, if recorded.
    pub worker_role_snapshot: Option<String>,
    /// When the move happened (upstream `Event.start_date`).
    pub moved_at: DateTime<Utc>,
    /// Free-text reason for the move, if given (upstream `Event.description`).
    pub reason: Option<String>,
}

/// Input for recording a new folder move. Mirrors [`MoveEvent`] minus the
/// server-assigned `id` and `moved_at`, which the service fills in.
#[derive(Debug, Clone)]
pub struct RecordMove {
    /// Folder being moved (a Main Thing Service `Thing` UUID).
    pub folder_id: Uuid,
    /// Patient the folder belongs to (a Main Patient Service UUID).
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot to store on the event.
    pub nhs_number: String,
    /// Patient name snapshot to store on the event.
    pub patient_name: String,
    /// Folder title snapshot (becomes the upstream `Event.name`).
    pub folder_title: String,
    /// Origin cabinet UUID (a Main Place Service UUID), if known.
    pub from_cabinet_id: Option<Uuid>,
    /// Destination cabinet UUID (a Main Place Service UUID), if known.
    pub to_cabinet_id: Option<Uuid>,
    /// Origin cabinet label snapshot for display.
    pub from_cabinet_label: String,
    /// Destination cabinet label snapshot for display.
    pub to_cabinet_label: String,
    /// Worker who performed the move (a Main Worker Service UUID), if recorded.
    pub worker_id: Option<Uuid>,
    /// Worker name snapshot — who moved it.
    pub moved_by: String,
    /// Worker role snapshot at move time, if recorded.
    pub worker_role_snapshot: Option<String>,
    /// Free-text reason for the move, if any.
    pub reason: Option<String>,
}

/// Errors raised by the Main Event Service client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Network/transport failure, or a non-success HTTP status from the service.
    #[error("Main Event Service request failed: {0}")]
    Transport(String),
    /// The service replied but the body could not be parsed/mapped as expected.
    #[error("Main Event Service returned an unexpected response: {0}")]
    BadResponse(String),
}

/// Flattens our typed [`Error`] into a `loco_rs::Error::Message` so
/// controllers can `?`-propagate it into Loco's error type.
impl From<Error> for loco_rs::Error {
    fn from(value: Error) -> Self {
        loco_rs::Error::Message(value.to_string())
    }
}

#[async_trait]
/// The move-event operations this crate needs from the upstream
/// main-event-service. A trait so tests can substitute a fake without a
/// live HTTP dependency.
pub trait Client: Send + Sync {
    /// Record a folder move. Returns the persisted event with its
    /// freshly-assigned UUID and `moved_at` timestamp.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn record(&self, input: RecordMove) -> Result<MoveEvent, Error>;

    /// All folder moves, newest first.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_all(&self) -> Result<Vec<MoveEvent>, Error>;

    /// The most recent `limit` folder moves.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_recent(&self, limit: u32) -> Result<Vec<MoveEvent>, Error>;

    /// All moves for a specific folder, newest first.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_for_folder(&self, folder_id: Uuid) -> Result<Vec<MoveEvent>, Error>;

    /// All moves for a specific patient (by Main Patient Service UUID).
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<MoveEvent>, Error>;
}
