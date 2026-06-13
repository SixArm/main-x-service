//! Volumes API — a **volume** is a movable bundle of one patient's
//! folders, stored in the Main Thing Service as a `Thing` with
//! `thing_type = Other("Volume")`. See root spec D-11.
//!
//! Moving a volume fans out to the per-folder move machinery: it records
//! a move event and updates the cabinet for every member folder, then
//! updates the volume's own cabinet pointer.

use axum::{
    debug_handler,
    extract::{Path, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    controllers::stats::latest_move_per_folder,
    main_event_service::{Client as MainEventServiceClient, RecordMove},
    main_patient_service::Client as MainPatientServiceClient,
    main_place_service::{label_path, Client as MainPlaceServiceClient},
    main_thing_service::{Client as MainThingServiceClient, NewVolume, Volume},
    main_worker_service::Client as MainWorkerServiceClient,
    nhs::{format_nhs_number, normalise_nhs_number},
    responses::{self, List},
};

#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    pub q: Option<String>,
    pub nhs_number: Option<String>,
}

#[debug_handler]
pub async fn index(
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Query(params): Query<FilterParams>,
) -> Response {
    let volumes = match things.list_volumes().await {
        Ok(v) => v,
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    };
    let folders = things.search("").await.unwrap_or_default();
    let nhs = params
        .nhs_number
        .as_deref()
        .map(normalise_nhs_number)
        .filter(|s| !s.is_empty());
    let q = params.q.unwrap_or_default();
    let q_lc = q.trim().to_lowercase();

    let items: Vec<_> = volumes
        .iter()
        .filter(|v| match &nhs {
            Some(n) => normalise_nhs_number(&v.nhs_number_snapshot) == *n,
            None => true,
        })
        .filter(|v| {
            q_lc.is_empty()
                || v.title.to_lowercase().contains(&q_lc)
                || v.patient_name_snapshot.to_lowercase().contains(&q_lc)
        })
        .map(|v| {
            let count = folders.iter().filter(|f| f.volume_id == Some(v.id)).count();
            let label = v
                .cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string());
            responses::volume(v, label, count)
        })
        .collect();
    Json(List::with_query(items, q)).into_response()
}

#[derive(Serialize)]
pub struct VolumeShow {
    #[serde(flatten)]
    pub volume: responses::Volume,
    pub folders: Vec<responses::Folder>,
    pub history: Vec<responses::Move>,
}

/// Build the full view of a volume: its member folders and the merged
/// move history of those folders.
async fn volume_show(
    things: &dyn MainThingServiceClient,
    events: &dyn MainEventServiceClient,
    volume: &Volume,
) -> VolumeShow {
    let members: Vec<_> = things
        .search("")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|f| f.volume_id == Some(volume.id))
        .collect();
    let member_ids: HashSet<Uuid> = members.iter().map(|f| f.id).collect();

    let history_all = events
        .list_for_patient(volume.patient_id)
        .await
        .unwrap_or_default();
    let latest = latest_move_per_folder(&history_all);

    let folder_items: Vec<_> = members
        .iter()
        .map(|f| {
            let label = f
                .cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string());
            responses::folder(f, label, latest.get(&f.id).copied())
        })
        .collect();
    let history: Vec<_> = history_all
        .iter()
        .filter(|m| member_ids.contains(&m.folder_id))
        .map(responses::move_event)
        .collect();

    let label = volume
        .cabinet_path_snapshot
        .clone()
        .unwrap_or_else(|| "In transit".to_string());
    VolumeShow {
        volume: responses::volume(volume, label, members.len()),
        folders: folder_items,
        history,
    }
}

#[debug_handler]
pub async fn show(
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let volume = match things.find_volume_by_id(id).await {
        Ok(Some(v)) => v,
        Ok(None) => return responses::not_found("Volume not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    };
    Json(volume_show(things.as_ref(), events.as_ref(), &volume).await).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateVolumeInput {
    pub nhs_number: String,
    pub title: String,
    pub cabinet_id: Option<String>,
}

#[debug_handler]
pub async fn create(
    Extension(patients): Extension<Arc<dyn MainPatientServiceClient>>,
    Extension(places): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Json(input): Json<CreateVolumeInput>,
) -> Response {
    let mut errors: HashMap<String, String> = HashMap::new();
    if input.title.trim().is_empty() {
        errors.insert("title".into(), "Volume title is required.".into());
    }
    if !errors.is_empty() {
        return responses::unprocessable(errors);
    }

    let patient = match patients.find_by_nhs_number(&input.nhs_number).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            errors.insert(
                "nhs_number".into(),
                "No patient with that NHS Number — register a folder for them first.".into(),
            );
            return responses::unprocessable(errors);
        }
        Err(e) => {
            return responses::service_unavailable(format!("Main Patient Service unreachable: {e}"))
        }
    };

    let cabinet_id = input
        .cabinet_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());
    let cabinet_path_snapshot = match cabinet_id {
        Some(id) => match label_path(places.as_ref(), id).await {
            Ok(p) if !p.is_empty() => Some(p),
            _ => None,
        },
        None => None,
    };

    let volume = match things
        .create_volume(NewVolume {
            patient_id: patient.id,
            nhs_number_snapshot: format_nhs_number(&patient.nhs_number),
            patient_name_snapshot: patient.name.clone(),
            title: input.title.trim().to_string(),
            cabinet_id,
            cabinet_path_snapshot: cabinet_path_snapshot.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            errors.insert("title".into(), e.to_string());
            return responses::unprocessable(errors);
        }
    };

    let label = cabinet_path_snapshot.unwrap_or_else(|| "In transit".to_string());
    let body = responses::volume(&volume, label, 0);
    let location = format!("/api/volumes/{}", volume.id);
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct RenameInput {
    pub title: String,
}

#[debug_handler]
pub async fn rename(
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
    Json(input): Json<RenameInput>,
) -> Response {
    if input.title.trim().is_empty() {
        let mut errors = HashMap::new();
        errors.insert("title".into(), "Volume title is required.".into());
        return responses::unprocessable(errors);
    }
    let volume = match things
        .rename_volume(id, input.title.trim().to_string())
        .await
    {
        Ok(v) => v,
        Err(_) => return responses::not_found("Volume not found"),
    };
    Json(volume_show(things.as_ref(), events.as_ref(), &volume).await).into_response()
}

#[derive(Debug, Deserialize)]
pub struct AddFolderInput {
    pub folder_id: String,
}

#[debug_handler]
pub async fn add_folder(
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
    Json(input): Json<AddFolderInput>,
) -> Response {
    let volume = match things.find_volume_by_id(id).await {
        Ok(Some(v)) => v,
        Ok(None) => return responses::not_found("Volume not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    };
    let folder_id = match Uuid::parse_str(&input.folder_id) {
        Ok(u) => u,
        Err(_) => {
            let mut errors = HashMap::new();
            errors.insert("folder_id".into(), "Provide a valid folder UUID.".into());
            return responses::unprocessable(errors);
        }
    };
    let folder = match things.find_by_id(folder_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return responses::not_found("Folder not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    };
    if folder.patient_id != volume.patient_id {
        let mut errors = HashMap::new();
        errors.insert(
            "folder_id".into(),
            "That folder belongs to a different patient.".into(),
        );
        return responses::unprocessable(errors);
    }
    if let Err(e) = things
        .set_folder_volume(folder_id, Some(volume.id), Some(volume.title.clone()))
        .await
    {
        return responses::service_unavailable(format!("Main Thing Service write failed: {e}"));
    }
    Json(volume_show(things.as_ref(), events.as_ref(), &volume).await).into_response()
}

#[debug_handler]
pub async fn remove_folder(
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path((id, folder_id)): Path<(Uuid, Uuid)>,
) -> Response {
    let volume = match things.find_volume_by_id(id).await {
        Ok(Some(v)) => v,
        Ok(None) => return responses::not_found("Volume not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    };
    match things.find_by_id(folder_id).await {
        Ok(Some(f)) if f.volume_id == Some(id) => {
            things.set_folder_volume(folder_id, None, None).await.ok();
        }
        Ok(Some(_)) => {} // not in this volume — no-op
        Ok(None) => return responses::not_found("Folder not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    }
    Json(volume_show(things.as_ref(), events.as_ref(), &volume).await).into_response()
}

#[derive(Debug, Deserialize)]
pub struct MoveVolumeInput {
    pub to_cabinet_id: Option<String>,
    pub worker_id: Option<String>,
    pub moved_by: Option<String>,
    pub reason: Option<String>,
}

#[debug_handler]
pub async fn move_volume(
    Extension(workers): Extension<Arc<dyn MainWorkerServiceClient>>,
    Extension(places): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
    Json(input): Json<MoveVolumeInput>,
) -> Response {
    let volume = match things.find_volume_by_id(id).await {
        Ok(Some(v)) => v,
        Ok(None) => return responses::not_found("Volume not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"))
        }
    };

    let to_cabinet_id = match input.to_cabinet_id.as_deref() {
        None | Some("") => None,
        Some(s) => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => {
                let mut errors = HashMap::new();
                errors.insert(
                    "to_cabinet_id".into(),
                    "Provide a valid cabinet UUID, or null/omit to mark in transit.".into(),
                );
                return responses::unprocessable(errors);
            }
        },
    };
    let to_cabinet_label = match to_cabinet_id {
        Some(c) => match label_path(places.as_ref(), c).await {
            Ok(p) if !p.is_empty() => Some(p),
            _ => None,
        },
        None => None,
    };

    let typed = input.moved_by.as_deref().unwrap_or("").trim().to_string();
    let worker_id = input
        .worker_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());
    let (final_worker_id, final_name, worker_role) = match worker_id {
        Some(wid) => match workers.find_by_id(wid).await {
            Ok(Some(w)) => (Some(w.id), w.name, w.role),
            _ => (None, fallback_name(&typed), None),
        },
        None => (None, fallback_name(&typed), None),
    };

    let members: Vec<_> = things
        .list_for_patient(volume.patient_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|f| f.volume_id == Some(volume.id))
        .collect();

    let base_reason = input.reason.clone().filter(|s| !s.trim().is_empty());
    for f in &members {
        let reason = base_reason
            .clone()
            .unwrap_or_else(|| format!("Moved with volume “{}”", volume.title));
        events
            .record(RecordMove {
                folder_id: f.id,
                patient_id: f.patient_id,
                nhs_number: f.nhs_number_snapshot.clone(),
                patient_name: f.patient_name_snapshot.clone(),
                folder_title: f.title.clone(),
                from_cabinet_id: f.cabinet_id,
                to_cabinet_id,
                from_cabinet_label: f
                    .cabinet_path_snapshot
                    .clone()
                    .unwrap_or_else(|| "In transit".to_string()),
                to_cabinet_label: to_cabinet_label
                    .clone()
                    .unwrap_or_else(|| "In transit".to_string()),
                worker_id: final_worker_id,
                moved_by: final_name.clone(),
                worker_role_snapshot: worker_role.clone(),
                reason: Some(reason),
            })
            .await
            .ok();
        things
            .update_cabinet(f.id, to_cabinet_id, to_cabinet_label.clone())
            .await
            .ok();
    }

    let updated = things
        .update_volume_cabinet(volume.id, to_cabinet_id, to_cabinet_label.clone())
        .await
        .unwrap_or(volume);

    Json(volume_show(things.as_ref(), events.as_ref(), &updated).await).into_response()
}

fn fallback_name(typed: &str) -> String {
    if typed.is_empty() {
        "Unknown porter".into()
    } else {
        typed.to_string()
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/volumes")
        .add("/", get(index))
        .add("/", post(create))
        .add("/{id}", get(show))
        .add("/{id}", axum::routing::patch(rename))
        .add("/{id}/folders", post(add_folder))
        .add(
            "/{id}/folders/{folder_id}",
            axum::routing::delete(remove_folder),
        )
        .add("/{id}/move", post(move_volume))
}
