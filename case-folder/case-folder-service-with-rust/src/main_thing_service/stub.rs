//! In-memory Main Thing Service for unit + integration tests.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, Error, Folder, NewFolder, NewVolume, Volume};
use crate::nhs::normalise_nhs_number;

/// In-memory test double for the Main Thing Service [`Client`]. Holds
/// folders and volumes in separate `Mutex`-guarded `Vec`s; no network.
#[derive(Default)]
pub struct StubClient {
    /// Stored folders, in insertion order.
    folders: Mutex<Vec<Folder>>,
    /// Stored volumes, in insertion order.
    volumes: Mutex<Vec<Volume>>,
}

impl StubClient {
    /// Creates an empty stub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-loads `folders`, skipping any whose `id` is already present
    /// (dedupe-by-id). Volumes are seeded via the trait `create_volume`.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn seed(&self, folders: Vec<Folder>) {
        let mut guard = self.folders.lock().unwrap();
        for f in folders {
            if !guard.iter().any(|g| g.id == f.id) {
                guard.push(f);
            }
        }
    }

    /// Returns a clone of all stored folders (test inspection helper).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    pub fn snapshot(&self) -> Vec<Folder> {
        self.folders.lock().unwrap().clone()
    }

    /// Removes all stored folders *and* volumes.
    ///
    /// # Panics
    /// Panics if either internal mutex is poisoned.
    pub fn clear(&self) {
        self.folders.lock().unwrap().clear();
        self.volumes.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
    /// Matches folders whose title or patient-name contains `query`
    /// (case-insensitive), or whose NHS Number contains the query digits.
    /// Empty query matches all.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn search(&self, query: &str) -> Result<Vec<Folder>, Error> {
        let q = query.trim().to_lowercase();
        let q_digits = normalise_nhs_number(query);
        let guard = self.folders.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|f| {
                if q.is_empty() {
                    return true;
                }
                f.title.to_lowercase().contains(&q)
                    || f.patient_name_snapshot.to_lowercase().contains(&q)
                    || (!q_digits.is_empty()
                        && normalise_nhs_number(&f.nhs_number_snapshot).contains(&q_digits))
            })
            .cloned()
            .collect())
    }

    /// Finds a folder by UUID.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Folder>, Error> {
        let guard = self.folders.lock().unwrap();
        Ok(guard.iter().find(|f| f.id == id).cloned())
    }

    /// All folders whose `patient_id` matches.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<Folder>, Error> {
        let guard = self.folders.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|f| f.patient_id == patient_id)
            .cloned()
            .collect())
    }

    /// All folders whose NHS Number snapshot matches (normalised digits).
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_for_nhs_number(&self, nhs_number: &str) -> Result<Vec<Folder>, Error> {
        let want = normalise_nhs_number(nhs_number);
        let guard = self.folders.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|f| normalise_nhs_number(&f.nhs_number_snapshot) == want)
            .cloned()
            .collect())
    }

    /// Creates a folder. Returns [`Error::DuplicateTitle`] when one already
    /// exists with the same `(patient_id, title)`; otherwise stores a new
    /// folder with a fresh UUID and returns it.
    ///
    /// # Errors
    /// Returns [`Error::DuplicateTitle`] on a `(patient_id, title)` collision.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn create(&self, input: NewFolder) -> Result<Folder, Error> {
        let mut guard = self.folders.lock().unwrap();
        if guard
            .iter()
            .any(|f| f.patient_id == input.patient_id && f.title == input.title)
        {
            return Err(Error::DuplicateTitle);
        }
        let folder = Folder {
            id: Uuid::new_v4(),
            title: input.title,
            patient_id: input.patient_id,
            nhs_number_snapshot: input.nhs_number_snapshot,
            patient_name_snapshot: input.patient_name_snapshot,
            cabinet_id: input.cabinet_id,
            cabinet_path_snapshot: input.cabinet_path_snapshot,
            notes: input.notes,
            volume_id: input.volume_id,
            volume_title_snapshot: input.volume_title_snapshot,
        };
        guard.push(folder.clone());
        Ok(folder)
    }

    /// Moves a folder in place: updates its `cabinet_id` and
    /// `cabinet_path_snapshot`. Unknown ids yield [`Error::BadResponse`].
    ///
    /// # Errors
    /// Returns [`Error::BadResponse`] when no folder has `id`.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn update_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Folder, Error> {
        let mut guard = self.folders.lock().unwrap();
        let folder = guard
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(|| Error::BadResponse(format!("folder {id} not found")))?;
        folder.cabinet_id = cabinet_id;
        folder.cabinet_path_snapshot = cabinet_path_snapshot;
        Ok(folder.clone())
    }

    /// Creates a volume. Returns [`Error::DuplicateTitle`] when one already
    /// exists with the same `(patient_id, title)`.
    ///
    /// # Errors
    /// Returns [`Error::DuplicateTitle`] on a `(patient_id, title)` collision.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn create_volume(&self, input: NewVolume) -> Result<Volume, Error> {
        let mut guard = self.volumes.lock().unwrap();
        if guard
            .iter()
            .any(|v| v.patient_id == input.patient_id && v.title == input.title)
        {
            return Err(Error::DuplicateTitle);
        }
        let volume = Volume {
            id: Uuid::new_v4(),
            title: input.title,
            patient_id: input.patient_id,
            nhs_number_snapshot: input.nhs_number_snapshot,
            patient_name_snapshot: input.patient_name_snapshot,
            cabinet_id: input.cabinet_id,
            cabinet_path_snapshot: input.cabinet_path_snapshot,
        };
        guard.push(volume.clone());
        Ok(volume)
    }

    /// Finds a volume by UUID.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn find_volume_by_id(&self, id: Uuid) -> Result<Option<Volume>, Error> {
        let guard = self.volumes.lock().unwrap();
        Ok(guard.iter().find(|v| v.id == id).cloned())
    }

    /// All stored volumes.
    ///
    /// # Errors
    /// Infallible; returns `Result` for trait parity.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn list_volumes(&self) -> Result<Vec<Volume>, Error> {
        Ok(self.volumes.lock().unwrap().clone())
    }

    /// Renames a volume in place. Unknown ids yield [`Error::BadResponse`].
    ///
    /// # Errors
    /// Returns [`Error::BadResponse`] when no volume has `id`.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn rename_volume(&self, id: Uuid, title: String) -> Result<Volume, Error> {
        let mut guard = self.volumes.lock().unwrap();
        let volume = guard
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| Error::BadResponse(format!("volume {id} not found")))?;
        volume.title = title;
        Ok(volume.clone())
    }

    /// Moves a volume in place: updates its `cabinet_id` and
    /// `cabinet_path_snapshot`. Unknown ids yield [`Error::BadResponse`].
    ///
    /// # Errors
    /// Returns [`Error::BadResponse`] when no volume has `id`.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn update_volume_cabinet(
        &self,
        id: Uuid,
        cabinet_id: Option<Uuid>,
        cabinet_path_snapshot: Option<String>,
    ) -> Result<Volume, Error> {
        let mut guard = self.volumes.lock().unwrap();
        let volume = guard
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| Error::BadResponse(format!("volume {id} not found")))?;
        volume.cabinet_id = cabinet_id;
        volume.cabinet_path_snapshot = cabinet_path_snapshot;
        Ok(volume.clone())
    }

    /// Sets or clears a folder's volume membership in place. Unknown ids
    /// yield [`Error::BadResponse`].
    ///
    /// # Errors
    /// Returns [`Error::BadResponse`] when no folder has `folder_id`.
    /// # Panics
    /// Panics if the internal mutex is poisoned.
    async fn set_folder_volume(
        &self,
        folder_id: Uuid,
        volume_id: Option<Uuid>,
        volume_title_snapshot: Option<String>,
    ) -> Result<Folder, Error> {
        let mut guard = self.folders.lock().unwrap();
        let folder = guard
            .iter_mut()
            .find(|f| f.id == folder_id)
            .ok_or_else(|| Error::BadResponse(format!("folder {folder_id} not found")))?;
        folder.volume_id = volume_id;
        folder.volume_title_snapshot = volume_title_snapshot;
        Ok(folder.clone())
    }
}
