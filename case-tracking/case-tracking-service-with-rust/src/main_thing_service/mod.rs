//! Client for the Main Thing Service.
//!
//! The Main Thing Service is an external Rust service that owns the
//! registry of generic "thing" records. Case Tracking uses it
//! to store **folders** — each folder is a `Thing` with
//! `thing_type = Other("CaseFile")`.
//!
//! ## Folder ↔ Thing mapping
//!
//! | Tracker field          | Thing field                                                   |
//! | ---------------------- | ------------------------------------------------------------- |
//! | `id`                   | `id`                                                          |
//! | `title`                | `name`                                                        |
//! | `patient_id`           | `keywords["patient_id=<uuid>"]`                               |
//! | `nhs_number_snapshot`  | `identifiers[{ property_id: Custom("nhs-number"), value }]`   |
//! | `patient_name_snapshot`| `keywords["patient_name=<name>"]`                             |
//! | `cabinet_id`           | `contained_in_thing` (cross-service ref to a Place UUID)      |
//! | `cabinet_path_snapshot`| `keywords["cabinet_path=<path>"]`                             |
//! | `notes`                | `description`                                                 |
//!
//! `status` and `last_moved_at` are *not* on the Thing — they're
//! derived from the move history that lives in the Main Event Service.
//!
//! Implementations live in [`http`] (REST) and [`stub`] (in-memory test
//! double). The [`Client`] trait keeps controllers decoupled from the
//! transport.

pub mod http;
pub mod stub;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tracker-side projection of a `Thing` representing a paper case-file
/// folder. The HTTP client serialises this to/from the upstream
/// `Thing` JSON; the stub keeps it as-is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub title: String,
    pub patient_id: Uuid,
    pub nhs_number_snapshot: String,
    pub patient_name_snapshot: String,
    pub cabinet_id: Option<Uuid>,
    pub cabinet_path_snapshot: Option<String>,
    pub notes: Option<String>,
    /// The volume this folder belongs to, if any (see [`Volume`]).
    #[serde(default)]
    pub volume_id: Option<Uuid>,
    #[serde(default)]
    pub volume_title_snapshot: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewFolder {
    pub patient_id: Uuid,
    pub nhs_number_snapshot: String,
    pub patient_name_snapshot: String,
    pub title: String,
    pub cabinet_id: Option<Uuid>,
    pub cabinet_path_snapshot: Option<String>,
    pub notes: Option<String>,
    pub volume_id: Option<Uuid>,
    pub volume_title_snapshot: Option<String>,
}

/// Tracker-side projection of a `Thing` representing a **volume** — a
/// movable bundle of one patient's folders. Stored with
/// `thing_type = Other("Volume")`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub id: Uuid,
    pub title: String,
    pub patient_id: Uuid,
    pub nhs_number_snapshot: String,
    pub patient_name_snapshot: String,
    pub cabinet_id: Option<Uuid>,
    pub cabinet_path_snapshot: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewVolume {
    pub patient_id: Uuid,
    pub nhs_number_snapshot: String,
    pub patient_name_snapshot: String,
    pub title: String,
    pub cabinet_id: Option<Uuid>,
    pub cabinet_path_snapshot: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Main Thing Service request failed: {0}")]
    Transport(String),
    #[error("Main Thing Service returned an unexpected response: {0}")]
    BadResponse(String),
    #[error("a folder with this title already exists for this patient")]
    DuplicateTitle,
}

impl From<Error> for loco_rs::Error {
    fn from(value: Error) -> Self {
        loco_rs::Error::Message(value.to_string())
    }
}

#[async_trait]
pub trait Client: Send + Sync {
    /// Free-text search by folder title, patient name, or NHS Number.
    /// Empty `query` returns all folders.
    async fn search(&self, query: &str) -> Result<Vec<Folder>, Error>;

    /// Look up by Thing UUID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Folder>, Error>;

    /// Folders for a specific patient (by Main Patient Service UUID).
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<Folder>, Error>;

    /// Folders that snapshot a given NHS Number. Used by the move
    /// workflow's "lookup by NHS Number" step.
    async fn list_for_nhs_number(&self, nhs_number: &str) -> Result<Vec<Folder>, Error>;

    /// Create a new folder. Returns `Error::DuplicateTitle` when the
    /// service already holds a folder with this `(patient_id, title)`.
    async fn create(&self, input: NewFolder) -> Result<Folder, Error>;

    /// Move a folder to a new cabinet (or set it in-transit). Updates
    /// `cabinet_id` + `cabinet_path_snapshot` on the upstream Thing.
    async fn update_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Folder, Error>;

    // --- Volumes (Things with `thing_type = Other("Volume")`) ---

    /// Create a new volume. `Error::DuplicateTitle` when the patient
    /// already has a volume with this title.
    async fn create_volume(&self, input: NewVolume) -> Result<Volume, Error>;

    /// Look up a volume by id.
    async fn find_volume_by_id(&self, id: Uuid) -> Result<Option<Volume>, Error>;

    /// All volumes (controllers filter by patient client-side).
    async fn list_volumes(&self) -> Result<Vec<Volume>, Error>;

    /// Rename a volume.
    async fn rename_volume(&self, id: Uuid, title: String) -> Result<Volume, Error>;

    /// Move a volume's own location pointer.
    async fn update_volume_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Volume, Error>;

    /// Assign (`Some`) or clear (`None`) a folder's volume membership.
    async fn set_folder_volume(
        &self,
        folder_id: Uuid,
        volume_id: Option<Uuid>,
        volume_title_snapshot: Option<String>,
    ) -> Result<Folder, Error>;
}
