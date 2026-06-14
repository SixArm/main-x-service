//! Optional e2e / offline mode for the Loco app.
//!
//! When the env var `USE_UPSTREAM_STUBS=1` is set, every Main-X-Service
//! client gets swapped for the in-process `StubClient` used by the
//! request tests, and the stubs are seeded with the same demo data
//! `cargo run -- task seed` would populate against real services.
//!
//! This lets `cargo run -- start` boot a fully functional API
//! end-to-end without needing the five external Main-X services to be
//! running — invaluable for the SvelteKit Playwright suite and for
//! local UI iteration.
//!
//! In production this initializer is a no-op (env var unset).

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Initializer},
};
use std::sync::Arc;

use crate::initializers::{
    main_event_service_client, main_patient_service_client, main_place_service_client,
    main_thing_service_client, main_worker_service_client,
};
use crate::main_event_service::{Client as EventClient, RecordMove, stub::StubClient as EventStub};
use crate::main_patient_service::stub::StubClient as PatientStub;
use crate::main_place_service::{
    Client as PlaceClient, Place, PlaceType, label_path, stub::StubClient as PlaceStub,
};
use crate::main_thing_service::{
    Client as ThingClient, Folder, NewVolume, stub::StubClient as ThingStub,
};
use crate::main_worker_service::{Worker, stub::StubClient as WorkerStub};
use crate::tasks::seed::run_seed;
use uuid::Uuid;

/// Loco initializer that, when enabled, replaces every external-service
/// client with an in-process stub and seeds demo data.
pub struct StubsInitializer;

/// Whether offline / e2e stub mode is requested via the
/// `USE_UPSTREAM_STUBS` env var. Accepted truthy values: `1`, `true`,
/// `TRUE`, `yes`. Anything else (including unset) means disabled.
fn enabled() -> bool {
    matches!(
        std::env::var("USE_UPSTREAM_STUBS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

#[async_trait]
impl Initializer for StubsInitializer {
    /// Stable identifier for this initializer in Loco's registry.
    fn name(&self) -> String {
        "bootstrap-stubs".to_string()
    }

    /// No-op unless [`enabled`]. When enabled: build one stub per
    /// external service, register each as the process-wide test client,
    /// seed a few demo workers, run the standard [`run_seed`] demo data
    /// set, then layer on demo worker-attributed moves and a demo volume.
    /// Returns the router unchanged (it only mutates global client slots).
    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        if !enabled() {
            return Ok(router);
        }

        tracing::info!(
            "USE_UPSTREAM_STUBS is set — swapping every Main-X-Service \
             client for in-process stubs and seeding demo data."
        );

        let patient_stub = Arc::new(PatientStub::new());
        let place_stub = Arc::new(PlaceStub::new());
        let worker_stub = Arc::new(WorkerStub::new());
        let thing_stub = Arc::new(ThingStub::new());
        let event_stub = Arc::new(EventStub::new());

        main_patient_service_client::set_test_client(patient_stub.clone());
        main_place_service_client::set_test_client(place_stub.clone());
        main_worker_service_client::set_test_client(worker_stub.clone());
        main_thing_service_client::set_test_client(thing_stub.clone());
        main_event_service_client::set_test_client(event_stub.clone());

        // A handful of demo workers — kept in named variables so the
        // demo moves below can be attributed to them. Attribution is what
        // makes the worker-detail and cabinet-presence-history views
        // non-empty in the offline demo.
        //
        // `alice_worker`: a nurse (seeded but not used to attribute a
        // move; she shares a name with the demo patient on purpose).
        let alice_worker = Worker {
            id: Uuid::new_v4(),
            name: "Alice Johnson".into(),
            role: Some("nurse".into()),
        };
        // `dr_bob`: a doctor used to attribute David Brown's demo move.
        let dr_bob = Worker {
            id: Uuid::new_v4(),
            name: "Dr Bob Carter".into(),
            role: Some("doctor".into()),
        };
        // `mira`: a records administrator used to attribute Carol's move.
        let mira = Worker {
            id: Uuid::new_v4(),
            name: "Mira (records)".into(),
            role: Some("administrator".into()),
        };
        worker_stub.seed(vec![alice_worker.clone(), dr_bob.clone(), mira.clone()]);

        if let Err(e) = run_seed(
            place_stub.as_ref(),
            thing_stub.as_ref(),
            event_stub.as_ref(),
            patient_stub.as_ref(),
        )
        .await
        {
            tracing::warn!(?e, "Failed to seed in-process stubs");
        } else {
            tracing::info!("In-process stubs seeded.");
            // Attribute a couple of moves to workers so the worker-detail
            // and cabinet-presence-history views are non-empty in the demo.
            seed_demo_worker_moves(&event_stub, &thing_stub, &place_stub, &dr_bob, &mira).await;
            // Bundle Alice's folders into a demo volume (left in place, so
            // tests that pin her folders' cabinets stay valid).
            seed_demo_volume(&thing_stub).await;
        }

        Ok(router)
    }
}

/// Bundle Alice Johnson's folders into a demo volume so the volume views
/// are populated. Membership only — folders are not relocated.
async fn seed_demo_volume(things: &ThingStub) {
    let folders = things.search("").await.unwrap_or_default();
    let alice: Vec<&Folder> = folders
        .iter()
        .filter(|f| f.patient_name_snapshot == "Alice Johnson")
        .collect();
    let Some(first) = alice.first() else {
        return;
    };
    let volume = match things
        .create_volume(NewVolume {
            patient_id: first.patient_id,
            nhs_number_snapshot: first.nhs_number_snapshot.clone(),
            patient_name_snapshot: first.patient_name_snapshot.clone(),
            title: "Alice Johnson — Vol 1".to_string(),
            cabinet_id: first.cabinet_id,
            cabinet_path_snapshot: first.cabinet_path_snapshot.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(_) => return,
    };
    for f in alice {
        let _ = things
            .set_folder_volume(f.id, Some(volume.id), Some(volume.title.clone()))
            .await;
    }
}

/// Record two worker-attributed moves over seeded demo data. Picks
/// Carol's and David's folders (Alice's `Volume 1 — General` is left in
/// Cabinet A1 because tests pin it there).
async fn seed_demo_worker_moves(
    events: &EventStub,
    things: &ThingStub,
    places: &PlaceStub,
    dr_bob: &Worker,
    mira: &Worker,
) {
    let folders = things.search("").await.unwrap_or_default();
    let cabinets = places
        .search("", Some(PlaceType::FILE_CABINET))
        .await
        .unwrap_or_default();
    let cabinet_named = |name: &str| cabinets.iter().find(|c| c.name == name).cloned();

    // Mira re-files Carol Williams' General folder (Cabinet A2 → C1).
    if let (Some(folder), Some(target)) = (
        folders
            .iter()
            .find(|f| f.patient_name_snapshot == "Carol Williams" && f.title == "General"),
        cabinet_named("Cabinet C1"),
    ) {
        record_demo_move(
            events,
            things,
            places,
            folder,
            &target,
            mira,
            "Outpatient appointment",
        )
        .await;
    }

    // Dr Bob Carter re-files David Brown's Outpatients folder (C1 → A2).
    if let (Some(folder), Some(target)) = (
        folders.iter().find(|f| f.title == "Outpatients 2026"),
        cabinet_named("Cabinet A2"),
    ) {
        record_demo_move(
            events,
            things,
            places,
            folder,
            &target,
            dr_bob,
            "Clinic review",
        )
        .await;
    }
}

/// Record one worker-attributed demo move and physically relocate the
/// folder to `target`.
///
/// Resolves the target cabinet's full label path (best-effort; falls
/// back to `"In transit"` when empty), writes a `RecordMove` to the
/// event stub attributed to `worker` with `reason`, then updates the
/// folder's cabinet in the thing stub. Errors from either call are
/// ignored — this is best-effort demo seeding.
async fn record_demo_move(
    events: &EventStub,
    things: &ThingStub,
    places: &PlaceStub,
    folder: &Folder,
    target: &Place,
    worker: &Worker,
    reason: &str,
) {
    let to_label = label_path(places, target.id).await.unwrap_or_default();
    let to_label_opt = if to_label.is_empty() {
        None
    } else {
        Some(to_label)
    };
    let _ = events
        .record(RecordMove {
            folder_id: folder.id,
            patient_id: folder.patient_id,
            nhs_number: folder.nhs_number_snapshot.clone(),
            patient_name: folder.patient_name_snapshot.clone(),
            folder_title: folder.title.clone(),
            from_cabinet_id: folder.cabinet_id,
            to_cabinet_id: Some(target.id),
            from_cabinet_label: folder
                .cabinet_path_snapshot
                .clone()
                .unwrap_or_else(|| "In transit".to_string()),
            to_cabinet_label: to_label_opt
                .clone()
                .unwrap_or_else(|| "In transit".to_string()),
            worker_id: Some(worker.id),
            moved_by: worker.name.clone(),
            worker_role_snapshot: worker.role.clone(),
            reason: Some(reason.to_string()),
        })
        .await;
    let _ = things
        .update_cabinet(folder.id, Some(target.id), to_label_opt)
        .await;
}
