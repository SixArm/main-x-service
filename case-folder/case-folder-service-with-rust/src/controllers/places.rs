//! Places API — thin proxy over the Main Place Service.
//!
//! `GET /api/places` returns buildings, rooms, and cabinets grouped by
//! kind. Folder counts per cabinet come from the Main Thing Service so
//! callers don't need a second round-trip.
//!
//! `POST /api/places` writes back to the Main Place Service. The
//! tracker keeps no local places table.

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
    main_event_service::{Client as MainEventServiceClient, MoveEvent},
    main_place_service::{label_path, Client as MainPlaceServiceClient, CreatePlace, PlaceType},
    main_thing_service::Client as MainThingServiceClient,
    responses::{self, Cabinet, Place},
};

/// Query parameters accepted by `GET /api/places`.
#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    /// Free-text search forwarded to the Main Place Service.
    pub q: Option<String>,
    /// Restrict the listing to one kind: `"building"`, `"room"`, or
    /// `"cabinet"`. Any other value (or absence) returns all three.
    pub kind: Option<String>,
}

/// Response body for `GET /api/places` — places grouped by kind.
#[derive(Serialize)]
pub struct PlacesIndex {
    /// Echoes the effective search term, or `None` when empty.
    pub query: Option<String>,
    /// Echoes the effective kind filter, or `None` when unset.
    pub kind: Option<String>,
    /// Matching buildings (`HOSPITAL` place type).
    pub buildings: Vec<Place>,
    /// Matching rooms (`RECORDS_ROOM` place type).
    pub rooms: Vec<Place>,
    /// Matching cabinets (`FILE_CABINET` place type), each with its
    /// current folder count.
    pub cabinets: Vec<Cabinet>,
}

/// `GET /api/places` — list buildings, rooms, and cabinets grouped by
/// kind.
///
/// Queries the Main Place Service three times (one per kind) and, for
/// cabinets, joins per-cabinet folder counts from the Main Thing
/// Service. The `kind` parameter maps `building → HOSPITAL`,
/// `room → RECORDS_ROOM`, `cabinet → FILE_CABINET`; a recognised kind
/// blanks out the other two groups. Each place gets a human-readable
/// container path ("Building — Room") resolved locally from the fetched
/// sets.
///
/// Request: query params `q`, `kind` — see `FilterParams`.
/// Response: `200 OK` with `PlacesIndex`.
/// Errors: `503 Service Unavailable` if the Main Place Service is
/// unreachable. The folder-count fetch is best-effort.
#[debug_handler]
pub async fn index(
    Extension(mps): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Query(params): Query<FilterParams>,
) -> Response {
    let q = params.q.clone().unwrap_or_default();
    let kind_filter = params.kind.clone().unwrap_or_default();

    // Map the public `kind` string onto the Place Service's place type.
    let place_type_filter = match kind_filter.as_str() {
        "building" => Some(PlaceType::HOSPITAL),
        "room" => Some(PlaceType::RECORDS_ROOM),
        "cabinet" => Some(PlaceType::FILE_CABINET),
        _ => None,
    };

    let (buildings, rooms, cabinets) = match (
        mps.search(&q, Some(PlaceType::HOSPITAL)).await,
        mps.search(&q, Some(PlaceType::RECORDS_ROOM)).await,
        mps.search(&q, Some(PlaceType::FILE_CABINET)).await,
    ) {
        (Ok(b), Ok(r), Ok(c)) => (b, r, c),
        _ => {
            return responses::service_unavailable("Main Place Service unreachable");
        }
    };

    let buildings_by_id: HashMap<Uuid, &crate::main_place_service::Place> =
        buildings.iter().map(|p| (p.id, p)).collect();
    let rooms_by_id: HashMap<Uuid, &crate::main_place_service::Place> =
        rooms.iter().map(|p| (p.id, p)).collect();

    // Pre-tally folders per cabinet so cabinet views avoid a second hop.
    let folders_list = things.search("").await.unwrap_or_default();
    let mut folders_per_cabinet: HashMap<Uuid, usize> = HashMap::new();
    for f in &folders_list {
        if let Some(id) = f.cabinet_id {
            *folders_per_cabinet.entry(id).or_insert(0) += 1;
        }
    }

    // Resolve a place's parent path label from the already-fetched sets:
    // a cabinet's parent is a room ("Building — Room"); a room's parent
    // is a building. Returns "" when the parent is unknown.
    let container_path_of = |p: &crate::main_place_service::Place| -> String {
        p.contained_in_place
            .and_then(|cid| {
                rooms_by_id
                    .get(&cid)
                    .map(|r| {
                        let bname = r
                            .contained_in_place
                            .and_then(|bid| buildings_by_id.get(&bid).map(|b| b.name.clone()))
                            .unwrap_or_default();
                        if bname.is_empty() {
                            r.name.clone()
                        } else {
                            format!("{} — {}", bname, r.name)
                        }
                    })
                    .or_else(|| buildings_by_id.get(&cid).map(|b| b.name.clone()))
            })
            .unwrap_or_default()
    };

    let visible_buildings =
        if place_type_filter.is_some_and(|t| t != PlaceType::HOSPITAL) && !kind_filter.is_empty() {
            Vec::new()
        } else {
            buildings
                .iter()
                .map(|p| responses::place(p, container_path_of(p)))
                .collect()
        };
    let visible_rooms = if place_type_filter.is_some_and(|t| t != PlaceType::RECORDS_ROOM)
        && !kind_filter.is_empty()
    {
        Vec::new()
    } else {
        rooms
            .iter()
            .map(|p| responses::place(p, container_path_of(p)))
            .collect()
    };
    let visible_cabinets = if place_type_filter.is_some_and(|t| t != PlaceType::FILE_CABINET)
        && !kind_filter.is_empty()
    {
        Vec::new()
    } else {
        cabinets
            .iter()
            .map(|p| Cabinet {
                place: responses::place(p, container_path_of(p)),
                folder_count: folders_per_cabinet.get(&p.id).copied().unwrap_or(0),
            })
            .collect()
    };

    Json(PlacesIndex {
        query: if q.is_empty() { None } else { Some(q) },
        kind: if kind_filter.is_empty() {
            None
        } else {
            Some(kind_filter)
        },
        buildings: visible_buildings,
        rooms: visible_rooms,
        cabinets: visible_cabinets,
    })
    .into_response()
}

/// Response body for `GET /api/places/{id}`.
#[derive(Serialize)]
pub struct PlaceShow {
    /// The place view, flattened into the top-level JSON object.
    #[serde(flatten)]
    pub place: Place,
    /// Folders parked in this cabinet (only populated for cabinets).
    pub folders: Vec<responses::Folder>,
}

/// `GET /api/places/{id}` — show one place.
///
/// Fetches the place from the Main Place Service and resolves its
/// container path label. For cabinets only, attaches the folders
/// currently filed there (scanning the Main Thing Service folder set);
/// buildings and rooms get an empty `folders` list.
///
/// Request: path param `id` (place UUID).
/// Response: `200 OK` with `PlaceShow`.
/// Errors: `404 Not Found` if no such place; `503 Service Unavailable`
/// if the Main Place Service is unreachable.
#[debug_handler]
pub async fn show(
    Extension(mps): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(things): Extension<Arc<dyn MainThingServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let place = match mps.find_by_id(id).await {
        Ok(Some(p)) => p,
        Ok(None) => return responses::not_found("Place not found"),
        Err(e) => {
            tracing::warn!(?e, "Main Place Service unreachable");
            return responses::service_unavailable(format!("Main Place Service unreachable: {e}"));
        }
    };
    let container = label_path(mps.as_ref(), place.contained_in_place.unwrap_or(place.id))
        .await
        .unwrap_or_default();
    let place_view = responses::place(&place, container);

    let folders = if place.place_type.as_deref() == Some(PlaceType::FILE_CABINET) {
        let all = things.search("").await.unwrap_or_default();
        all.iter()
            .filter(|f| f.cabinet_id == Some(id))
            .map(|f| {
                let label = f
                    .cabinet_path_snapshot
                    .clone()
                    .unwrap_or_else(|| "In transit".to_string());
                responses::folder(f, label, None)
            })
            .collect()
    } else {
        vec![]
    };

    Json(PlaceShow {
        place: place_view,
        folders,
    })
    .into_response()
}

/// Request body for `POST /api/places`.
#[derive(Debug, Deserialize)]
pub struct CreatePlaceInput {
    /// Place name. Required.
    pub name: String,
    /// Kind: `"building"`, `"room"`, or `"cabinet"`. Any other value is
    /// a `422`.
    pub kind: String,
    /// Optional parent place UUID (e.g. the room a cabinet sits in).
    pub contained_in_place: Option<String>,
    /// Optional free-text description.
    pub description: Option<String>,
    /// Optional capacity hint (e.g. folder slots).
    pub capacity: Option<i32>,
}

/// `POST /api/places` — create a building, room, or cabinet.
///
/// Validates the name and kind, maps the kind to the Place Service's
/// place type, and writes the place back to the Main Place Service (the
/// tracker keeps no local places table). Resolves and returns the new
/// place's container path label.
///
/// Request: JSON `CreatePlaceInput`.
/// Response: `201 Created` with the place view and a `Location:
/// /api/places/{id}` header.
/// Errors: `422 Unprocessable Entity` for a missing name, an
/// unrecognised kind, or a Place Service write error.
#[debug_handler]
pub async fn create(
    Extension(mps): Extension<Arc<dyn MainPlaceServiceClient>>,
    Json(input): Json<CreatePlaceInput>,
) -> Response {
    let mut errors: HashMap<String, String> = HashMap::new();
    if input.name.trim().is_empty() {
        errors.insert("name".into(), "Name is required.".into());
    }
    let place_type = match input.kind.as_str() {
        "building" => PlaceType::HOSPITAL,
        "room" => PlaceType::RECORDS_ROOM,
        "cabinet" => PlaceType::FILE_CABINET,
        _ => {
            errors.insert(
                "kind".into(),
                "Pick a kind: building, room, or cabinet.".into(),
            );
            ""
        }
    };
    let parent = input
        .contained_in_place
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok());

    if !errors.is_empty() {
        return responses::unprocessable(errors);
    }

    let create = CreatePlace {
        name: input.name.trim().to_string(),
        place_type: place_type.to_string(),
        description: input.description.clone().filter(|s| !s.trim().is_empty()),
        contained_in_place: parent,
        capacity: input.capacity,
    };

    match mps.create(create).await {
        Ok(p) => {
            let container = label_path(mps.as_ref(), p.contained_in_place.unwrap_or(p.id))
                .await
                .unwrap_or_default();
            let body = responses::place(&p, container);
            let location = format!("/api/places/{}", p.id);
            (
                StatusCode::CREATED,
                [(header::LOCATION, location)],
                Json(body),
            )
                .into_response()
        }
        Err(e) => {
            errors.insert("name".into(), format!("Main Place Service error: {e}"));
            responses::unprocessable(errors)
        }
    }
}

/// One stay of a folder in a cabinet: entered at one move, left at the
/// next (or `null` if still resident). See root spec D-10.
#[derive(Serialize)]
pub struct Presence {
    /// Folder that was present.
    pub folder_id: Uuid,
    /// Folder title snapshotted at the entering move.
    pub folder_title: String,
    /// Patient owning the folder.
    pub patient_id: Uuid,
    /// Patient NHS Number snapshot.
    pub nhs_number: String,
    /// Patient name snapshot.
    pub patient_name: String,
    /// Cabinet the folder was present in.
    pub cabinet_id: Uuid,
    /// Resolved cabinet path label.
    pub cabinet_label: String,
    /// RFC 3339 timestamp the folder entered the cabinet.
    pub entered_at: String,
    /// RFC 3339 timestamp the folder left, or `None` if still resident.
    pub left_at: Option<String>,
    /// Reason recorded on the entering move, if any.
    pub entered_reason: Option<String>,
    /// Reason recorded on the leaving move, if any.
    pub left_reason: Option<String>,
}

/// Response body for `GET /api/places/{id}/history`.
#[derive(Serialize)]
pub struct PlaceHistory {
    /// The place view, flattened into the top-level JSON object.
    #[serde(flatten)]
    pub place: Place,
    /// Presence intervals across every cabinet this place covers,
    /// newest first.
    pub presences: Vec<Presence>,
}

/// `GET /api/places/{id}/history` — presence history for a place.
///
/// Computes, for every cabinet the place covers, the intervals during
/// which folders were resident. Coverage resolution: a cabinet covers
/// itself; a room covers its cabinets; a building covers all cabinets in
/// all its rooms. Move events are then paired (enter/leave) into
/// presence intervals via `build_presences`.
///
/// Request: path param `id` (place UUID).
/// Response: `200 OK` with `PlaceHistory`.
/// Errors: `404 Not Found` if no such place; `503 Service Unavailable`
/// if the Main Place Service is unreachable. Event/cabinet/room fetches
/// are best-effort.
#[debug_handler]
pub async fn history(
    Extension(mps): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
    Path(id): Path<Uuid>,
) -> Response {
    let place = match mps.find_by_id(id).await {
        Ok(Some(p)) => p,
        Ok(None) => return responses::not_found("Place not found"),
        Err(e) => {
            return responses::service_unavailable(format!("Main Place Service unreachable: {e}"))
        }
    };

    // Which cabinets does this place cover? A cabinet is itself; a room
    // is its cabinets; a building is all cabinets in all its rooms.
    let cabinets = mps
        .search("", Some(PlaceType::FILE_CABINET))
        .await
        .unwrap_or_default();
    let rooms = mps
        .search("", Some(PlaceType::RECORDS_ROOM))
        .await
        .unwrap_or_default();

    let covered: Vec<Uuid> = match place.place_type.as_deref() {
        Some(PlaceType::FILE_CABINET) => vec![place.id],
        Some(PlaceType::RECORDS_ROOM) => cabinets
            .iter()
            .filter(|c| c.contained_in_place == Some(place.id))
            .map(|c| c.id)
            .collect(),
        Some(PlaceType::HOSPITAL) => {
            let room_ids: HashSet<Uuid> = rooms
                .iter()
                .filter(|r| r.contained_in_place == Some(place.id))
                .map(|r| r.id)
                .collect();
            cabinets
                .iter()
                .filter(|c| {
                    c.contained_in_place
                        .map(|rid| room_ids.contains(&rid))
                        .unwrap_or(false)
                })
                .map(|c| c.id)
                .collect()
        }
        _ => vec![place.id],
    };
    let covered_set: HashSet<Uuid> = covered.iter().copied().collect();

    let mut labels: HashMap<Uuid, String> = HashMap::new();
    for cid in &covered {
        let label = label_path(mps.as_ref(), *cid).await.unwrap_or_default();
        labels.insert(*cid, label);
    }

    let all_events = events.list_all().await.unwrap_or_default();
    let presences = build_presences(&all_events, &covered_set, &labels);

    let container = label_path(mps.as_ref(), place.contained_in_place.unwrap_or(place.id))
        .await
        .unwrap_or_default();

    Json(PlaceHistory {
        place: responses::place(&place, container),
        presences,
    })
    .into_response()
}

/// Pair `to_cabinet` (enter) with the next `from_cabinet` (leave) per
/// (cabinet, folder) to produce presence intervals. An open interval
/// (still resident) has `left_at = None`. Newest interval first.
fn build_presences(
    events: &[MoveEvent],
    covered: &HashSet<Uuid>,
    labels: &HashMap<Uuid, String>,
) -> Vec<Presence> {
    // Bucket each relevant move under (cabinet, folder). A move counts
    // as an enter for its destination and a leave for its origin; the
    // origin == destination case is skipped (no-op self moves).
    let mut grouped: HashMap<(Uuid, Uuid), Vec<&MoveEvent>> = HashMap::new();
    for ev in events {
        if let Some(to) = ev.to_cabinet_id {
            if covered.contains(&to) {
                grouped.entry((to, ev.folder_id)).or_default().push(ev);
            }
        }
        if let Some(from) = ev.from_cabinet_id {
            if covered.contains(&from) && Some(from) != ev.to_cabinet_id {
                grouped.entry((from, ev.folder_id)).or_default().push(ev);
            }
        }
    }

    let mut out: Vec<Presence> = Vec::new();
    for ((cabinet_id, _folder_id), mut evs) in grouped {
        // Walk this folder's moves for this cabinet in time order,
        // pairing each enter with the next leave.
        evs.sort_by_key(|e| e.moved_at);
        let label = labels.get(&cabinet_id).cloned().unwrap_or_default();
        let mut entered: Option<&MoveEvent> = None;
        for ev in evs {
            let is_enter = ev.to_cabinet_id == Some(cabinet_id);
            let is_leave = ev.from_cabinet_id == Some(cabinet_id);
            if is_enter && entered.is_none() {
                entered = Some(ev);
            } else if is_leave {
                if let Some(en) = entered.take() {
                    out.push(presence_from(en, Some(ev), cabinet_id, &label));
                }
            }
        }
        // A dangling enter with no matching leave is an open interval
        // (folder still resident): `left_at = None`.
        if let Some(en) = entered.take() {
            out.push(presence_from(en, None, cabinet_id, &label));
        }
    }

    // Newest interval first.
    out.sort_by(|a, b| b.entered_at.cmp(&a.entered_at));
    out
}

/// Build a `Presence` from an entering move and an optional leaving move.
///
/// Folder / patient identity and the entering reason come from `enter`;
/// `left_at` and `left_reason` come from `leave` (`None` ⇒ still
/// resident). `cabinet_id` / `label` identify the cabinet.
fn presence_from(
    enter: &MoveEvent,
    leave: Option<&MoveEvent>,
    cabinet_id: Uuid,
    label: &str,
) -> Presence {
    Presence {
        folder_id: enter.folder_id,
        folder_title: enter.folder_title.clone(),
        patient_id: enter.patient_id,
        nhs_number: enter.nhs_number.clone(),
        patient_name: enter.patient_name.clone(),
        cabinet_id,
        cabinet_label: label.to_string(),
        entered_at: enter.moved_at.to_rfc3339(),
        left_at: leave.map(|l| l.moved_at.to_rfc3339()),
        entered_reason: enter.reason.clone(),
        left_reason: leave.and_then(|l| l.reason.clone()),
    }
}

/// Route table for the places controller, mounted under `/api/places`.
///
/// `GET /` (index), `POST /` (create), `GET /{id}` (show),
/// `GET /{id}/history`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/places")
        .add("/", get(index))
        .add("/", post(create))
        .add("/{id}", get(show))
        .add("/{id}/history", get(history))
}
