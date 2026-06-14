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
    main_event_service::{Client as MainEventServiceClient, RecordMove},
    main_place_service::{Client as MainPlaceServiceClient, label_path},
    main_thing_service::Client as MainThingServiceClient,
    main_worker_service::Client as MainWorkerServiceClient,
    responses::{self, List},
};

/// Query parameters accepted by `GET /api/moves`.
#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    /// Free-text filter applied client-side across patient name, folder
    /// title, NHS Number, cabinet labels, mover, and worker role. Empty
    /// or absent returns the whole log.
    pub q: Option<String>,
}

/// `GET /api/moves` — list every recorded move event (global audit log).
///
/// Fetches all move events from the Main Event Service and applies an
/// optional case-insensitive free-text filter (`q`) across the patient
/// name, folder title, NHS Number (spaces ignored on both sides),
/// cabinet labels, mover name, and worker role.
///
/// Request: query param `q` — see `FilterParams`.
/// Response: `200 OK` with a `List` of move-event views.
/// Errors: `503 Service Unavailable` if the Main Event Service is
/// unreachable.
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

/// Request body for `POST /api/moves`.
#[derive(Debug, Deserialize)]
pub struct CreateMoveInput {
    /// UUID of the folder being moved. Must parse as a UUID and resolve
    /// to a known folder.
    pub folder_id: String,
    /// Destination cabinet UUID. Pass `null` or omit to mark "in transit".
    pub to_cabinet_id: Option<String>,
    /// Optional Main Worker Service UUID. Takes precedence over `moved_by`.
    pub worker_id: Option<String>,
    /// Free-text fallback when no `worker_id` is supplied.
    pub moved_by: Option<String>,
    /// Optional free-text reason for the move.
    pub reason: Option<String>,
}

/// `POST /api/moves` — record a folder move.
///
/// Two steps. Step 1 records a `MoveEvent` on the Main Event Service
/// (the durable audit log) — if this fails the request fails. Step 2 is
/// a best-effort PATCH of the folder's `cabinet_id` +
/// `cabinet_path_snapshot` on the Main Thing Service; a failure there is
/// logged but the move is still considered recorded (next sync
/// reconciles).
///
/// Mover attribution: a valid `worker_id` resolves the worker's name +
/// role from the Main Worker Service and takes precedence; otherwise the
/// free-text `moved_by` is used, falling back to `"Unknown porter"`.
/// A `null`/omitted `to_cabinet_id` marks the folder "in transit".
///
/// Request: JSON `CreateMoveInput`.
/// Response: `201 Created` with the move-event view and a `Location:
/// /api/moves/{id}` header.
/// Errors: `422 Unprocessable Entity` for an invalid folder or cabinet
/// UUID; `404 Not Found` if the folder does not exist; `503 Service
/// Unavailable` if the Main Thing Service is unreachable or the Main
/// Event Service write fails.
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
    // `null`/empty destination means "in transit"; otherwise parse it.
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

    // Mover attribution: a resolvable `worker_id` wins (name + role
    // snapshotted from the Worker Service); otherwise fall back to the
    // typed `moved_by` name with no worker id/role.
    let typed_name = input.moved_by.as_deref().unwrap_or("").trim().to_string();
    let (final_worker_id, final_name, worker_role) = match worker_id {
        Some(id) => match workers.find_by_id(id).await {
            Ok(Some(w)) => (Some(w.id), w.name, w.role),
            Ok(None) | Err(_) => (None, fallback_name(&typed_name), None),
        },
        None => (None, fallback_name(&typed_name), None),
    };

    // Step 1: record the audit-log event. A failure here aborts the move.
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

    // Step 2: best-effort cabinet update on the Thing Service. A failure
    // is only logged — the move is already in the audit log.
    if let Err(e) = things
        .update_cabinet(folder.id, to_cabinet_id, to_cabinet_label.clone())
        .await
    {
        tracing::warn!(?e, "Failed to update folder cabinet on Main Thing Service");
    }

    let body = responses::move_event(&event);
    // `201 Created` + `Location` header pointing at the new move event.
    let location = format!("/api/moves/{}", event.id);
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

/// `GET /api/moves/{id}` — show a single move event.
///
/// The Main Event Service exposes no per-id lookup, so this fetches the
/// full log and scans for the matching id.
///
/// Request: path param `id` (move-event UUID).
/// Response: `200 OK` with the move-event view.
/// Errors: `404 Not Found` if no event has that id; `503 Service
/// Unavailable` if the Main Event Service is unreachable.
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

/// Resolve the mover's display name from free text.
///
/// Returns the trimmed `typed` name, or the `"Unknown porter"` sentinel
/// when it is empty.
fn fallback_name(typed: &str) -> String {
    if typed.is_empty() {
        "Unknown porter".into()
    } else {
        typed.to_string()
    }
}

/// Route table for the moves controller, mounted under `/api/moves`.
///
/// `GET /` (index), `POST /` (create), `GET /{id}` (show).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/moves")
        .add("/", get(index))
        .add("/", post(create))
        .add("/{id}", get(show))
}
