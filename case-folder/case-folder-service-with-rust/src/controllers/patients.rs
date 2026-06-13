//! Patients API — read-only aggregator.
//!
//! Patient records live in the **Main Patient Service**; folders live
//! in the **Main Thing Service**; per-patient move history lives in
//! the **Main Event Service**. This controller proxies the three.

use axum::{
    debug_handler,
    extract::{Path, Query},
    response::{IntoResponse, Response},
    Extension, Json,
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

#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    pub q: Option<String>,
}

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

#[derive(Serialize)]
pub struct PatientShow {
    pub patient: Option<responses::Patient>,
    pub folders: Vec<responses::Folder>,
    pub history: Vec<responses::Move>,
    pub nhs_number: String,
    /// `true` when the Main Patient Service had no record for this NHS
    /// Number and we fell back to the Main Thing Service's folder
    /// snapshots. Clients can surface this however they like.
    pub patient_service_match: bool,
}

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

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/patients")
        .add("/", get(index))
        .add("/{nhs}", get(show))
}
