//! Patients API — read-only aggregator.
//!
//! Patient records live in the **Main Patient Service**; folders live
//! in the **Main Thing Service**; per-patient move history lives in
//! the **Main Event Service**. This controller proxies the three.

use axum::{
    Extension, Json, debug_handler,
    extract::{Path, Query},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    controllers::stats::latest_move_per_folder,
    main_event_service::Client as MainEventServiceClient,
    main_patient_service::{Client as MainPatientServiceClient, Patient},
    main_thing_service::Client as MainThingServiceClient,
    nhs::format_nhs_number,
    responses::{self, List},
};

/// Query parameters accepted by `GET /api/patients`.
#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    /// When empty, the listing is derived from folder snapshots; when
    /// present, it is treated as an NHS Number lookup against the Main
    /// Patient Service.
    pub q: Option<String>,
}

/// `GET /api/patients` — list / search patients.
///
/// With no query, derives a distinct patient list from the Main Thing
/// Service's folder snapshots (cheap, no Patient Service round-trip). A
/// non-empty `q` is treated as an NHS Number and looked up against the
/// Main Patient Service. Each entry carries that patient's folder count.
///
/// Request: query param `q` — see `FilterParams`.
/// Response: `200 OK` with a `List` of patient views.
/// Errors: `503 Service Unavailable` if the Main Patient Service lookup
/// fails for a non-empty query. The folder fetch is best-effort.
#[debug_handler]
pub async fn index(
    Extension(patients): Extension<Arc<dyn MainPatientServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Query(params): Query<FilterParams>,
) -> Response {
    let q = params.q.unwrap_or_default();
    let folders = things.search("").await.unwrap_or_default();

    let mut folder_counts: std::collections::HashMap<Uuid, usize> =
        std::collections::HashMap::new();
    for f in &folders {
        *folder_counts.entry(f.patient_id).or_insert(0) += 1;
    }

    let patients_list: Vec<Patient> = if q.trim().is_empty() {
        let mut seen = std::collections::HashSet::new();
        folders
            .iter()
            .filter(|f| seen.insert(f.patient_id))
            .map(|f| Patient {
                id: f.patient_id,
                nhs_number: f.nhs_number_snapshot.clone(),
                name: f.patient_name_snapshot.clone(),
                date_of_birth: None,
            })
            .collect()
    } else {
        match patients.find_by_nhs_number(q.trim()).await {
            Ok(Some(p)) => vec![p],
            Ok(None) => vec![],
            Err(e) => {
                tracing::warn!(?e, "Main Patient Service search failed");
                return responses::service_unavailable(format!(
                    "Main Patient Service unreachable: {e}"
                ));
            }
        }
    };

    let items: Vec<_> = patients_list
        .iter()
        .map(|p| responses::patient(p, *folder_counts.get(&p.id).unwrap_or(&0)))
        .collect();
    Json(List::with_query(items, q)).into_response()
}

/// Response body for `GET /api/patients/{nhs}`.
#[derive(Serialize)]
pub struct PatientShow {
    /// The patient view, or `None` when the Main Patient Service had no
    /// record and we fell back to folder snapshots.
    pub patient: Option<responses::Patient>,
    /// The patient's folders with current-location labels.
    pub folders: Vec<responses::Folder>,
    /// Merged move history across the patient's folders (empty in
    /// fallback mode, since history is keyed by patient id).
    pub history: Vec<responses::Move>,
    /// The NHS Number, formatted for display.
    pub nhs_number: String,
    /// `true` when the Main Patient Service had no record for this NHS
    /// Number and we fell back to the Main Thing Service's folder
    /// snapshots. Clients can surface this however they like.
    pub patient_service_match: bool,
}

/// `GET /api/patients/{nhs}` — show one patient by NHS Number.
///
/// Looks the patient up in the Main Patient Service. On a hit, attaches
/// the patient's folders (from the Main Thing Service) and merged move
/// history (from the Main Event Service). On a miss — or if the Patient
/// Service is unreachable — falls back to the folder snapshots filed
/// under that NHS Number and sets `patient_service_match = false`.
///
/// Request: path param `nhs` (NHS Number).
/// Response: `200 OK` with `PatientShow`. Always `200`: an unreachable
/// Patient Service degrades to snapshot fallback rather than erroring.
#[debug_handler]
pub async fn show(
    Extension(patients): Extension<Arc<dyn MainPatientServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(nhs): Path<String>,
) -> Response {
    let nhs_display = format_nhs_number(&nhs);
    let patient = match patients.find_by_nhs_number(&nhs).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                ?e,
                "Main Patient Service unreachable; falling back to snapshots"
            );
            None
        }
    };

    let body = match &patient {
        // Patient Service hit: enrich with folders + merged history.
        Some(p) => {
            let folders = things.list_for_patient(p.id).await.unwrap_or_default();
            let history = events.list_for_patient(p.id).await.unwrap_or_default();
            let latest = latest_move_per_folder(&history);
            let folder_items: Vec<_> = folders
                .iter()
                .map(|f| {
                    let label = f
                        .cabinet_path_snapshot
                        .clone()
                        .unwrap_or_else(|| "In transit".to_string());
                    responses::folder(f, label, latest.get(&f.id).copied())
                })
                .collect();
            let move_items: Vec<_> = history.iter().map(responses::move_event).collect();
            PatientShow {
                patient: Some(responses::patient(p, folders.len())),
                folders: folder_items,
                history: move_items,
                nhs_number: nhs_display,
                patient_service_match: true,
            }
        }
        // Fallback: no Patient Service record, use folder snapshots.
        None => {
            let folders = things.list_for_nhs_number(&nhs).await.unwrap_or_default();
            let folder_items: Vec<_> = folders
                .iter()
                .map(|f| {
                    let label = f
                        .cabinet_path_snapshot
                        .clone()
                        .unwrap_or_else(|| "In transit".to_string());
                    responses::folder(f, label, None)
                })
                .collect();
            PatientShow {
                patient: None,
                folders: folder_items,
                history: vec![],
                nhs_number: nhs_display,
                patient_service_match: false,
            }
        }
    };

    Json(body).into_response()
}

/// Route table for the patients controller, mounted under
/// `/api/patients`.
///
/// `GET /` (index), `GET /{nhs}` (show).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/patients")
        .add("/", get(index))
        .add("/{nhs}", get(show))
}
