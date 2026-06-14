//! Aggregate counters across every Main-X-Service.
//!
//! Returns folder / patient / cabinet / move tallies. When any single
//! upstream is unreachable that slice of the response is zero and a
//! warning is logged — the endpoint itself stays 200 because partial
//! data is more useful here than a hard failure.

use axum::{Extension, Json, debug_handler};
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

/// Top-level dashboard counters returned by `GET /api/stats`.
#[derive(Serialize)]
pub struct Stats {
    /// Distinct patients across all known folders.
    pub patients: usize,
    /// Folder tallies (total / in-cabinet / in-transit).
    pub folders: Folders,
    /// Place tallies (buildings / rooms / cabinets).
    pub places: Places,
    /// Number of move events recorded in the last 24 hours.
    pub moves_24h: usize,
}

/// Folder tallies. `total == in_cabinet + in_transit`.
#[derive(Serialize)]
pub struct Folders {
    /// All known folders.
    pub total: usize,
    /// Folders whose current location is a cabinet.
    pub in_cabinet: usize,
    /// Folders with no current cabinet (between locations).
    pub in_transit: usize,
}

/// Place tallies by kind, from the Main Place Service.
#[derive(Serialize)]
pub struct Places {
    /// Buildings (mapped to the `HOSPITAL` place type).
    pub buildings: usize,
    /// Rooms (mapped to the `RECORDS_ROOM` place type).
    pub rooms: usize,
    /// Cabinets (mapped to the `FILE_CABINET` place type).
    pub cabinets: usize,
}

/// `GET /api/stats` — aggregate dashboard counters across every upstream.
///
/// Tallies patients, folders (total / in-cabinet / in-transit), places
/// (buildings / rooms / cabinets), and moves in the last 24 hours. Each
/// upstream is queried independently and degrades gracefully: an
/// unreachable service contributes zeros and logs a warning rather than
/// failing the request.
///
/// A folder counts as `in_cabinet` when its latest move event has a
/// destination cabinet, falling back to the folder's own `cabinet_id`
/// when it has no recorded moves; `in_transit = total - in_cabinet`.
///
/// Request: none.
/// Response: `200 OK` with `Stats` (always `200`, even on partial
/// upstream failure).
///
/// # Errors
///
/// Returns `Err` only if response serialization fails; upstream outages
/// are absorbed as zeroed slices, never propagated as errors.
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

    // A folder is "in a cabinet" if its latest move has a destination
    // cabinet; with no moves on record, fall back to its own cabinet_id.
    let in_cabinet = folders
        .iter()
        .filter(|f| match latest_by_folder.get(&f.id) {
            Some(ev) => ev.to_cabinet_id.is_some(),
            None => f.cabinet_id.is_some(),
        })
        .count();
    let total = folders.len();
    // Everything not in a cabinet is in transit.
    let in_transit = total - in_cabinet;
    let distinct_patients: HashSet<Uuid> = folders.iter().map(|f| f.patient_id).collect();

    // Rolling 24-hour window measured from now.
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

/// Route table for the stats controller, mounted under `/api/stats`.
///
/// `GET /` (index).
pub fn routes() -> Routes {
    Routes::new().prefix("/api/stats").add("/", get(index))
}
