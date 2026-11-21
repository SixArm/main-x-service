//! FHIR R5 `Practitioner` writes and discovery: `POST`, `PUT` and
//! `DELETE /fhir/Practitioner`, a name search, and `GET /fhir/metadata`.
//! The read paths' audit and masking are covered by `fhir_read_audit.rs`
//! and `fhir_sec_g3.rs`. DB-gated.

mod common;

use axa_test::request;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;
use worker_service::app::App;
use worker_service::config::Config;

fn practitioner(family: &str, given: &str) -> Value {
    json!({
        "resourceType": "Practitioner",
        "name": [{ "use": "official", "family": family, "given": [given] }],
        "gender": "female",
        "birthDate": "1980-04-02"
    })
}

fn boot_dirs() {
    let config = Config::from_env().expect("test config");
    std::fs::create_dir_all(&config.search.index_path).unwrap();
}

/// The capability statement says what this server is: FHIR R5, JSON, and
/// the one resource it serves, with the interactions mounted below.
#[tokio::test]
#[serial]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn metadata_describes_the_practitioner_endpoint() {
    boot_dirs();
    request::<App, _, _>(|server, _ctx| async move {
        let response = server.get("/fhir/metadata").await;
        response.assert_status_ok();
        assert_eq!(
            response.header("content-type").to_str().unwrap(),
            "application/fhir+json"
        );
        let statement: Value = response.json();
        assert_eq!(statement["resourceType"], "CapabilityStatement");
        assert_eq!(statement["fhirVersion"], "5.0.0");
        let text = statement.to_string();
        assert!(text.contains("Practitioner"), "{text}");
    })
    .await;
}

/// Create, read back, update, find by name, delete, and see it gone.
#[tokio::test]
#[serial]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn practitioner_create_update_delete_round_trip() {
    boot_dirs();
    request::<App, _, _>(|server, _ctx| async move {
        // Short and letters only: the name tokenizer splits on digits and
        // drops long tokens, so a long hex suffix would never match.
        let family: String = std::iter::once('F')
            .chain(
                Uuid::new_v4()
                    .simple()
                    .to_string()
                    .chars()
                    .take(10)
                    .map(|c| char::from(b'a' + (c.to_digit(16).unwrap() as u8))),
            )
            .collect();
        let created = server
            .post("/fhir/Practitioner")
            .json(&practitioner(&family, "Ada"))
            .await;
        assert_eq!(created.status_code(), 201, "{}", created.text());
        let body: Value = created.json();
        assert_eq!(body["resourceType"], "Practitioner");
        let id = body["id"].as_str().expect("an id").to_string();

        let read = server.get(&format!("/fhir/Practitioner/{id}")).await;
        read.assert_status_ok();
        assert_eq!(read.json::<Value>()["name"][0]["family"], family);

        let mut changed = practitioner(&family, "Augusta");
        changed["id"] = json!(id);
        let updated = server
            .put(&format!("/fhir/Practitioner/{id}"))
            .json(&changed)
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());
        assert_eq!(updated.json::<Value>()["name"][0]["given"][0], "Augusta");

        let found = server
            .get(&format!("/fhir/Practitioner?name={family}"))
            .await;
        found.assert_status_ok();
        let bundle: Value = found.json();
        assert_eq!(bundle["resourceType"], "Bundle");
        assert_eq!(bundle["type"], "searchset");
        assert!(bundle.to_string().contains(&id), "{bundle}");

        let deleted = server.delete(&format!("/fhir/Practitioner/{id}")).await;
        assert_eq!(deleted.status_code(), 204, "{}", deleted.text());
        let gone = server.get(&format!("/fhir/Practitioner/{id}")).await;
        assert_eq!(gone.status_code(), 404);
        assert_eq!(gone.json::<Value>()["resourceType"], "OperationOutcome");
    })
    .await;
}

/// Malformed input is an `OperationOutcome`, never a bare error page.
#[tokio::test]
#[serial]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn bad_practitioner_input_is_an_operation_outcome() {
    boot_dirs();
    request::<App, _, _>(|server, _ctx| async move {
        let no_name = json!({ "resourceType": "Practitioner", "gender": "female" });
        let r = server.post("/fhir/Practitioner").json(&no_name).await;
        assert!(r.status_code().is_client_error(), "{}", r.text());
        assert_eq!(r.json::<Value>()["resourceType"], "OperationOutcome");

        let bad_id = json!({
            "resourceType": "Practitioner",
            "id": "not-a-uuid",
            "name": [{ "family": "X", "given": ["Y"] }]
        });
        let r = server.post("/fhir/Practitioner").json(&bad_id).await;
        assert!(r.status_code().is_client_error(), "{}", r.text());
        assert_eq!(r.json::<Value>()["resourceType"], "OperationOutcome");

        // The native validation applies: an over-long family name (SEC-M1
        // caps it at 1024) is a 422 with one issue per problem.
        let long = practitioner(&"x".repeat(5000), "Ada");
        let r = server.post("/fhir/Practitioner").json(&long).await;
        assert_eq!(r.status_code(), 422, "{}", r.text());
        assert_eq!(r.json::<Value>()["issue"][0]["code"], "processing");

        let missing = Uuid::new_v4();
        let r = server
            .put(&format!("/fhir/Practitioner/{missing}"))
            .json(&practitioner("Nobody", "None"))
            .await;
        assert_eq!(r.status_code(), 404, "{}", r.text());
        let r = server
            .delete(&format!("/fhir/Practitioner/{missing}"))
            .await;
        assert!(r.status_code().is_client_error(), "{}", r.text());
    })
    .await;
}
