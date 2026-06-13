//! Workers API — read-only proxy to the Main Worker Service.
//!
//! `GET /api/workers` lists/searches workers. `GET /api/workers/{id}`
//! is a derived view (see root spec D-10): from the move-event log it
//! shows the folders the worker has moved and every folder belonging to
//! a patient that worker has handled. The tracker stores nothing about
//! workers itself.

use axum::{
    debug_handler,
    extract::{Path, Query},
    response::{IntoResponse, Response},
    Extension, Json,
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    controllers::stats::latest_move_per_folder,
    main_event_service::Client as MainEventServiceClient,
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
    Extension(mws): Extension<Arc<dyn MainWorkerServiceClient>>,
    Query(params): Query<FilterParams>,
) -> Response {
    let q = params.q.unwrap_or_default();
    let workers = match mws.search(q.trim()).await {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(?e, "Main Worker Service search failed");
            return responses::service_unavailable(format!("Main Worker Service unreachable: {e}"));
        }
    };
    let items: Vec<_> = workers.iter().map(responses::worker).collect();
    Json(List::with_query(items, q)).into_response()
}

/// Derived worker detail. `moved_folders` are the distinct folders this
/// worker has moved; `patient_folders` is every folder of any patient
/// the worker has handled (a superset); `moves` is their move log.
#[derive(Serialize)]
pub struct WorkerShow {
    pub worker: responses::Worker,
    pub moved_folders: Vec<responses::Folder>,
    pub patient_folders: Vec<responses::Folder>,
    pub moves: Vec<responses::Move>,
}

#[debug_handler]
pub async fn show(
    Extension(mws): Extension<Arc<dyn MainWorkerServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let worker = match mws.find_by_id(id).await {
        Ok(Some(w)) => w,
        Ok(None) => return responses::not_found("Worker not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Worker Service unreachable: {e}"))
        }
    };

    let all_events = events.list_all().await.unwrap_or_default();
    let worker_moves: Vec<&crate::main_event_service::MoveEvent> = all_events
        .iter()
        .filter(|m| m.worker_id == Some(id))
        .collect();

    let moved_folder_ids: HashSet<Uuid> = worker_moves.iter().map(|m| m.folder_id).collect();
    let patient_ids: HashSet<Uuid> = worker_moves.iter().map(|m| m.patient_id).collect();

    let all_folders = things.search("").await.unwrap_or_default();
    let latest = latest_move_per_folder(&all_events);
    let project = |f: &crate::main_thing_service::Folder| {
        let label = f
            .cabinet_path_snapshot
            .clone()
            .unwrap_or_else(|| "In transit".to_string());
        responses::folder(f, label, latest.get(&f.id).copied())
    };

    let moved_folders: Vec<_> = all_folders
        .iter()
        .filter(|f| moved_folder_ids.contains(&f.id))
        .map(&project)
        .collect();
    let patient_folders: Vec<_> = all_folders
        .iter()
        .filter(|f| patient_ids.contains(&f.patient_id))
        .map(&project)
        .collect();
    let moves: Vec<_> = worker_moves
        .iter()
        .copied()
        .map(responses::move_event)
        .collect();

    Json(WorkerShow {
        worker: responses::worker(&worker),
        moved_folders,
        patient_folders,
        moves,
    })
    .into_response()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/workers")
        .add("/", get(index))
        .add("/{id}", get(show))
}
