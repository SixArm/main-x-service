#![warn(clippy::pedantic)]

//! FHIR reads are audited as REST reads are (HIPAA §164.312(b): reads are
//! activity). With `PERSON_AUDIT_READS` on, `GET /fhir/Patient/{id}` and the
//! `Person` alias each write the `read` row REST `GET /api/persons/{id}`
//! writes against the person, and a FHIR search writes the one `search` row
//! against the nil id that REST search writes.
//!
//! Its **own test binary**: `PERSON_AUDIT_READS` is a process-wide
//! `OnceLock` read on first use, and this test turns it on.
//!
//! `#[ignore]`d — requires `PostgreSQL`. Run with
//! `cargo test --test fhir_read_audit -- --ignored`.

mod common;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::NaiveDate;
use person_service::api::fhir::handlers::{
    FhirSearchParams, get_fhir_patient, get_fhir_person_alias, search_fhir_patients,
};
use person_service::api::rest::AppState;
use person_service::api::rest::auth::MaybeAuthUser;
use person_service::api::rest::handlers::get_person;
use person_service::compliance::disclosure::AccessContext;
use person_service::db::AuditContext;
use person_service::models::{Gender, HumanName, Person};
use serial_test::serial;
use uuid::Uuid;

fn access() -> AccessContext {
    AccessContext::from_parts(Some("treatment"), None, None)
}

/// The `action` rows recorded against `entity_id`, newest first.
async fn rows(state: &AppState, entity_id: Uuid, action: &str) -> Vec<serde_json::Value> {
    state
        .audit_log
        .get_logs_for_entity("Person", entity_id, 10_000)
        .await
        .expect("read the audit trail")
        .into_iter()
        .filter(|row| row.action == action)
        .map(|row| {
            serde_json::json!({
                "action": row.action,
                "entity_type": row.entity_type,
                "user_id": row.user_id,
                "context": row.context,
            })
        })
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --test fhir_read_audit -- --ignored`"]
async fn fhir_reads_and_searches_write_the_rows_rest_writes() {
    // Set before the state is built: read once into a `OnceLock`.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup.
    unsafe {
        std::env::set_var("PERSON_AUDIT_READS", "1");
    }
    let state = common::create_test_app_state().await;

    let family = format!("Fhira{}", &Uuid::new_v4().simple().to_string()[..12]);
    let mut p = Person::new(
        HumanName {
            use_type: None,
            family: family.clone(),
            given: vec!["Rowan".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    );
    p.birth_date = NaiveDate::from_ymd_opt(1979, 3, 8);
    let created = state
        .person_repository
        .create(&p, &AuditContext::default())
        .await
        .expect("seed a person");
    state
        .search_engine
        .index_person(&created)
        .expect("index the person");
    let id = created.id;

    // A FHIR read, then its Person alias, then the REST read.
    let response = get_fhir_patient(
        State(state.clone()),
        Path(id.to_string()),
        MaybeAuthUser(None),
        access(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        rows(&state, id, "read").await.len(),
        1,
        "FHIR read recorded"
    );
    let response = get_fhir_person_alias(
        State(state.clone()),
        Path(id.to_string()),
        MaybeAuthUser(None),
        access(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        rows(&state, id, "read").await.len(),
        2,
        "alias read recorded"
    );
    let response = axum::response::IntoResponse::into_response(
        get_person(
            State(state.clone()),
            axa::extract::Path(id),
            MaybeAuthUser(None),
            access(),
        )
        .await,
    );
    assert_eq!(response.status(), StatusCode::OK);
    let reads = rows(&state, id, "read").await;
    assert_eq!(reads.len(), 3);
    // Newest first: the REST row, then the two FHIR rows, all alike.
    assert_eq!(reads[1], reads[0], "a FHIR read row is the REST read row");
    assert_eq!(reads[2], reads[0], "an alias read row is the REST read row");

    // A FHIR search writes one `search` row against the nil id.
    let before = rows(&state, Uuid::nil(), "search").await.len();
    let response = search_fhir_patients(
        State(state.clone()),
        Query(FhirSearchParams {
            name: Some(family),
            family: None,
            given: None,
            identifier: None,
            birth_date: None,
            gender: None,
            count: Some(10),
        }),
        MaybeAuthUser(None),
        access(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        rows(&state, Uuid::nil(), "search").await.len(),
        before + 1,
        "FHIR search recorded once, against the nil id"
    );
}
