//! Aggregate counters across every Main-X-Service.
//!
//! Returns folder / patient / cabinet / move tallies. When any single
//! upstream is unreachable that slice of the response is zero and a
//! warning is logged — the endpoint itself stays 200 because partial
//! data is more useful here than a hard failure.

use axum::{debug_handler, Extension, Json};
use loco_rs::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    main_event_service::Client as MainEventServiceClient,
    main_place_service::{Client as MainPlaceServiceClient, PlaceType},
    main_thing_service::Client as MainThingServiceClient,
};

#[derive(Serialize)]
pub struct Stats {
    pub patients: usize,
    pub folders: Folders,
    pub places: Places,
    pub moves_24h: usize,
}

#[derive(Serialize)]
pub struct Folders {
    pub total: usize,
    pub in_cabinet: usize,
    pub in_transit: usize,
}

#[derive(Serialize)]
pub struct Places {
    pub buildings: usize,
    pub rooms: usize,
    pub cabinets: usize,
}

#[debug_handler]
pub async fn index(
    Extension(places): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
) -> Result<Json<Stats>> {
    let folders = things.search("").await.unwrap_or_else(|e| {
        tracing::warn!(
            ?e,
            "Main Thing Service unreachable; folder counts will be 0"
        );
        vec![]
    });
    let history = events.list_all().await.unwrap_or_else(|e| {
        tracing::warn!(?e, "Main Event Service unreachable; move counts will be 0");
        vec![]
    });

    let (buildings, rooms, cabinets) = match (
        places.search("", Some(PlaceType::HOSPITAL)).await,
        places.search("", Some(PlaceType::RECORDS_ROOM)).await,
        places.search("", Some(PlaceType::FILE_CABINET)).await,
    ) {
        (Ok(b), Ok(r), Ok(c)) => (b, r, c),
        _ => {
            tracing::warn!("Main Place Service unreachable; place counts will be 0");
            (vec![], vec![], vec![])
        }
    };

    let latest_by_folder = latest_move_per_folder(&history);

    let in_cabinet = folders
        .iter()
        .filter(|f| match latest_by_folder.get(&f.id) {
            Some(ev) => ev.to_cabinet_id.is_some(),
            None => f.cabinet_id.is_some(),
        })
        .count();
    let total = folders.len();
    let in_transit = total - in_cabinet;
    let distinct_patients: HashSet<Uuid> = folders.iter().map(|f| f.patient_id).collect();

    let now = chrono::Utc::now();
    let moves_24h = history
        .iter()
        .filter(|m| (now - m.moved_at) < chrono::Duration::hours(24))
        .count();

    Ok(Json(Stats {
        patients: distinct_patients.len(),
        folders: Folders {
            total,
            in_cabinet,
            in_transit,
        },
        places: Places {
            buildings: buildings.len(),
            rooms: rooms.len(),
            cabinets: cabinets.len(),
        },
        moves_24h,
    }))
}

/// Build a `folder_id → latest MoveEvent` map for cheap status lookup.
/// Shared with `controllers::folders::index`.
pub fn latest_move_per_folder(
    history: &[crate::main_event_service::MoveEvent],
) -> HashMap<Uuid, &crate::main_event_service::MoveEvent> {
    let mut out: HashMap<Uuid, &crate::main_event_service::MoveEvent> = HashMap::new();
    for ev in history {
        let entry = out.entry(ev.folder_id).or_insert(ev);
        if ev.moved_at > entry.moved_at {
            *entry = ev;
        }
    }
    out
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api/stats").add("/", get(index))
}
