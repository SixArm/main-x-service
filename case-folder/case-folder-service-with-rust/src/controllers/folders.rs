//! Folder CRUD — pure proxy to the Main Thing Service.
//!
//! Each folder is a `Thing` with `thing_type = Other("CaseFile")` in
//! the upstream service. This crate keeps no folders table; the only
//! side effect on create is a synthetic "Folder created" move event
//! posted to the Main Event Service so the audit trail starts where
//! the folder first landed.

use axum::{
    debug_handler,
    extract::{Path, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use loco_rs::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    controllers::stats::latest_move_per_folder,
    main_event_service::{Client as MainEventServiceClient, RecordMove},
    main_patient_service::{self, Client as MainPatientServiceClient, CreatePatient},
    main_place_service::{label_path, Client as MainPlaceServiceClient},
    main_thing_service::{Client as MainThingServiceClient, NewFolder},
    nhs::{format_nhs_number, is_valid_nhs_number},
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
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Query(params): Query<FilterParams>,
) -> Response {
    let q = params.q.unwrap_or_default();

    let folders = if let Some(nhs) = params.nhs_number.as_deref().filter(|s| !s.is_empty()) {
        match things.list_for_nhs_number(nhs).await {
            Ok(fs) => fs,
            Err(e) => return upstream_unavailable("Main Thing Service", e),
        }
    } else {
        match things.search(&q).await {
            Ok(fs) => fs,
            Err(e) => return upstream_unavailable("Main Thing Service", e),
        }
    };

    let history = events.list_all().await.unwrap_or_default();
    let latest_by_folder = latest_move_per_folder(&history);

    let items: Vec<_> = folders
        .iter()
        .map(|f| {
            let label = f
                .cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string());
            responses::folder(f, label, latest_by_folder.get(&f.id).copied())
        })
        .collect();

    Json(List::with_query(items, q)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderInput {
    pub nhs_number: String,
    pub patient_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub title: String,
    pub cabinet_id: Option<String>,
    pub notes: Option<String>,
    /// Optional volume to file this folder into on creation.
    pub volume_id: Option<String>,
}

#[debug_handler]
pub async fn create(
    Extension(patients): Extension<Arc<dyn MainPatientServiceClient>>,
    Extension(places): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Json(input): Json<CreateFolderInput>,
) -> Response {
    let mut errors: HashMap<String, String> = HashMap::new();
    if !is_valid_nhs_number(&input.nhs_number) {
        errors.insert(
            "nhs_number".into(),
            "Enter a valid 10-digit NHS Number (Modulus 11).".into(),
        );
    }
    if input.title.trim().is_empty() {
        errors.insert("title".into(), "Folder title is required.".into());
    }
    let dob = input
        .date_of_birth
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let cabinet_id = input
        .cabinet_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());

    if !errors.is_empty() {
        return responses::unprocessable(errors);
    }

    let patient = match patients.find_by_nhs_number(&input.nhs_number).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let name = input
                .patient_name
                .as_deref()
                .map(str::trim)
                .unwrap_or_default();
            if name.is_empty() {
                errors.insert(
                    "patient_name".into(),
                    "Patient name is required to register a new patient with the Main Patient Service.".into(),
                );
            }
            if dob.is_none() {
                errors.insert(
                    "date_of_birth".into(),
                    "Date of birth is required to register a new patient with the Main Patient Service.".into(),
                );
            }
            if !errors.is_empty() {
                return responses::unprocessable(errors);
            }
            match main_patient_service::find_or_create(
                patients.as_ref(),
                CreatePatient {
                    nhs_number: input.nhs_number.clone(),
                    name: name.to_string(),
                    date_of_birth: dob.unwrap(),
                },
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    return responses::service_unavailable(format!(
                        "Main Patient Service error: {e}"
                    ));
                }
            }
        }
        Err(e) => {
            return responses::service_unavailable(format!(
                "Main Patient Service unreachable: {e}"
            ));
        }
    };

    let cabinet_path_snapshot = match cabinet_id {
        Some(id) => match label_path(places.as_ref(), id).await {
            Ok(p) if !p.is_empty() => Some(p),
            _ => None,
        },
        None => None,
    };

    // Optional volume membership at creation. The volume must belong to
    // the same patient.
    let (volume_id, volume_title_snapshot) = match input
        .volume_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(vid) => match things.find_volume_by_id(vid).await {
            Ok(Some(v)) if v.patient_id == patient.id => (Some(v.id), Some(v.title)),
            Ok(Some(_)) => {
                errors.insert(
                    "volume_id".into(),
                    "That volume belongs to a different patient.".into(),
                );
                return responses::unprocessable(errors);
            }
            _ => (None, None),
        },
        None => (None, None),
    };

    let folder = match things
        .create(NewFolder {
            patient_id: patient.id,
            nhs_number_snapshot: format_nhs_number(&patient.nhs_number),
            patient_name_snapshot: patient.name.clone(),
            title: input.title.trim().to_string(),
            cabinet_id,
            cabinet_path_snapshot: cabinet_path_snapshot.clone(),
            notes: input.notes.clone().filter(|s| !s.trim().is_empty()),
            volume_id,
            volume_title_snapshot,
        })
        .await
    {
        Ok(f) => f,
        Err(e) => {
            errors.insert("title".into(), e.to_string());
            return responses::unprocessable(errors);
        }
    };

    events
        .record(RecordMove {
            folder_id: folder.id,
            patient_id: folder.patient_id,
            nhs_number: folder.nhs_number_snapshot.clone(),
            patient_name: folder.patient_name_snapshot.clone(),
            folder_title: folder.title.clone(),
            from_cabinet_id: None,
            to_cabinet_id: cabinet_id,
            from_cabinet_label: "(new folder)".into(),
            to_cabinet_label: cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string()),
            worker_id: None,
            moved_by: "System".into(),
            worker_role_snapshot: None,
            reason: Some("Folder created".into()),
        })
        .await
        .ok();

    let body = responses::folder(
        &folder,
        cabinet_path_snapshot.unwrap_or_else(|| "In transit".to_string()),
        None,
    );
    let location = format!("/api/folders/{}", folder.id);
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

#[debug_handler]
pub async fn show(
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let folder = match things.find_by_id(id).await {
        Ok(Some(f)) => f,
        Ok(None) => return responses::not_found("Folder not found"),
        Err(e) => return upstream_unavailable("Main Thing Service", e),
    };
    let history = events.list_for_folder(id).await.unwrap_or_default();
    let latest = history.first();
    let label = folder
        .cabinet_path_snapshot
        .clone()
        .unwrap_or_else(|| "In transit".to_string());
    Json(responses::folder(&folder, label, latest)).into_response()
}

#[debug_handler]
pub async fn history(
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let moves = match events.list_for_folder(id).await {
        Ok(ms) => ms,
        Err(e) => return upstream_unavailable("Main Event Service", e),
    };
    let items: Vec<_> = moves.iter().map(responses::move_event).collect();
    Json(List::<responses::Move>::new(items)).into_response()
}

fn upstream_unavailable(name: &str, err: impl std::fmt::Display) -> Response {
    tracing::warn!(error = %err, service = name, "upstream service unavailable");
    responses::service_unavailable(format!("{name} unreachable: {err}"))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/folders")
        .add("/", get(index))
        .add("/", post(create))
        .add("/{id}", get(show))
        .add("/{id}/history", get(history))
}
