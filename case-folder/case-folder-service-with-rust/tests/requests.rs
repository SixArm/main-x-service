// Request-level integration tests for the Loco app.
//
// Each submodule covers one controller. They share the helpers defined
// below for resetting the in-process stub services so each test starts
// from a known clean slate. Case Tracking no longer keeps any
// local database tables; everything is proxied to one of:
//
//   * Main Patient Service  (stub injected here)
//   * Main Place   Service  (stub injected here)
//   * Main Worker  Service  (stub injected here)
//   * Main Thing   Service  (stub injected here — holds folders)
//   * Main Event   Service  (stub injected here — holds move events)

use case_folder_service_with_rust::{
    initializers::{
        main_event_service_client, main_patient_service_client, main_place_service_client,
        main_thing_service_client, main_worker_service_client,
    },
    main_event_service::stub::StubClient as EventStubClient,
    main_patient_service::{Patient, stub::StubClient as PatientStubClient},
    main_place_service::{Place, PlaceType, stub::StubClient as PlaceStubClient},
    main_thing_service::{Folder, stub::StubClient as ThingStubClient},
    main_worker_service::{Worker, stub::StubClient as WorkerStubClient},
    nhs::format_nhs_number,
};
use loco_rs::prelude::*;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

#[path = "requests/alerts.rs"]
mod alerts;
#[path = "requests/auth.rs"]
mod auth;
#[path = "requests/folders.rs"]
mod folders;
#[path = "requests/home.rs"]
mod home;
#[path = "requests/moves.rs"]
mod moves;
#[path = "requests/patients.rs"]
mod patients;
#[path = "requests/places.rs"]
mod places;
#[path = "requests/volumes.rs"]
mod volumes;
#[path = "requests/workers.rs"]
mod workers;

/// Lazily create the Main Patient Service stub and register it as the
/// process-wide test client (once, via `OnceLock`); subsequent calls
/// return the same handle so tests can seed/inspect it.
pub fn main_patient_service() -> Arc<PatientStubClient> {
    static STUB: OnceLock<Arc<PatientStubClient>> = OnceLock::new();
    STUB.get_or_init(|| {
        let stub = Arc::new(PatientStubClient::new());
        main_patient_service_client::set_test_client(stub.clone());
        stub
    })
    .clone()
}

/// Lazily create the Main Place Service stub and register it as the
/// process-wide test client (once); returns the shared handle.
pub fn main_place_service() -> Arc<PlaceStubClient> {
    static STUB: OnceLock<Arc<PlaceStubClient>> = OnceLock::new();
    STUB.get_or_init(|| {
        let stub = Arc::new(PlaceStubClient::new());
        main_place_service_client::set_test_client(stub.clone());
        stub
    })
    .clone()
}

/// Lazily create the Main Worker Service stub and register it as the
/// process-wide test client (once); returns the shared handle.
pub fn main_worker_service() -> Arc<WorkerStubClient> {
    static STUB: OnceLock<Arc<WorkerStubClient>> = OnceLock::new();
    STUB.get_or_init(|| {
        let stub = Arc::new(WorkerStubClient::new());
        main_worker_service_client::set_test_client(stub.clone());
        stub
    })
    .clone()
}

/// Lazily create the Main Thing Service stub (holds folders + volumes)
/// and register it as the process-wide test client (once); returns the
/// shared handle.
pub fn main_thing_service() -> Arc<ThingStubClient> {
    static STUB: OnceLock<Arc<ThingStubClient>> = OnceLock::new();
    STUB.get_or_init(|| {
        let stub = Arc::new(ThingStubClient::new());
        main_thing_service_client::set_test_client(stub.clone());
        stub
    })
    .clone()
}

/// Lazily create the Main Event Service stub (holds move events) and
/// register it as the process-wide test client (once); returns the
/// shared handle.
pub fn main_event_service() -> Arc<EventStubClient> {
    static STUB: OnceLock<Arc<EventStubClient>> = OnceLock::new();
    STUB.get_or_init(|| {
        let stub = Arc::new(EventStubClient::new());
        main_event_service_client::set_test_client(stub.clone());
        stub
    })
    .clone()
}

/// Reset every stub so each test starts from a known clean slate.
/// `_ctx` is kept for signature parity with the prior helper — the
/// tracker has no local DB tables to truncate any more.
pub async fn clean_db(_ctx: &AppContext) {
    main_patient_service().clear();
    main_place_service().clear();
    main_worker_service().clear();
    main_thing_service().clear();
    main_event_service().clear();
}

/// Pull the bare session JWT out of a `Set-Cookie: cts_session=…; …`
/// header so a follow-up request can present it as a Bearer token.
pub fn session_token_from_set_cookie(header: &str) -> String {
    header
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .strip_prefix("cts_session=")
        .unwrap_or("")
        .to_string()
}

/// Inject a top-level building (`HOSPITAL` place, no parent) into the
/// Place stub and return it.
pub fn seed_building(name: &str) -> Place {
    let building = Place {
        id: Uuid::new_v4(),
        name: name.to_string(),
        place_type: Some(PlaceType::HOSPITAL.to_string()),
        description: None,
        contained_in_place: None,
        capacity: None,
    };
    main_place_service().seed(vec![building.clone()]);
    building
}

/// Inject a records room (`RECORDS_ROOM` place) contained in
/// `building_id` into the Place stub and return it.
pub fn seed_room(building_id: Uuid, name: &str) -> Place {
    let room = Place {
        id: Uuid::new_v4(),
        name: name.to_string(),
        place_type: Some(PlaceType::RECORDS_ROOM.to_string()),
        description: None,
        contained_in_place: Some(building_id),
        capacity: None,
    };
    main_place_service().seed(vec![room.clone()]);
    room
}

/// Inject a file cabinet (`FILE_CABINET` place, capacity 100) contained
/// in `room_id` into the Place stub and return it.
pub fn seed_cabinet(room_id: Uuid, label: &str) -> Place {
    let cabinet = Place {
        id: Uuid::new_v4(),
        name: label.to_string(),
        place_type: Some(PlaceType::FILE_CABINET.to_string()),
        description: None,
        contained_in_place: Some(room_id),
        capacity: Some(100),
    };
    main_place_service().seed(vec![cabinet.clone()]);
    cabinet
}

/// Inject a full building → room → cabinet chain named after
/// `cabinet_label` and return the leaf cabinet. Convenience for tests
/// that only care about the cabinet but need a valid containment path.
pub fn seed_location_chain(cabinet_label: &str) -> Place {
    let b = seed_building(&format!("Building for {cabinet_label}"));
    let r = seed_room(b.id, &format!("Room for {cabinet_label}"));
    seed_cabinet(r.id, cabinet_label)
}

/// Reconstruct the label path a cabinet seeded by [`seed_location_chain`]
/// would carry, e.g. `"Building for X — Room for X — X"`.
pub fn cabinet_path(cabinet: &Place) -> String {
    format!(
        "Building for {label} — Room for {label} — {label}",
        label = cabinet.name
    )
}

/// Inject a patient (NHS number normalized via `format_nhs_number`) into
/// the Patient stub and return it.
pub fn seed_patient_in_main_patient_service(nhs: &str, name: &str) -> Patient {
    let patient = Patient {
        id: Uuid::new_v4(),
        nhs_number: format_nhs_number(nhs),
        name: name.to_string(),
        date_of_birth: None,
    };
    main_patient_service().seed(vec![patient.clone()]);
    patient
}

/// Inject a worker (optional role) into the Worker stub and return it.
pub fn seed_worker_in_main_worker_service(name: &str, role: Option<&str>) -> Worker {
    let worker = Worker {
        id: Uuid::new_v4(),
        name: name.to_string(),
        role: role.map(str::to_string),
    };
    main_worker_service().seed(vec![worker.clone()]);
    worker
}

/// Inject a folder into the Thing stub for `patient`, optionally filed
/// in `cabinet` (with NHS-number / name / cabinet-path snapshots copied
/// from the patient and cabinet), and return it.
pub fn seed_folder(patient: &Patient, title: &str, cabinet: Option<&Place>) -> Folder {
    let folder = Folder {
        id: Uuid::new_v4(),
        title: title.to_string(),
        patient_id: patient.id,
        nhs_number_snapshot: patient.nhs_number.clone(),
        patient_name_snapshot: patient.name.clone(),
        cabinet_id: cabinet.map(|c| c.id),
        cabinet_path_snapshot: cabinet.map(cabinet_path),
        notes: None,
        volume_id: None,
        volume_title_snapshot: None,
    };
    main_thing_service().seed(vec![folder.clone()]);
    folder
}
