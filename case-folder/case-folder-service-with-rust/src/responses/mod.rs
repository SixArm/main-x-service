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

/// Wire shape for a patient, projected from the Main Patient Service.
#[derive(Serialize)]
pub struct Patient {
    /// Stable patient identifier (from the upstream service).
    pub id: Uuid,
    /// NHS Number in canonical `XXX XXX XXXX` form.
    pub nhs_number: String,
    /// Patient display name.
    pub name: String,
    /// Date of birth as `YYYY-MM-DD`, or `None` if unknown.
    pub date_of_birth: Option<String>,
    /// Number of folders tracked for this patient.
    pub folder_count: usize,
    /// Provenance label for clients (`"Main Patient Service"`).
    pub source: &'static str,
}

/// Project an upstream patient plus a folder count into the wire shape.
///
/// Clones the upstream fields and formats the date of birth as
/// `YYYY-MM-DD`. The `source` is stamped as `"Main Patient Service"`.
///
/// # Parameters
/// - `p`: the upstream patient record.
/// - `folder_count`: how many folders this patient has.
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

/// Wire shape for a place (hospital, records room, cabinet, ...),
/// projected from the Main Place Service.
#[derive(Serialize)]
pub struct Place {
    /// Stable place identifier (from the upstream service).
    pub id: Uuid,
    /// Place display name.
    pub name: String,
    /// Raw upstream place-type string (may be empty if absent).
    pub place_type: String,
    /// Coarse UI kind derived from `place_type` (see [`place_kind_for`]).
    pub place_kind: &'static str,
    /// Optional free-text description.
    pub description: Option<String>,
    /// Parent place id, if this place is nested in another.
    pub contained_in_place: Option<Uuid>,
    /// Human-readable containment breadcrumb (e.g. hospital > room).
    pub container_path: String,
    /// Optional folder capacity for cabinet-like places.
    pub capacity: Option<i32>,
    /// Provenance label for clients (`"Main Place Service"`).
    pub source: &'static str,
}

/// Map an upstream place-type string to a coarse UI kind.
///
/// `Hospital` -> `"building"`, `RecordsRoom` -> `"room"`,
/// `FileCabinet` -> `"cabinet"`, and anything else (or `None`) ->
/// `"other"`.
///
/// # Parameters
/// - `place_type`: the upstream place-type string, if present.
pub fn place_kind_for(place_type: Option<&str>) -> &'static str {
    match place_type {
        Some(main_place_service::PlaceType::HOSPITAL) => "building",
        Some(main_place_service::PlaceType::RECORDS_ROOM) => "room",
        Some(main_place_service::PlaceType::FILE_CABINET) => "cabinet",
        _ => "other",
    }
}

/// Project an upstream place plus its containment path into the wire shape.
///
/// Derives `place_kind` from the upstream place type and stamps the
/// `source` as `"Main Place Service"`. An absent place type becomes an
/// empty string.
///
/// # Parameters
/// - `p`: the upstream place record.
/// - `container_path`: pre-computed containment breadcrumb.
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

/// A cabinet-kind [`Place`] enriched with the number of folders it holds.
#[derive(Serialize)]
pub struct Cabinet {
    /// The underlying place, flattened into this struct's JSON fields.
    #[serde(flatten)]
    pub place: Place,
    /// Count of folders currently located in this cabinet.
    pub folder_count: usize,
}

// ---------------------------------------------------------------------------
// Folders
// ---------------------------------------------------------------------------

/// Status value for a folder that currently rests in a cabinet.
pub const STATUS_IN_CABINET: &str = "in-cabinet";
/// Status value for a folder in transit (not in any cabinet).
pub const STATUS_IN_TRANSIT: &str = "in-transit";

/// Wire shape for a case-note folder, projected from the Main Thing Service.
#[derive(Serialize)]
pub struct Folder {
    /// Stable folder identifier (from the upstream service).
    pub id: Uuid,
    /// Folder title.
    pub title: String,
    /// Owning patient's id.
    pub patient_id: Uuid,
    /// Snapshot of the patient's NHS Number at last sync.
    pub nhs_number: String,
    /// Snapshot of the patient's name at last sync.
    pub patient_name: String,
    /// Cabinet the folder is in, if any.
    pub cabinet_id: Option<Uuid>,
    /// Human-readable cabinet label (empty when not in a cabinet).
    pub cabinet_label: String,
    /// Derived location status: `STATUS_IN_CABINET` or `STATUS_IN_TRANSIT`.
    pub status: &'static str,
    /// RFC 3339 timestamp of the latest move, if one exists.
    pub last_moved_at: Option<String>,
    /// Optional free-text notes.
    pub notes: Option<String>,
    /// Parent volume id, if this folder belongs to a volume.
    pub volume_id: Option<Uuid>,
    /// Parent volume title snapshot, if any.
    pub volume_title: Option<String>,
}

/// Project an upstream folder into the wire shape, deriving its status.
///
/// Status is computed from the latest move event when available,
/// otherwise from the folder's own cabinet field:
/// - latest move *with* a destination cabinet -> `STATUS_IN_CABINET`;
/// - latest move *without* a destination cabinet -> `STATUS_IN_TRANSIT`;
/// - no move, but the folder records a cabinet -> `STATUS_IN_CABINET`;
/// - no move and no cabinet -> `STATUS_IN_TRANSIT`.
///
/// # Parameters
/// - `f`: the upstream folder record.
/// - `cabinet_label`: human-readable label for the folder's cabinet.
/// - `latest_move`: the most recent move event for this folder, if any.
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

/// Wire shape for a volume (a bound collection of folders), projected
/// from the Main Thing Service.
#[derive(Serialize)]
pub struct Volume {
    /// Stable volume identifier (from the upstream service).
    pub id: Uuid,
    /// Volume title.
    pub title: String,
    /// Owning patient's id.
    pub patient_id: Uuid,
    /// Snapshot of the patient's NHS Number at last sync.
    pub nhs_number: String,
    /// Snapshot of the patient's name at last sync.
    pub patient_name: String,
    /// Cabinet the volume is in, if any.
    pub cabinet_id: Option<Uuid>,
    /// Human-readable cabinet label (empty when not in a cabinet).
    pub cabinet_label: String,
    /// Derived location status: `STATUS_IN_CABINET` or `STATUS_IN_TRANSIT`.
    pub status: &'static str,
    /// Number of folders contained in this volume.
    pub folder_count: usize,
}

/// Project an upstream volume into the wire shape, deriving its status.
///
/// Status is `STATUS_IN_CABINET` when the volume records a cabinet, else
/// `STATUS_IN_TRANSIT`. (Unlike folders, volumes do not consult move
/// events here.)
///
/// # Parameters
/// - `v`: the upstream volume record.
/// - `cabinet_label`: human-readable label for the volume's cabinet.
/// - `folder_count`: number of folders in the volume.
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

/// Wire shape for a folder move event, projected from the Main Event Service.
#[derive(Serialize)]
pub struct Move {
    /// Stable move-event identifier (from the upstream service).
    pub id: Uuid,
    /// Id of the folder that was moved.
    pub folder_id: Uuid,
    /// Title of the moved folder (snapshot).
    pub folder_title: String,
    /// Owning patient's id.
    pub patient_id: Uuid,
    /// Patient's NHS Number (snapshot).
    pub nhs_number: String,
    /// Patient's name (snapshot).
    pub patient_name: String,
    /// Source cabinet id, if the folder came from a cabinet.
    pub from_cabinet_id: Option<Uuid>,
    /// Human-readable source cabinet label.
    pub from_cabinet_label: String,
    /// Destination cabinet id, if the folder went into a cabinet.
    pub to_cabinet_id: Option<Uuid>,
    /// Human-readable destination cabinet label.
    pub to_cabinet_label: String,
    /// Id of the worker who performed the move, if known.
    pub worker_id: Option<Uuid>,
    /// Display name of who moved the folder.
    pub moved_by: String,
    /// Worker role snapshot at the time of the move.
    pub worker_role: Option<String>,
    /// RFC 3339 timestamp of the move.
    pub moved_at: String,
    /// Optional free-text reason for the move.
    pub reason: Option<String>,
}

/// Project an upstream move event into the wire shape.
///
/// Clones the upstream fields and renders `moved_at` as RFC 3339.
///
/// # Parameters
/// - `m`: the upstream move-event record.
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

/// Wire shape for a worker, projected from the Main Worker Service.
#[derive(Serialize)]
pub struct Worker {
    /// Stable worker identifier (from the upstream service).
    pub id: Uuid,
    /// Worker display name.
    pub name: String,
    /// Optional worker role.
    pub role: Option<String>,
}

/// Project an upstream worker into the wire shape.
///
/// # Parameters
/// - `w`: the upstream worker record.
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
    /// The listed items.
    pub items: Vec<T>,
    /// The query that produced this list, echoed back; omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl<T: Serialize> List<T> {
    /// Wrap `items` in a list envelope with no associated query.
    pub fn new(items: Vec<T>) -> Self {
        Self { items, query: None }
    }

    /// Wrap `items` in a list envelope, echoing the search `query`.
    ///
    /// An empty query string is normalised to `None` so it is omitted
    /// from the serialized JSON.
    ///
    /// # Parameters
    /// - `items`: the listed items.
    /// - `query`: the query string to echo back.
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

/// Single-message error body: `{ "error": "..." }`.
#[derive(Serialize)]
pub struct ErrorBody {
    /// Human-readable error message.
    pub error: String,
}

/// Field-keyed validation error body: `{ "errors": { field: msg } }`.
#[derive(Serialize)]
pub struct ValidationBody {
    /// Map of field name to validation message.
    pub errors: HashMap<String, String>,
}

/// Build a `404 Not Found` response with an [`ErrorBody`] JSON payload.
///
/// # Parameters
/// - `message`: the error message to report.
pub fn not_found(message: impl Into<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}

/// Build a `422 Unprocessable Entity` response with a [`ValidationBody`]
/// JSON payload of per-field errors.
///
/// # Parameters
/// - `errors`: map of field name to validation message.
pub fn unprocessable(errors: HashMap<String, String>) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ValidationBody { errors }),
    )
        .into_response()
}

/// Build a `401 Unauthorized` response with an [`ErrorBody`] JSON payload.
///
/// # Parameters
/// - `message`: the error message to report.
pub fn unauthorized(message: impl Into<String>) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}

/// Build a `503 Service Unavailable` response with an [`ErrorBody`] JSON
/// payload — used when an upstream Main-X service is unreachable.
///
/// # Parameters
/// - `message`: the error message to report.
pub fn service_unavailable(message: impl Into<String>) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}
