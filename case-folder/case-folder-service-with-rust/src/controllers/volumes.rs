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

/// Query parameters accepted by `GET /api/volumes`.
#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    /// Free-text filter matched against volume title and patient name.
    pub q: Option<String>,
    /// NHS Number filter, normalised before comparison.
    pub nhs_number: Option<String>,
}

/// `GET /api/volumes` — list / search volumes.
///
/// Fetches all volumes from the Main Thing Service and filters them
/// client-side by normalised NHS Number and/or a case-insensitive
/// free-text term over title and patient name. Member folder counts come
/// from the folder set.
///
/// Request: query params `q`, `nhs_number` — see `FilterParams`.
/// Response: `200 OK` with a `List` of volume views.
/// Errors: `503 Service Unavailable` if the Main Thing Service is
/// unreachable. The folder fetch is best-effort.
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

/// Full view of a volume returned by the show / mutation handlers.
#[derive(Serialize)]
pub struct VolumeShow {
    /// The volume view, flattened into the top-level JSON object.
    #[serde(flatten)]
    pub volume: responses::Volume,
    /// Member folders with current-location labels.
    pub folders: Vec<responses::Folder>,
    /// Merged move history of the member folders, newest first.
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

/// `GET /api/volumes/{id}` — show one volume with its folders and history.
///
/// Request: path param `id` (volume UUID).
/// Response: `200 OK` with `VolumeShow`.
/// Errors: `404 Not Found` if no such volume; `503 Service Unavailable`
/// if the Main Thing Service is unreachable.
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

/// Request body for `POST /api/volumes`.
#[derive(Debug, Deserialize)]
pub struct CreateVolumeInput {
    /// NHS Number of the owning patient, who must already exist in the
    /// Main Patient Service.
    pub nhs_number: String,
    /// Volume title. Required.
    pub title: String,
    /// Optional starting cabinet UUID; empty / unparseable means none.
    pub cabinet_id: Option<String>,
}

/// `POST /api/volumes` — create an empty volume for an existing patient.
///
/// Validates the title, requires the patient to already exist (register
/// a folder for them first), resolves the optional cabinet's path label,
/// and creates the volume `Thing` in the Main Thing Service.
///
/// Request: JSON `CreateVolumeInput`.
/// Response: `201 Created` with the volume view and a `Location:
/// /api/volumes/{id}` header.
/// Errors: `422 Unprocessable Entity` for a missing title, an unknown
/// patient, or a Thing Service write error; `503 Service Unavailable`
/// if the Main Patient Service is unreachable.
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

/// Request body for `PATCH /api/volumes/{id}` (rename).
#[derive(Debug, Deserialize)]
pub struct RenameInput {
    /// New volume title. Required.
    pub title: String,
}

/// `PATCH /api/volumes/{id}` — rename a volume.
///
/// Request: path param `id` (volume UUID) + JSON `RenameInput`.
/// Response: `200 OK` with the refreshed `VolumeShow`.
/// Errors: `422 Unprocessable Entity` for an empty title; `404 Not
/// Found` if the volume does not exist.
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

/// Request body for `POST /api/volumes/{id}/folders` (add folder).
#[derive(Debug, Deserialize)]
pub struct AddFolderInput {
    /// UUID of the folder to file into the volume.
    pub folder_id: String,
}

/// `POST /api/volumes/{id}/folders` — add a folder to a volume.
///
/// The folder must belong to the same patient as the volume
/// (same-patient membership guard). Sets the folder's volume pointer +
/// title snapshot on the Main Thing Service.
///
/// Request: path param `id` (volume UUID) + JSON `AddFolderInput`.
/// Response: `200 OK` with the refreshed `VolumeShow`.
/// Errors: `404 Not Found` if the volume or folder is missing; `422
/// Unprocessable Entity` for an invalid folder UUID or a different-patient
/// folder; `503 Service Unavailable` if the Main Thing Service is
/// unreachable or the write fails.
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
    // Same-patient membership guard: a volume bundles one patient's
    // folders only.
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

/// `DELETE /api/volumes/{id}/folders/{folder_id}` — remove a folder from
/// a volume.
///
/// Clears the folder's volume pointer only when it is actually in this
/// volume; a folder belonging to a different volume is a silent no-op.
///
/// Request: path params `id` (volume UUID), `folder_id` (folder UUID).
/// Response: `200 OK` with the refreshed `VolumeShow`.
/// Errors: `404 Not Found` if the volume or folder is missing; `503
/// Service Unavailable` if the Main Thing Service is unreachable.
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
        // In this volume: detach it (best-effort).
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

/// Request body for `POST /api/volumes/{id}/move`.
#[derive(Debug, Deserialize)]
pub struct MoveVolumeInput {
    /// Destination cabinet UUID; `null`/empty marks the bundle in transit.
    pub to_cabinet_id: Option<String>,
    /// Optional Main Worker Service UUID. Takes precedence over `moved_by`.
    pub worker_id: Option<String>,
    /// Free-text fallback mover name when no `worker_id` is supplied.
    pub moved_by: Option<String>,
    /// Optional free-text reason; defaults per folder to "Moved with
    /// volume …" when omitted.
    pub reason: Option<String>,
}

/// `POST /api/volumes/{id}/move` — move a whole volume.
///
/// Fans out to the per-folder move machinery: for every member folder it
/// records a move event (Main Event Service) and updates that folder's
/// cabinet (Main Thing Service), then updates the volume's own cabinet
/// pointer. Mover attribution mirrors the single-folder move: a
/// resolvable `worker_id` wins, else `moved_by`, else `"Unknown porter"`.
/// A `null`/omitted destination marks everything in transit.
///
/// Request: path param `id` (volume UUID) + JSON `MoveVolumeInput`.
/// Response: `200 OK` with the refreshed `VolumeShow`.
/// Errors: `404 Not Found` if the volume is missing; `422 Unprocessable
/// Entity` for an invalid cabinet UUID; `503 Service Unavailable` if the
/// Main Thing Service is unreachable. Per-folder event/cabinet writes are
/// best-effort.
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

    // Record a move event + cabinet update for every member folder. The
    // default per-folder reason references the volume title.
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

    // Finally update the volume's own cabinet pointer; on failure keep
    // the pre-move volume so the response still renders.
    let updated = things
        .update_volume_cabinet(volume.id, to_cabinet_id, to_cabinet_label.clone())
        .await
        .unwrap_or(volume);

    Json(volume_show(things.as_ref(), events.as_ref(), &updated).await).into_response()
}

/// Resolve the mover's display name from free text.
///
/// Returns the `typed` name, or the `"Unknown porter"` sentinel when it
/// is empty.
fn fallback_name(typed: &str) -> String {
    if typed.is_empty() {
        "Unknown porter".into()
    } else {
        typed.to_string()
    }
}

/// Route table for the volumes controller, mounted under `/api/volumes`.
///
/// `GET /` (index), `POST /` (create), `GET /{id}` (show),
/// `PATCH /{id}` (rename), `POST /{id}/folders` (add folder),
/// `DELETE /{id}/folders/{folder_id}` (remove folder),
/// `POST /{id}/move` (move volume).
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
