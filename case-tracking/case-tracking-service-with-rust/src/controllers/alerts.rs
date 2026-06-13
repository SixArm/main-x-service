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
    main_event_service::Client as MainEventServiceClient,
    main_place_service::{Client as MainPlaceServiceClient, PlaceType},
    responses::{self, List},
};

#[derive(Serialize)]
pub struct Alert {
    pub move_id: Uuid,
    pub folder_id: Uuid,
    pub folder_title: String,
    pub patient_name: String,
    pub nhs_number: String,
    pub from_building: String,
    pub to_building: String,
    pub from_cabinet_label: String,
    pub to_cabinet_label: String,
    pub moved_by: String,
    pub moved_at: String,
    pub reason: Option<String>,
}

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

    // cabinet id → building name, walking cabinet → room → building.
    let building_name: HashMap<Uuid, String> =
        buildings.iter().map(|b| (b.id, b.name.clone())).collect();
    let room_building: HashMap<Uuid, String> = rooms
        .iter()
        .filter_map(|r| {
            r.contained_in_place
                .and_then(|bid| building_name.get(&bid))
                .map(|name| (r.id, name.clone()))
        })
        .collect();
    let cabinet_building: HashMap<Uuid, String> = cabinets
        .iter()
        .filter_map(|c| {
            c.contained_in_place
                .and_then(|rid| room_building.get(&rid))
                .map(|name| (c.id, name.clone()))
        })
        .collect();

    let moves = match events.list_all().await {
        Ok(m) => m,
        Err(e) => {
            return responses::service_unavailable(format!("Main Event Service unreachable: {e}"))
        }
    };

    let alerts: Vec<Alert> = moves
        .iter()
        .filter_map(|m| {
            let from_cabinet = m.from_cabinet_id?;
            let to_cabinet = m.to_cabinet_id?;
            let from_building = cabinet_building.get(&from_cabinet)?;
            let to_building = cabinet_building.get(&to_cabinet)?;
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
        .collect();

    Json(List::new(alerts)).into_response()
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api/alerts").add("/", get(index))
}
