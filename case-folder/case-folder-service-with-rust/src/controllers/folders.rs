//! Folder CRUD — pure proxy to the Main Thing Service.
//!
//! Each folder is a `Thing` with `thing_type = Other("CaseFile")` in
//! the upstream service. This crate keeps no folders table; the only
//! side effect on create is a synthetic "Folder created" move event
//! posted to the Main Event Service so the audit trail starts where
//! the folder first landed.

use axum::{
    Extension, Json, debug_handler,
    extract::{Path, Query},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
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
    main_place_service::{Client as MainPlaceServiceClient, label_path},
    main_thing_service::{Client as MainThingServiceClient, NewFolder},
    nhs::{format_nhs_number, is_valid_nhs_number},
    responses::{self, List},
};

/// Query parameters accepted by `GET /api/folders`.
///
/// When `nhs_number` is present and non-empty it takes precedence and
/// the listing is scoped to that patient's folders; otherwise `q` is a
/// free-text search passed through to the Main Thing Service.
#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    /// Free-text search term forwarded to the Main Thing Service. Empty
    /// or absent means "list everything".
    pub q: Option<String>,
    /// NHS Number to scope the listing to a single patient. Takes
    /// precedence over `q` when present and non-empty.
    pub nhs_number: Option<String>,
}

/// `GET /api/folders` — list / search case-note folders.
///
/// Pure proxy: folders live in the Main Thing Service, never here. If
/// `nhs_number` is supplied the listing is scoped to that patient,
/// otherwise `q` is a free-text search. Each folder's current location
/// label is taken from its `cabinet_path_snapshot`, falling back to the
/// `"In transit"` sentinel when it has no cabinet. The latest move event
/// per folder is attached from the Main Event Service.
///
/// Request: query params (`q`, `nhs_number`) — see `FilterParams`.
/// Response: `200 OK` with a `List` of folder views.
/// Errors: `503 Service Unavailable` if the Main Thing Service is
/// unreachable. Event-history failure is non-fatal (falls back to an
/// empty history).
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

    // Event-history fetch is best-effort: an unreachable Event Service
    // just means no "latest move" badge, not a failed listing.
    let history = events.list_all().await.unwrap_or_default();
    let latest_by_folder = latest_move_per_folder(&history);

    let items: Vec<_> = folders
        .iter()
        .map(|f| {
            // Snapshot label is the folder's last known cabinet path;
            // absence means the folder is between cabinets.
            let label = f
                .cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string());
            responses::folder(f, label, latest_by_folder.get(&f.id).copied())
        })
        .collect();

    Json(List::with_query(items, q)).into_response()
}

/// Request body for `POST /api/folders`.
#[derive(Debug, Deserialize)]
pub struct CreateFolderInput {
    /// Patient NHS Number. Validated with the Modulus-11 check; an
    /// invalid value yields a `422` before any upstream call.
    pub nhs_number: String,
    /// Patient name. Required only when the patient is not yet known to
    /// the Main Patient Service and must be registered.
    pub patient_name: Option<String>,
    /// Patient date of birth (`YYYY-MM-DD`). Required only when
    /// registering a new patient; parsed leniently (bad dates are
    /// treated as absent and surfaced as a `422` if registration needs
    /// one).
    pub date_of_birth: Option<String>,
    /// Human-readable folder title (e.g. "Volume 1"). Required.
    pub title: String,
    /// Optional cabinet UUID the folder starts in. Empty / unparseable
    /// is treated as "no cabinet" (in transit).
    pub cabinet_id: Option<String>,
    /// Optional free-text notes.
    pub notes: Option<String>,
    /// Optional volume to file this folder into on creation.
    pub volume_id: Option<String>,
}

/// `POST /api/folders` — register a new case-note folder.
///
/// Flow: validate the NHS Number (Modulus-11) and title; find-or-create
/// the patient in the Main Patient Service (a new patient needs name +
/// date of birth); resolve the optional cabinet's path label; optionally
/// attach to a same-patient volume; create the folder `Thing` in the
/// Main Thing Service; then post a synthetic "Folder created" move event
/// to the Main Event Service so the audit trail starts where the folder
/// first landed.
///
/// Request: JSON `CreateFolderInput`.
/// Response: `201 Created` with the folder view and a `Location:
/// /api/folders/{id}` header.
/// Errors: `422 Unprocessable Entity` for invalid NHS Number, missing
/// title, missing new-patient name/date-of-birth, or a volume that
/// belongs to a different patient; `503 Service Unavailable` when the
/// Main Patient Service is unreachable.
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
    // Parse the date of birth leniently; a bad value becomes `None` and
    // is only fatal if the patient turns out to need registration.
    let dob = input
        .date_of_birth
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    // Empty or unparseable cabinet id means "no cabinet" (in transit).
    let cabinet_id = input
        .cabinet_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());

    // Bail out early on the cheap field-level validation errors.
    if !errors.is_empty() {
        return responses::unprocessable(errors);
    }

    // Find-or-create the patient. An existing record is used as-is; a
    // missing one is registered, which requires name + date of birth.
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
                    // Safe: the `dob.is_none()` guard above already
                    // returned `422` when the date was missing.
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

    // Resolve a human-readable "Building — Room — Cabinet" label for the
    // starting cabinet. Any failure or empty path leaves it unset.
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
            // Same patient: attach and snapshot the volume title.
            Ok(Some(v)) if v.patient_id == patient.id => (Some(v.id), Some(v.title)),
            // Different patient: reject with `422`.
            Ok(Some(_)) => {
                errors.insert(
                    "volume_id".into(),
                    "That volume belongs to a different patient.".into(),
                );
                return responses::unprocessable(errors);
            }
            // Missing volume or upstream error: silently file unbundled.
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

    // Synthetic "Folder created" move event seeds the audit trail. From
    // origin `(new folder)` into the starting cabinet, attributed to the
    // `System` actor. Best-effort: a failure here does not undo the
    // already-created folder.
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
    // `201 Created` + `Location` header pointing at the new resource.
    let location = format!("/api/folders/{}", folder.id);
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

/// `GET /api/folders/{id}` — show a single folder.
///
/// Fetches the folder from the Main Thing Service and attaches its most
/// recent move event from the Main Event Service. The location label is
/// the folder's `cabinet_path_snapshot`, falling back to `"In transit"`.
///
/// Request: path param `id` (folder UUID).
/// Response: `200 OK` with the folder view.
/// Errors: `404 Not Found` if no such folder; `503 Service Unavailable`
/// if the Main Thing Service is unreachable. Event-history failure is
/// non-fatal.
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
    // History is newest-first, so the first entry is the latest move.
    let history = events.list_for_folder(id).await.unwrap_or_default();
    let latest = history.first();
    let label = folder
        .cabinet_path_snapshot
        .clone()
        .unwrap_or_else(|| "In transit".to_string());
    Json(responses::folder(&folder, label, latest)).into_response()
}

/// `GET /api/folders/{id}/history` — full move history of one folder.
///
/// Returns every recorded move event for the folder, newest first,
/// straight from the Main Event Service audit log.
///
/// Request: path param `id` (folder UUID).
/// Response: `200 OK` with a `List` of move-event views.
/// Errors: `503 Service Unavailable` if the Main Event Service is
/// unreachable.
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

/// Log an upstream-service failure and build a `503 Service
/// Unavailable` response.
///
/// `name` is the upstream service's display name and `err` its error;
/// both are recorded via `tracing::warn!` and the name + error are
/// echoed in the response body.
fn upstream_unavailable(name: &str, err: impl std::fmt::Display) -> Response {
    tracing::warn!(error = %err, service = name, "upstream service unavailable");
    responses::service_unavailable(format!("{name} unreachable: {err}"))
}

/// Route table for the folders controller, mounted under `/api/folders`.
///
/// `GET /` (index), `POST /` (create), `GET /{id}` (show), and
/// `GET /{id}/history`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/folders")
        .add("/", get(index))
        .add("/", post(create))
        .add("/{id}", get(show))
        .add("/{id}/history", get(history))
}
