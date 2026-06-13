//! Serializable response structs for the JSON API.
//!
//! Each shape is the wire contract that clients depend on. Source rows
//! come exclusively from external-service projections (the tracker
//! keeps no local tables): `Patient` from the Main Patient Service,
//! `Worker` from the Main Worker Service, `Place` from the Main Place
//! Service, `Folder` from the Main Thing Service, and `MoveEvent`
//! from the Main Event Service.
//!
//! Cosmetic helpers (badge classes, display-formatted dates) that the
//! old server-rendered HTML needed are intentionally absent — clients
//! format their own UI. Dates and NHS Numbers are emitted in their
//! canonical machine forms (RFC 3339 / `XXX XXX XXXX`).

use crate::main_event_service;
use crate::main_patient_service;
use crate::main_place_service;
use crate::main_thing_service;
use crate::main_worker_service;
use serde::Serialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Patients
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Patient {
    pub id: Uuid,
    pub nhs_number: String,
    pub name: String,
    pub date_of_birth: Option<String>,
    pub folder_count: usize,
    pub source: &'static str,
}

pub fn patient(p: &main_patient_service::Patient, folder_count: usize) -> Patient {
    Patient {
        id: p.id,
        nhs_number: p.nhs_number.clone(),
        name: p.name.clone(),
        date_of_birth: p.date_of_birth.map(|d| d.format("%Y-%m-%d").to_string()),
        folder_count,
        source: "Main Patient Service",
    }
}

// ---------------------------------------------------------------------------
// Places
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Place {
    pub id: Uuid,
    pub name: String,
    pub place_type: String,
    pub place_kind: &'static str,
    pub description: Option<String>,
    pub contained_in_place: Option<Uuid>,
    pub container_path: String,
    pub capacity: Option<i32>,
    pub source: &'static str,
}

pub fn place_kind_for(place_type: Option<&str>) -> &'static str {
    match place_type {
        Some(main_place_service::PlaceType::HOSPITAL) => "building",
        Some(main_place_service::PlaceType::RECORDS_ROOM) => "room",
        Some(main_place_service::PlaceType::FILE_CABINET) => "cabinet",
        _ => "other",
    }
}

pub fn place(p: &main_place_service::Place, container_path: String) -> Place {
    Place {
        id: p.id,
        name: p.name.clone(),
        place_type: p.place_type.clone().unwrap_or_default(),
        place_kind: place_kind_for(p.place_type.as_deref()),
        description: p.description.clone(),
        contained_in_place: p.contained_in_place,
        container_path,
        capacity: p.capacity,
        source: "Main Place Service",
    }
}

#[derive(Serialize)]
pub struct Cabinet {
    #[serde(flatten)]
    pub place: Place,
    pub folder_count: usize,
}

// ---------------------------------------------------------------------------
// Folders
// ---------------------------------------------------------------------------

pub const STATUS_IN_CABINET: &str = "in-cabinet";
pub const STATUS_IN_TRANSIT: &str = "in-transit";

#[derive(Serialize)]
pub struct Folder {
    pub id: Uuid,
    pub title: String,
    pub patient_id: Uuid,
    pub nhs_number: String,
    pub patient_name: String,
    pub cabinet_id: Option<Uuid>,
    pub cabinet_label: String,
    pub status: &'static str,
    pub last_moved_at: Option<String>,
    pub notes: Option<String>,
    pub volume_id: Option<Uuid>,
    pub volume_title: Option<String>,
}

pub fn folder(
    f: &main_thing_service::Folder,
    cabinet_label: String,
    latest_move: Option<&main_event_service::MoveEvent>,
) -> Folder {
    let status = match latest_move {
        Some(ev) if ev.to_cabinet_id.is_some() => STATUS_IN_CABINET,
        Some(_) => STATUS_IN_TRANSIT,
        None if f.cabinet_id.is_some() => STATUS_IN_CABINET,
        None => STATUS_IN_TRANSIT,
    };
    let last_moved_at = latest_move.map(|e| e.moved_at.to_rfc3339());
    Folder {
        id: f.id,
        title: f.title.clone(),
        patient_id: f.patient_id,
        nhs_number: f.nhs_number_snapshot.clone(),
        patient_name: f.patient_name_snapshot.clone(),
        cabinet_id: f.cabinet_id,
        cabinet_label,
        status,
        last_moved_at,
        notes: f.notes.clone(),
        volume_id: f.volume_id,
        volume_title: f.volume_title_snapshot.clone(),
    }
}

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Volume {
    pub id: Uuid,
    pub title: String,
    pub patient_id: Uuid,
    pub nhs_number: String,
    pub patient_name: String,
    pub cabinet_id: Option<Uuid>,
    pub cabinet_label: String,
    pub status: &'static str,
    pub folder_count: usize,
}

pub fn volume(
    v: &main_thing_service::Volume,
    cabinet_label: String,
    folder_count: usize,
) -> Volume {
    let status = if v.cabinet_id.is_some() {
        STATUS_IN_CABINET
    } else {
        STATUS_IN_TRANSIT
    };
    Volume {
        id: v.id,
        title: v.title.clone(),
        patient_id: v.patient_id,
        nhs_number: v.nhs_number_snapshot.clone(),
        patient_name: v.patient_name_snapshot.clone(),
        cabinet_id: v.cabinet_id,
        cabinet_label,
        status,
        folder_count,
    }
}

// ---------------------------------------------------------------------------
// Move events
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Move {
    pub id: Uuid,
    pub folder_id: Uuid,
    pub folder_title: String,
    pub patient_id: Uuid,
    pub nhs_number: String,
    pub patient_name: String,
    pub from_cabinet_id: Option<Uuid>,
    pub from_cabinet_label: String,
    pub to_cabinet_id: Option<Uuid>,
    pub to_cabinet_label: String,
    pub worker_id: Option<Uuid>,
    pub moved_by: String,
    pub worker_role: Option<String>,
    pub moved_at: String,
    pub reason: Option<String>,
}

pub fn move_event(m: &main_event_service::MoveEvent) -> Move {
    Move {
        id: m.id,
        folder_id: m.folder_id,
        folder_title: m.folder_title.clone(),
        patient_id: m.patient_id,
        nhs_number: m.nhs_number.clone(),
        patient_name: m.patient_name.clone(),
        from_cabinet_id: m.from_cabinet_id,
        from_cabinet_label: m.from_cabinet_label.clone(),
        to_cabinet_id: m.to_cabinet_id,
        to_cabinet_label: m.to_cabinet_label.clone(),
        worker_id: m.worker_id,
        moved_by: m.moved_by.clone(),
        worker_role: m.worker_role_snapshot.clone(),
        moved_at: m.moved_at.to_rfc3339(),
        reason: m.reason.clone(),
    }
}

// ---------------------------------------------------------------------------
// Workers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Worker {
    pub id: Uuid,
    pub name: String,
    pub role: Option<String>,
}

pub fn worker(w: &main_worker_service::Worker) -> Worker {
    Worker {
        id: w.id,
        name: w.name.clone(),
        role: w.role.clone(),
    }
}

// ---------------------------------------------------------------------------
// Envelopes
// ---------------------------------------------------------------------------

/// List envelope. Echoes the query (if any) so clients can confirm
/// what they searched for without parsing the request URL.
#[derive(Serialize)]
pub struct List<T: Serialize> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl<T: Serialize> List<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items, query: None }
    }

    pub fn with_query(items: Vec<T>, query: impl Into<String>) -> Self {
        let q = query.into();
        Self {
            items,
            query: if q.is_empty() { None } else { Some(q) },
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct ErrorBody {
    pub error: String,
}

#[derive(Serialize)]
pub struct ValidationBody {
    pub errors: HashMap<String, String>,
}

pub fn not_found(message: impl Into<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}

pub fn unprocessable(errors: HashMap<String, String>) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ValidationBody { errors }),
    )
        .into_response()
}

pub fn unauthorized(message: impl Into<String>) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}

pub fn service_unavailable(message: impl Into<String>) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}
