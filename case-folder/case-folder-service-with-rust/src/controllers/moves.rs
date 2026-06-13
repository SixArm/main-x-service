//! Moves API.
//!
//! `GET /api/moves` lists every recorded move event (the global audit
//! log) with an optional free-text filter. `POST /api/moves` records a
//! new move:
//!
//! 1. Posts a `MoveEvent` to the Main Event Service (audit log).
//! 2. PATCHes the Main Thing Service to update the folder's
//!    `cabinet_id` + `cabinet_path_snapshot`.
//!
//! Step 2 is best-effort — a failure there is logged as a warning but
//! the move is still recorded in the audit trail, and the next sync
//! reconciles.

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
    main_event_service::{Client as MainEventServiceClient, RecordMove},
    main_place_service::{label_path, Client as MainPlaceServiceClient},
    main_thing_service::Client as MainThingServiceClient,
    main_worker_service::Client as MainWorkerServiceClient,
    responses::{self, List},
};

#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    pub q: Option<String>,
}

#[debug_handler]
pub async fn index(
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Query(params): Query<FilterParams>,
) -> Response {
    let q = params.q.unwrap_or_default();
    let q_lc = q.trim().to_lowercase();

    let moves = match events.list_all().await {
        Ok(ms) => ms,
        Err(e) => {
            tracing::warn!(?e, "Main Event Service unreachable");
            return responses::service_unavailable(format!("Main Event Service unreachable: {e}"));
        }
    };
    let items: Vec<_> = moves
        .iter()
        .filter(|m| {
            if q_lc.is_empty() {
                return true;
            }
            m.patient_name.to_lowercase().contains(&q_lc)
                || m.folder_title.to_lowercase().contains(&q_lc)
                || m.nhs_number
                    .replace(' ', "")
                    .contains(&q_lc.replace(' ', ""))
                || m.from_cabinet_label.to_lowercase().contains(&q_lc)
                || m.to_cabinet_label.to_lowercase().contains(&q_lc)
                || m.moved_by.to_lowercase().contains(&q_lc)
                || m.worker_role_snapshot
                    .as_deref()
                    .map(|r| r.to_lowercase().contains(&q_lc))
                    .unwrap_or(false)
        })
        .map(responses::move_event)
        .collect();

    Json(List::with_query(items, q)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateMoveInput {
    pub folder_id: String,
    /// Destination cabinet UUID. Pass `null` or omit to mark "in transit".
    pub to_cabinet_id: Option<String>,
    /// Optional Main Worker Service UUID. Takes precedence over `moved_by`.
    pub worker_id: Option<String>,
    /// Free-text fallback when no `worker_id` is supplied.
    pub moved_by: Option<String>,
    pub reason: Option<String>,
}

#[debug_handler]
pub async fn create(
    Extension(workers): Extension<Arc<dyn MainWorkerServiceClient>>,
    Extension(places): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Json(input): Json<CreateMoveInput>,
) -> Response {
    let mut errors: HashMap<String, String> = HashMap::new();
    let folder_id = match Uuid::parse_str(&input.folder_id) {
        Ok(id) => id,
        Err(_) => {
            errors.insert("folder_id".into(), "Provide a valid folder UUID.".into());
            return responses::unprocessable(errors);
        }
    };
    let to_cabinet_id = match input.to_cabinet_id.as_deref() {
        None | Some("") => None,
        Some(s) => match Uuid::parse_str(s) {
            Ok(u) => Some(u),
            Err(_) => {
                errors.insert(
                    "to_cabinet_id".into(),
                    "Provide a valid cabinet UUID, or null/omit to mark in transit.".into(),
                );
                return responses::unprocessable(errors);
            }
        },
    };
    let worker_id = input
        .worker_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());

    let folder = match things.find_by_id(folder_id).await {
        Ok(Some(f)) => f,
        Ok(None) => return responses::not_found("Folder not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Thing Service unreachable: {e}"));
        }
    };

    let to_cabinet_label = match to_cabinet_id {
        Some(id) => match label_path(places.as_ref(), id).await {
            Ok(p) if !p.is_empty() => Some(p),
            _ => None,
        },
        None => None,
    };

    let typed_name = input.moved_by.as_deref().unwrap_or("").trim().to_string();
    let (final_worker_id, final_name, worker_role) = match worker_id {
        Some(id) => match workers.find_by_id(id).await {
            Ok(Some(w)) => (Some(w.id), w.name, w.role),
            Ok(None) | Err(_) => (None, fallback_name(&typed_name), None),
        },
        None => (None, fallback_name(&typed_name), None),
    };

    let event = match events
        .record(RecordMove {
            folder_id: folder.id,
            patient_id: folder.patient_id,
            nhs_number: folder.nhs_number_snapshot.clone(),
            patient_name: folder.patient_name_snapshot.clone(),
            folder_title: folder.title.clone(),
            from_cabinet_id: folder.cabinet_id,
            to_cabinet_id,
            from_cabinet_label: folder
                .cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string()),
            to_cabinet_label: to_cabinet_label
                .clone()
                .unwrap_or_else(|| "In transit".to_string()),
            worker_id: final_worker_id,
            moved_by: final_name,
            worker_role_snapshot: worker_role,
            reason: input.reason.clone().filter(|s| !s.trim().is_empty()),
        })
        .await
    {
        Ok(ev) => ev,
        Err(e) => {
            return responses::service_unavailable(format!("Main Event Service write failed: {e}"));
        }
    };

    if let Err(e) = things
        .update_cabinet(folder.id, to_cabinet_id, to_cabinet_label.clone())
        .await
    {
        tracing::warn!(?e, "Failed to update folder cabinet on Main Thing Service");
    }

    let body = responses::move_event(&event);
    let location = format!("/api/moves/{}", event.id);
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

#[debug_handler]
pub async fn show(
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let moves = match events.list_all().await {
        Ok(ms) => ms,
        Err(e) => {
            tracing::warn!(?e, "Main Event Service unreachable");
            return responses::service_unavailable(format!("Main Event Service unreachable: {e}"));
        }
    };
    match moves.iter().find(|m| m.id == id) {
        Some(m) => Json(responses::move_event(m)).into_response(),
        None => responses::not_found("Move event not found"),
    }
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
        .prefix("/api/moves")
        .add("/", get(index))
        .add("/", post(create))
        .add("/{id}", get(show))
}
