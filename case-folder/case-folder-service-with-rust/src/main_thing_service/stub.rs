//! In-memory Main Thing Service for unit + integration tests.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{Client, Error, Folder, NewFolder, NewVolume, Volume};
use crate::nhs::normalise_nhs_number;

#[derive(Default)]
pub struct StubClient {
    folders: Mutex<Vec<Folder>>,
    volumes: Mutex<Vec<Volume>>,
}

impl StubClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, folders: Vec<Folder>) {
        let mut guard = self.folders.lock().unwrap();
        for f in folders {
            if !guard.iter().any(|g| g.id == f.id) {
                guard.push(f);
            }
        }
    }

    pub fn snapshot(&self) -> Vec<Folder> {
        self.folders.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.folders.lock().unwrap().clear();
        self.volumes.lock().unwrap().clear();
    }
}

#[async_trait]
impl Client for StubClient {
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

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Folder>, Error> {
        let guard = self.folders.lock().unwrap();
        Ok(guard.iter().find(|f| f.id == id).cloned())
    }

    async fn list_for_patient(&self, patient_id: Uuid) -> Result<Vec<Folder>, Error> {
        let guard = self.folders.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|f| f.patient_id == patient_id)
            .cloned()
            .collect())
    }

    async fn list_for_nhs_number(&self, nhs_number: &str) -> Result<Vec<Folder>, Error> {
        let want = normalise_nhs_number(nhs_number);
        let guard = self.folders.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|f| normalise_nhs_number(&f.nhs_number_snapshot) == want)
            .cloned()
            .collect())
    }

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

    async fn find_volume_by_id(&self, id: Uuid) -> Result<Option<Volume>, Error> {
        let guard = self.volumes.lock().unwrap();
        Ok(guard.iter().find(|v| v.id == id).cloned())
    }

    async fn list_volumes(&self) -> Result<Vec<Volume>, Error> {
        Ok(self.volumes.lock().unwrap().clone())
    }

    async fn rename_volume(&self, id: Uuid, title: String) -> Result<Volume, Error> {
        let mut guard = self.volumes.lock().unwrap();
        let volume = guard
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| Error::BadResponse(format!("volume {id} not found")))?;
        volume.title = title;
        Ok(volume.clone())
    }

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
