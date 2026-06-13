//! `seed` task — registers demo data across **all** the external
//! services the tracker depends on:
//!
//! - Main Place Service: 3 buildings + 4 rooms + 5 cabinets
//! - Main Patient Service: 6 demo patients
//! - Main Thing Service: 9 demo folders (one Thing per folder)
//! - Main Event Service: one synthetic "Folder created" move event per folder
//!
//! Run with:
//!   cargo run -- task seed
//!
//! Idempotent. If any service is unreachable the task logs a warning
//! and continues with placeholder data so the rest of the demo still
//! produces useful rows.

use chrono::NaiveDate;
use loco_rs::prelude::*;
use uuid::Uuid;

use crate::main_event_service::{self, RecordMove};
use crate::main_patient_service::{self, CreatePatient};
use crate::main_place_service::{
    self, Client as MainPlaceServiceClient, CreatePlace, Place, PlaceType,
};
use crate::main_thing_service::{self, Client as MainThingServiceClient, NewFolder};

pub struct Seed;

#[async_trait]
impl Task for Seed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Register demo buildings/rooms/cabinets (Place service), patients (Patient service), folders (Thing service), and seed move events (Event service).".to_string(),
        }
    }

    async fn run(&self, _ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let place_client = main_place_service::http::HttpClient::new(env_or(
            "MAIN_PLACE_SERVICE_BASE_URL",
            "http://localhost:5153",
        ));
        let thing_client = main_thing_service::http::HttpClient::new(env_or(
            "MAIN_THING_SERVICE_BASE_URL",
            "http://localhost:5154",
        ));
        let event_client = main_event_service::http::HttpClient::new(env_or(
            "MAIN_EVENT_SERVICE_BASE_URL",
            "http://localhost:5155",
        ));
        let patient_client = main_patient_service::http::HttpClient::new(env_or(
            "MAIN_PATIENT_SERVICE_BASE_URL",
            "http://localhost:5151",
        ));

        run_seed(&place_client, &thing_client, &event_client, &patient_client).await
    }
}

/// Seed the demo data set against any combination of Place / Thing /
/// Event / Patient clients. The `Seed` task uses this against HTTP
/// clients pointed at the real services; the `bootstrap_stubs`
/// initializer uses it against in-process stubs for offline / e2e
/// runs.
pub async fn run_seed(
    place_client: &dyn MainPlaceServiceClient,
    thing_client: &dyn MainThingServiceClient,
    event_client: &dyn main_event_service::Client,
    patient_client: &dyn main_patient_service::Client,
) -> Result<()> {
    let cabs = match seed_places(place_client).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "Main Place Service unavailable while seeding places: {} — using placeholder UUIDs",
                e
            );
            CabinetsSeed::placeholders()
        }
    };
    seed_folders_and_moves(patient_client, thing_client, event_client, &cabs).await?;
    Ok(())
}

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| fallback.to_string())
}

struct CabinetsSeed {
    a1: (Uuid, String),
    a2: (Uuid, String),
    b1: (Uuid, String),
    c1: (Uuid, String),
    arc: (Uuid, String),
}

impl CabinetsSeed {
    fn placeholders() -> Self {
        let mk = |label: &str| (Uuid::new_v4(), label.to_string());
        Self {
            a1: mk("Main Hospital — Ward A — Cabinet A1"),
            a2: mk("Main Hospital — Ward A — Cabinet A2"),
            b1: mk("Main Hospital — Ward B — Cabinet B1"),
            c1: mk("Outpatients Wing — Reception — Cabinet C1"),
            arc: mk("Off-site Archive — Basement — Archive Cabinet 1"),
        }
    }
}

async fn seed_places(
    client: &dyn MainPlaceServiceClient,
) -> std::result::Result<CabinetsSeed, main_place_service::Error> {
    let main_hospital = ensure_place(
        client,
        "Main Hospital",
        PlaceType::HOSPITAL,
        None,
        None,
        None,
    )
    .await?;
    let outpatients = ensure_place(
        client,
        "Outpatients Wing",
        PlaceType::HOSPITAL,
        None,
        None,
        None,
    )
    .await?;
    let archive_building = ensure_place(
        client,
        "Off-site Archive",
        PlaceType::HOSPITAL,
        None,
        None,
        None,
    )
    .await?;

    let ward_a = ensure_place(
        client,
        "Ward A Records Room",
        PlaceType::RECORDS_ROOM,
        Some(main_hospital.id),
        None,
        None,
    )
    .await?;
    let ward_b = ensure_place(
        client,
        "Ward B Records Room",
        PlaceType::RECORDS_ROOM,
        Some(main_hospital.id),
        None,
        None,
    )
    .await?;
    let recep = ensure_place(
        client,
        "Outpatients Reception",
        PlaceType::RECORDS_ROOM,
        Some(outpatients.id),
        None,
        None,
    )
    .await?;
    let basement = ensure_place(
        client,
        "Long-term Archive Basement",
        PlaceType::RECORDS_ROOM,
        Some(archive_building.id),
        None,
        None,
    )
    .await?;

    let a1 = ensure_place(
        client,
        "Cabinet A1",
        PlaceType::FILE_CABINET,
        Some(ward_a.id),
        Some(120),
        Some("Active admissions"),
    )
    .await?;
    let a2 = ensure_place(
        client,
        "Cabinet A2",
        PlaceType::FILE_CABINET,
        Some(ward_a.id),
        Some(120),
        None,
    )
    .await?;
    let b1 = ensure_place(
        client,
        "Cabinet B1",
        PlaceType::FILE_CABINET,
        Some(ward_b.id),
        Some(80),
        None,
    )
    .await?;
    let c1 = ensure_place(
        client,
        "Cabinet C1",
        PlaceType::FILE_CABINET,
        Some(recep.id),
        Some(60),
        None,
    )
    .await?;
    let arc = ensure_place(
        client,
        "Archive Cabinet 1",
        PlaceType::FILE_CABINET,
        Some(basement.id),
        Some(500),
        Some("Discharged > 12 months"),
    )
    .await?;

    let path_for = |cab: &str, parent: &str, building: &str| -> String {
        format!("{building} — {parent} — {cab}")
    };

    Ok(CabinetsSeed {
        a1: (
            a1.id,
            path_for("Cabinet A1", "Ward A Records Room", "Main Hospital"),
        ),
        a2: (
            a2.id,
            path_for("Cabinet A2", "Ward A Records Room", "Main Hospital"),
        ),
        b1: (
            b1.id,
            path_for("Cabinet B1", "Ward B Records Room", "Main Hospital"),
        ),
        c1: (
            c1.id,
            path_for("Cabinet C1", "Outpatients Reception", "Outpatients Wing"),
        ),
        arc: (
            arc.id,
            path_for(
                "Archive Cabinet 1",
                "Long-term Archive Basement",
                "Off-site Archive",
            ),
        ),
    })
}

async fn ensure_place(
    client: &dyn MainPlaceServiceClient,
    name: &str,
    place_type: &str,
    parent: Option<Uuid>,
    capacity: Option<i32>,
    description: Option<&str>,
) -> std::result::Result<Place, main_place_service::Error> {
    let matches = client.search(name, Some(place_type)).await?;
    if let Some(existing) = matches
        .into_iter()
        .find(|p| p.name == name && p.contained_in_place == parent)
    {
        return Ok(existing);
    }
    client
        .create(CreatePlace {
            name: name.to_string(),
            place_type: place_type.to_string(),
            description: description.map(str::to_string),
            contained_in_place: parent,
            capacity,
        })
        .await
}

async fn seed_folders_and_moves(
    patient_client: &dyn main_patient_service::Client,
    thing_client: &dyn MainThingServiceClient,
    event_client: &dyn main_event_service::Client,
    cabs: &CabinetsSeed,
) -> Result<()> {
    type FolderSpec<'a> = (&'a str, Option<(Uuid, String)>);
    let seeds: Vec<(&str, &str, NaiveDate, Vec<FolderSpec>)> = vec![
        (
            "943 476 5919",
            "Alice Johnson",
            date(1991, 4, 12),
            vec![
                ("Volume 1 — General", Some(cabs.a1.clone())),
                ("Maternity 2023", Some(cabs.a1.clone())),
            ],
        ),
        (
            "987 654 3210",
            "Bob Smith",
            date(1958, 11, 2),
            vec![
                ("Cardiology 2019", Some(cabs.arc.clone())),
                ("General — Archived", Some(cabs.arc.clone())),
            ],
        ),
        (
            "999 999 9999",
            "Carol Williams",
            date(1981, 7, 30),
            vec![("General", Some(cabs.a2.clone()))],
        ),
        (
            "614 309 0432",
            "David Brown",
            date(1974, 2, 18),
            vec![
                ("General", Some(cabs.c1.clone())),
                ("Outpatients 2026", Some(cabs.c1.clone())),
            ],
        ),
        (
            "630 162 4483",
            "Eleanor Patel",
            date(1962, 9, 5),
            vec![("General", None)],
        ),
        (
            "485 777 3457",
            "Frank O\u{2019}Connor",
            date(1949, 1, 22),
            vec![("General", Some(cabs.b1.clone()))],
        ),
    ];

    for (nhs, name, dob, folder_specs) in seeds {
        let (patient_id, patient_name) = match main_patient_service::find_or_create(
            patient_client,
            CreatePatient {
                nhs_number: nhs.to_string(),
                name: name.to_string(),
                date_of_birth: dob,
            },
        )
        .await
        {
            Ok(p) => (p.id, p.name),
            Err(e) => {
                tracing::warn!(
                    "Main Patient Service unavailable while seeding {}: {} — using local placeholder UUID",
                    nhs,
                    e
                );
                (Uuid::new_v4(), name.to_string())
            }
        };

        for (title, cabinet) in folder_specs {
            let (cabinet_id, cabinet_path) = match cabinet {
                Some((id, p)) => (Some(id), Some(p)),
                None => (None, None),
            };

            // Skip if the Thing service already has this folder.
            let existing = thing_client
                .list_for_patient(patient_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .find(|f| f.title == title);
            let folder = match existing {
                Some(f) => f,
                None => {
                    match thing_client
                        .create(NewFolder {
                            patient_id,
                            nhs_number_snapshot: crate::nhs::format_nhs_number(nhs),
                            patient_name_snapshot: patient_name.clone(),
                            title: title.to_string(),
                            cabinet_id,
                            cabinet_path_snapshot: cabinet_path.clone(),
                            notes: None,
                            volume_id: None,
                            volume_title_snapshot: None,
                        })
                        .await
                    {
                        Ok(f) => f,
                        Err(e) => {
                            tracing::warn!(
                                "Main Thing Service unavailable while seeding folder {} for {}: {} — skipping",
                                title,
                                nhs,
                                e
                            );
                            continue;
                        }
                    }
                }
            };

            // Best-effort initial move event. Skip if the Event
            // service already has one for this folder (idempotency).
            let prior = event_client
                .list_for_folder(folder.id)
                .await
                .unwrap_or_default();
            if prior.is_empty() {
                if let Err(e) = event_client
                    .record(RecordMove {
                        folder_id: folder.id,
                        patient_id: folder.patient_id,
                        nhs_number: folder.nhs_number_snapshot.clone(),
                        patient_name: folder.patient_name_snapshot.clone(),
                        folder_title: folder.title.clone(),
                        from_cabinet_id: None,
                        to_cabinet_id: cabinet_id,
                        from_cabinet_label: "(new folder)".into(),
                        to_cabinet_label: cabinet_path
                            .clone()
                            .unwrap_or_else(|| "In transit".to_string()),
                        worker_id: None,
                        moved_by: "System (seed)".to_string(),
                        worker_role_snapshot: None,
                        reason: Some("Folder created".to_string()),
                    })
                    .await
                {
                    tracing::warn!(
                        "Main Event Service unavailable while seeding move for {}: {}",
                        folder.title,
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}
