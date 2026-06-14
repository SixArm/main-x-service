//! Geofence alerts — an iFIT-inspired software feature (root spec D-12).
//!
//! A "geofence breach" is a move that took a folder **across a building
//! boundary**: its origin and destination cabinets resolve, via the
//! place hierarchy (cabinet → room → building), to different buildings.
//! Derived entirely from the move log + Main Place Service; no new
//! stored data.

use axum::{
    debug_handler,
    response::{IntoResponse, Response},
    Extension, Json,
};
use loco_rs::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    main_event_service::{Client as MainEventServiceClient, MoveEvent},
    main_place_service::{Client as MainPlaceServiceClient, Place, PlaceType},
    responses::{self, List},
};

/// One geofence-breach alert: a single move that carried a folder
/// across a building boundary. Serialised as a JSON object in the
/// `GET /api/alerts` list. All fields are denormalised snapshots taken
/// from the originating `MoveEvent` plus the resolved building names, so
/// the alert is self-contained and needs no further joins.
#[derive(Serialize)]
pub struct Alert {
    /// Id of the move that constitutes the breach.
    pub move_id: Uuid,
    /// Id of the folder that was moved.
    pub folder_id: Uuid,
    /// Human-readable folder title at the time of the move.
    pub folder_title: String,
    /// Patient name snapshot for the folder's patient.
    pub patient_name: String,
    /// Patient's NHS number (snapshot, formatted as stored on the move).
    pub nhs_number: String,
    /// Name of the building the folder moved **out of** (origin).
    pub from_building: String,
    /// Name of the building the folder moved **into** (destination) —
    /// differs from `from_building`, which is what makes this a breach.
    pub to_building: String,
    /// Label of the origin cabinet.
    pub from_cabinet_label: String,
    /// Label of the destination cabinet.
    pub to_cabinet_label: String,
    /// Display name of the worker who performed the move.
    pub moved_by: String,
    /// Timestamp of the move, RFC 3339 formatted.
    pub moved_at: String,
    /// Optional free-text reason recorded for the move.
    pub reason: Option<String>,
}

/// List all geofence-breach alerts.
///
/// Route: `GET /api/alerts`. Method: `GET`. Request: none. Response:
/// `List<Alert>` JSON.
///
/// Pulls the place hierarchy (buildings, records rooms, file cabinets)
/// from the Main Place Service and the full move log from the Main Event
/// Service, then runs the pure `detect_geofence_breaches` core.
///
/// Status codes:
/// - `200` — alerts (possibly empty) computed successfully.
/// - `503` — either upstream service is unreachable; no partial result
///   is returned because a missing place layer or move log would yield
///   false negatives.
#[debug_handler]
pub async fn index(
    Extension(places): Extension<Arc<dyn MainPlaceServiceClient>>,
    Extension(events): Extension<Arc<dyn MainEventServiceClient>>,
) -> Response {
    let (buildings, rooms, cabinets) = match (
        places.search("", Some(PlaceType::HOSPITAL)).await,
        places.search("", Some(PlaceType::RECORDS_ROOM)).await,
        places.search("", Some(PlaceType::FILE_CABINET)).await,
    ) {
        (Ok(b), Ok(r), Ok(c)) => (b, r, c),
        _ => return responses::service_unavailable("Main Place Service unreachable"),
    };

    let moves = match events.list_all().await {
        Ok(m) => m,
        Err(e) => {
            return responses::service_unavailable(format!("Main Event Service unreachable: {e}"))
        }
    };

    let alerts = detect_geofence_breaches(&buildings, &rooms, &cabinets, &moves);
    Json(List::new(alerts)).into_response()
}

/// Resolve each cabinet to the **building** that contains it, by walking
/// the place hierarchy cabinet → room → building. Cabinets whose room or
/// building is missing are simply absent from the result.
fn cabinet_buildings(
    buildings: &[Place],
    rooms: &[Place],
    cabinets: &[Place],
) -> HashMap<Uuid, String> {
    let building_name: HashMap<Uuid, &str> =
        buildings.iter().map(|b| (b.id, b.name.as_str())).collect();
    let room_building: HashMap<Uuid, &str> = rooms
        .iter()
        .filter_map(|r| {
            r.contained_in_place
                .and_then(|bid| building_name.get(&bid))
                .map(|name| (r.id, *name))
        })
        .collect();
    cabinets
        .iter()
        .filter_map(|c| {
            c.contained_in_place
                .and_then(|rid| room_building.get(&rid))
                .map(|name| (c.id, (*name).to_owned()))
        })
        .collect()
}

/// Derive geofence-breach alerts from the move log: a move is a breach
/// when its origin and destination cabinets both resolve, via the place
/// hierarchy, to **different** buildings.
///
/// A move is **not** a breach (and yields no alert) when:
/// - either endpoint cabinet is absent (`None` — created-in-place, or
///   sent in-transit with no destination), or
/// - either endpoint cabinet cannot be resolved to a building, or
/// - both endpoints resolve to the **same** building.
///
/// This is the pure core of `GET /api/alerts` (root spec D-12); the
/// handler only supplies the upstream data and serialises the result.
fn detect_geofence_breaches(
    buildings: &[Place],
    rooms: &[Place],
    cabinets: &[Place],
    moves: &[MoveEvent],
) -> Vec<Alert> {
    let cabinet_building = cabinet_buildings(buildings, rooms, cabinets);
    moves
        .iter()
        .filter_map(|m| {
            let from_building = cabinet_building.get(&m.from_cabinet_id?)?;
            let to_building = cabinet_building.get(&m.to_cabinet_id?)?;
            if from_building == to_building {
                return None;
            }
            Some(Alert {
                move_id: m.id,
                folder_id: m.folder_id,
                folder_title: m.folder_title.clone(),
                patient_name: m.patient_name.clone(),
                nhs_number: m.nhs_number.clone(),
                from_building: from_building.clone(),
                to_building: to_building.clone(),
                from_cabinet_label: m.from_cabinet_label.clone(),
                to_cabinet_label: m.to_cabinet_label.clone(),
                moved_by: m.moved_by.clone(),
                moved_at: m.moved_at.to_rfc3339(),
                reason: m.reason.clone(),
            })
        })
        .collect()
}

/// Mount the alerts route under the `/api/alerts` prefix: `GET /`.
pub fn routes() -> Routes {
    Routes::new().prefix("/api/alerts").add("/", get(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Build a `Place` fixture with just the fields the breach logic
    /// reads: `id`, `name`, and the `contained_in_place` parent link.
    /// All other place fields are left empty/`None`.
    fn place(id: Uuid, name: &str, parent: Option<Uuid>) -> Place {
        Place {
            id,
            name: name.to_owned(),
            place_type: None,
            description: None,
            contained_in_place: parent,
            capacity: None,
        }
    }

    /// Build a `MoveEvent` fixture parameterised only by its origin and
    /// destination cabinet ids (the inputs the breach logic keys on).
    /// All other fields use fixed placeholder values, since the detector
    /// merely copies them through to the resulting `Alert`.
    fn move_event(from: Option<Uuid>, to: Option<Uuid>) -> MoveEvent {
        MoveEvent {
            id: Uuid::new_v4(),
            folder_id: Uuid::new_v4(),
            patient_id: Uuid::new_v4(),
            nhs_number: "943 476 5919".to_owned(),
            patient_name: "Alice Johnson".to_owned(),
            folder_title: "Volume 1".to_owned(),
            from_cabinet_id: from,
            to_cabinet_id: to,
            from_cabinet_label: "Cab A".to_owned(),
            to_cabinet_label: "Cab B".to_owned(),
            worker_id: None,
            moved_by: "Joe Porter".to_owned(),
            worker_role_snapshot: None,
            moved_at: Utc::now(),
            reason: None,
        }
    }

    /// A complete two-building estate: Hospital A (Room A1 → Cab A) and
    /// Hospital B (Room B1 → Cab B).
    fn estate() -> (Vec<Place>, Vec<Place>, Vec<Place>, Uuid, Uuid) {
        let (b1, r1, cab_a) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (b2, r2, cab_b) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let buildings = vec![place(b1, "Hospital A", None), place(b2, "Hospital B", None)];
        let rooms = vec![
            place(r1, "Room A1", Some(b1)),
            place(r2, "Room B1", Some(b2)),
        ];
        let cabinets = vec![
            place(cab_a, "Cab A", Some(r1)),
            place(cab_b, "Cab B", Some(r2)),
        ];
        (buildings, rooms, cabinets, cab_a, cab_b)
    }

    /// Pins the happy path: a move from a cabinet in Hospital A to a
    /// cabinet in Hospital B yields exactly one alert, with the origin
    /// and destination building names resolved correctly.
    #[test]
    fn cross_building_move_is_a_breach() {
        let (b, r, c, cab_a, cab_b) = estate();
        let alerts = detect_geofence_breaches(&b, &r, &c, &[move_event(Some(cab_a), Some(cab_b))]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].from_building, "Hospital A");
        assert_eq!(alerts[0].to_building, "Hospital B");
    }

    /// Pins that a move between two cabinets in the **same** building
    /// (both under Room A1) raises no alert — only boundary crossings
    /// count.
    #[test]
    fn same_building_move_is_not_a_breach() {
        // Two cabinets in the same building (both under Room A1).
        let (b1, r1) = (Uuid::new_v4(), Uuid::new_v4());
        let (cab1, cab2) = (Uuid::new_v4(), Uuid::new_v4());
        let buildings = vec![place(b1, "Hospital A", None)];
        let rooms = vec![place(r1, "Room A1", Some(b1))];
        let cabinets = vec![
            place(cab1, "Cab 1", Some(r1)),
            place(cab2, "Cab 2", Some(r1)),
        ];
        let alerts = detect_geofence_breaches(
            &buildings,
            &rooms,
            &cabinets,
            &[move_event(Some(cab1), Some(cab2))],
        );
        assert!(alerts.is_empty());
    }

    /// Pins that a `None` endpoint cabinet (created-in-place with no
    /// origin, or sent in-transit with no destination) is ignored:
    /// there is no boundary to cross, so neither direction yields an
    /// alert.
    #[test]
    fn move_with_missing_endpoint_cabinet_is_not_a_breach() {
        let (b, r, c, cab_a, _cab_b) = estate();
        // Created in place (no origin) and sent in-transit (no destination)
        // are both ignored — there is no boundary crossing to detect.
        let none_from = move_event(None, Some(cab_a));
        let none_to = move_event(Some(cab_a), None);
        assert!(detect_geofence_breaches(&b, &r, &c, &[none_from]).is_empty());
        assert!(detect_geofence_breaches(&b, &r, &c, &[none_to]).is_empty());
    }

    /// Pins that an endpoint cabinet id unknown to the place hierarchy
    /// resolves to no building and is therefore skipped — guarding
    /// against false alerts from dangling references.
    #[test]
    fn move_via_unresolvable_cabinet_is_not_a_breach() {
        let (b, r, c, cab_a, _cab_b) = estate();
        // A cabinet id the place hierarchy does not know about cannot be
        // resolved to a building, so the move is skipped (no false alert).
        let unknown = Uuid::new_v4();
        let alerts =
            detect_geofence_breaches(&b, &r, &c, &[move_event(Some(cab_a), Some(unknown))]);
        assert!(alerts.is_empty());
    }

    /// Pins resolution of an orphaned hierarchy: a cabinet whose room
    /// exists but whose room points at a missing building resolves to no
    /// building. Checks both `cabinet_buildings` (the orphan is absent /
    /// the good cabinet present) and that a move through the orphan
    /// raises no alert.
    #[test]
    fn cabinet_under_orphan_room_is_unresolved() {
        // Cabinet → Room exists, but the room's building is missing, so
        // the cabinet resolves to no building (orphaned hierarchy).
        let (b1, r1, r_orphan) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (cab_a, cab_orphan) = (Uuid::new_v4(), Uuid::new_v4());
        let buildings = vec![place(b1, "Hospital A", None)];
        let rooms = vec![
            place(r1, "Room A1", Some(b1)),
            // Orphan room points at a building id that is not present.
            place(r_orphan, "Orphan Room", Some(Uuid::new_v4())),
        ];
        let cabinets = vec![
            place(cab_a, "Cab A", Some(r1)),
            place(cab_orphan, "Cab Orphan", Some(r_orphan)),
        ];
        let resolved = cabinet_buildings(&buildings, &rooms, &cabinets);
        assert_eq!(resolved.get(&cab_a).map(String::as_str), Some("Hospital A"));
        assert!(!resolved.contains_key(&cab_orphan));
        // A move through the orphan cabinet therefore raises no alert.
        let alerts = detect_geofence_breaches(
            &buildings,
            &rooms,
            &cabinets,
            &[move_event(Some(cab_a), Some(cab_orphan))],
        );
        assert!(alerts.is_empty());
    }

    /// Pins the filter behaviour over a mixed log: from four moves
    /// (cross-building, same-cabinet, created-in-place, and a reverse
    /// cross-building), exactly the two genuine breaches are returned.
    #[test]
    fn only_breaching_moves_are_returned_from_a_mixed_log() {
        let (b, r, c, cab_a, cab_b) = estate();
        let moves = vec![
            move_event(Some(cab_a), Some(cab_b)), // breach
            move_event(Some(cab_a), Some(cab_a)), // same cabinet → same building
            move_event(None, Some(cab_b)),        // created in place
            move_event(Some(cab_b), Some(cab_a)), // breach (reverse direction)
        ];
        let alerts = detect_geofence_breaches(&b, &r, &c, &moves);
        assert_eq!(alerts.len(), 2);
    }
}
