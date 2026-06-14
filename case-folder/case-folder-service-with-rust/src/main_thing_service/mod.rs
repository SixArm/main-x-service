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
    /// Upstream `Thing` UUID (the folder's stable identifier).
    pub id: Uuid,
    /// Folder title (the upstream `Thing.name`).
    pub title: String,
    /// Patient the folder belongs to (a Main Patient Service UUID).
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot (stored as a Thing identifier).
    pub nhs_number_snapshot: String,
    /// Patient name snapshot for display without a patient-service hop.
    pub patient_name_snapshot: String,
    /// Cabinet the folder currently sits in (a Main Place Service UUID),
    /// or `None` when in transit. Maps to `Thing.contained_in_thing`.
    pub cabinet_id: Option<Uuid>,
    /// Human-readable cabinet path snapshot for list rendering.
    pub cabinet_path_snapshot: Option<String>,
    /// Free-text notes (the upstream `Thing.description`).
    pub notes: Option<String>,
    /// The volume this folder belongs to, if any (see [`Volume`]).
    #[serde(default)]
    pub volume_id: Option<Uuid>,
    /// Volume title snapshot for display, when the folder is in a volume.
    #[serde(default)]
    pub volume_title_snapshot: Option<String>,
}

/// Input for creating a new folder. Mirrors [`Folder`] minus the
/// server-assigned `id`.
#[derive(Debug, Clone)]
pub struct NewFolder {
    /// Patient the folder belongs to (a Main Patient Service UUID).
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot to store as a Thing identifier.
    pub nhs_number_snapshot: String,
    /// Patient name snapshot to store as a keyword.
    pub patient_name_snapshot: String,
    /// Folder title (becomes the upstream `Thing.name`).
    pub title: String,
    /// Cabinet to file the folder in, if known.
    pub cabinet_id: Option<Uuid>,
    /// Cabinet path snapshot for display.
    pub cabinet_path_snapshot: Option<String>,
    /// Free-text notes (becomes the upstream `Thing.description`).
    pub notes: Option<String>,
    /// Volume to assign the folder to on creation, if any.
    pub volume_id: Option<Uuid>,
    /// Volume title snapshot, when `volume_id` is set.
    pub volume_title_snapshot: Option<String>,
}

/// Tracker-side projection of a `Thing` representing a **volume** — a
/// movable bundle of one patient's folders. Stored with
/// `thing_type = Other("Volume")`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    /// Upstream `Thing` UUID (the volume's stable identifier).
    pub id: Uuid,
    /// Volume title (the upstream `Thing.name`).
    pub title: String,
    /// Patient the volume belongs to (a Main Patient Service UUID).
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot (stored as a Thing identifier).
    pub nhs_number_snapshot: String,
    /// Patient name snapshot for display.
    pub patient_name_snapshot: String,
    /// Cabinet the volume currently sits in (a Main Place Service UUID),
    /// or `None` when in transit.
    pub cabinet_id: Option<Uuid>,
    /// Human-readable cabinet path snapshot for display.
    pub cabinet_path_snapshot: Option<String>,
}

/// Input for creating a new volume. Mirrors [`Volume`] minus the
/// server-assigned `id`.
#[derive(Debug, Clone)]
pub struct NewVolume {
    /// Patient the volume belongs to (a Main Patient Service UUID).
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot to store as a Thing identifier.
    pub nhs_number_snapshot: String,
    /// Patient name snapshot to store as a keyword.
    pub patient_name_snapshot: String,
    /// Volume title (becomes the upstream `Thing.name`).
    pub title: String,
    /// Cabinet to file the volume in, if known.
    pub cabinet_id: Option<Uuid>,
    /// Cabinet path snapshot for display.
    pub cabinet_path_snapshot: Option<String>,
}

/// Errors raised by the Main Thing Service client.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Network/transport failure, or a non-success HTTP status.
    #[error("Main Thing Service request failed: {0}")]
    Transport(String),
    /// The service replied but the body could not be parsed/mapped.
    #[error("Main Thing Service returned an unexpected response: {0}")]
    BadResponse(String),
    /// A folder/volume with this `(patient_id, title)` already exists
    /// (mapped from an upstream `409 Conflict`).
    #[error("a folder with this title already exists for this patient")]
    DuplicateTitle,
}

/// Flattens our typed [`Error`] into a `loco_rs::Error::Message` for
/// `?`-propagation in controllers.
impl From<Error> for loco_rs::Error {
    fn from(value: Error) -> Self {
        loco_rs::Error::Message(value.to_string())
    }
}

#[async_trait]
pub trait Client: Send + Sync {
    /// Free-text search by folder title, patient name, or NHS Number.
    /// Empty `query` returns all folders.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn search(&self, query: &str) -> Result<Vec<Folder>, Error>;

    /// Look up by Thing UUID.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Folder>, Error>;

    /// Folders for a specific patient (by Main Patient Service UUID).
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<Folder>, Error>;

    /// Folders that snapshot a given NHS Number. Used by the move
    /// workflow's "lookup by NHS Number" step.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_for_nhs_number(&self, nhs_number: &str) -> Result<Vec<Folder>, Error>;

    /// Create a new folder. Returns `Error::DuplicateTitle` when the
    /// service already holds a folder with this `(patient_id, title)`.
    ///
    /// # Errors
    /// Returns [`Error::DuplicateTitle`] on a collision, or other
    /// [`Error`] variants on transport/parse failure.
    async fn create(&self, input: NewFolder) -> Result<Folder, Error>;

    /// Move a folder to a new cabinet (or set it in-transit). Updates
    /// `cabinet_id` + `cabinet_path_snapshot` on the upstream Thing.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn update_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Folder, Error>;

    // --- Volumes (Things with `thing_type = Other("Volume")`) ---

    /// Create a new volume. `Error::DuplicateTitle` when the patient
    /// already has a volume with this title.
    ///
    /// # Errors
    /// Returns [`Error::DuplicateTitle`] on a collision, or other
    /// [`Error`] variants on transport/parse failure.
    async fn create_volume(&self, input: NewVolume) -> Result<Volume, Error>;

    /// Look up a volume by id.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn find_volume_by_id(&self, id: Uuid) -> Result<Option<Volume>, Error>;

    /// All volumes (controllers filter by patient client-side).
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn list_volumes(&self) -> Result<Vec<Volume>, Error>;

    /// Rename a volume.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn rename_volume(&self, id: Uuid, title: String) -> Result<Volume, Error>;

    /// Move a volume's own location pointer.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn update_volume_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Volume, Error>;

    /// Assign (`Some`) or clear (`None`) a folder's volume membership.
    ///
    /// # Errors
    /// Returns [`Error`] on transport failure or an unparseable response.
    async fn set_folder_volume(
        &self,
        folder_id: Uuid,
        volume_id: Option<Uuid>,
        volume_title_snapshot: Option<String>,
    ) -> Result<Folder, Error>;
}
