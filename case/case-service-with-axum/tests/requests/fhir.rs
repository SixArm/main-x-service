//! FHIR R5 `Task`: the capability statement, a create → read → update →
//! delete round trip, a search, and the `OperationOutcome` each malformed
//! or unknown request gets. Enforcement is off; the record-level decision
//! on writes is `tests/fhir_write_abac.rs`, masking `tests/masking.rs`.

use axa_test::prelude::*;
use case_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

fn task(description: &str) -> Value {
    json!({ "resourceType": "Task", "status": "requested", "intent": "order", "description": description })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn fhir_metadata_crud_search_and_errors() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let json_of = |text: String| serde_json::from_str::<Value>(&text).unwrap();

        let meta = request.get("/fhir/metadata").await;
        assert_eq!(meta.status_code(), 200);
        let statement = json_of(meta.text());
        assert_eq!(statement["resourceType"], "CapabilityStatement");
        assert_eq!(statement["fhirVersion"], "5.0.0");
        assert_eq!(statement["rest"][0]["resource"][0]["type"], "Task");

        let word: String = uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .map(|c| char::from(b'a' + u8::try_from(c.to_digit(16).unwrap()).unwrap()))
            .collect();
        let description = format!("Roundtrip {word}");
        let created = request.post("/fhir/Task").json(&task(&description)).await;
        assert_eq!(created.status_code(), 201, "{}", created.text());
        let id = json_of(created.text())["id"].as_str().unwrap().to_string();

        let read = request.get(&format!("/fhir/Task/{id}")).await;
        assert_eq!(read.status_code(), 200);
        assert_eq!(json_of(read.text())["description"], description);

        let found = request.get(&format!("/fhir/Task?description={word}")).await;
        assert_eq!(found.status_code(), 200, "{}", found.text());
        assert_eq!(json_of(found.text())["resourceType"], "Bundle");

        let mut changed = task("Renamed task");
        changed["id"] = json!(id);
        let updated = request
            .put(&format!("/fhir/Task/{id}"))
            .json(&changed)
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());
        assert_eq!(json_of(updated.text())["description"], "Renamed task");

        let deleted = request.delete(&format!("/fhir/Task/{id}")).await;
        assert!(deleted.status_code().is_success(), "{}", deleted.text());
        let gone = request.get(&format!("/fhir/Task/{id}")).await;
        assert_eq!(gone.status_code(), 404);
        assert_eq!(json_of(gone.text())["issue"][0]["code"], "not-found");

        for (method, uri) in [
            ("POST", "/fhir/Task".to_string()),
            ("PUT", format!("/fhir/Task/{id}")),
        ] {
            let builder = if method == "POST" {
                request.post(&uri)
            } else {
                request.put(&uri)
            };
            let bad = builder
                .bytes("not json".into())
                .content_type("application/fhir+json")
                .await;
            assert_eq!(bad.status_code(), 400, "{method}: {}", bad.text());
            assert_eq!(json_of(bad.text())["issue"][0]["code"], "structure");
        }
        let no_description = request
            .post("/fhir/Task")
            .json(&json!({ "resourceType": "Task", "status": "requested", "intent": "order" }))
            .await;
        assert_eq!(
            no_description.status_code(),
            400,
            "{}",
            no_description.text()
        );
        assert_eq!(
            json_of(no_description.text())["issue"][0]["code"],
            "invalid"
        );

        let missing = uuid::Uuid::new_v4();
        assert_eq!(
            request
                .get(&format!("/fhir/Task/{missing}"))
                .await
                .status_code(),
            404
        );
        let r = request
            .put(&format!("/fhir/Task/{missing}"))
            .json(&task("Nobody"))
            .await;
        assert_eq!(r.status_code(), 404, "{}", r.text());
        assert_eq!(
            request
                .delete(&format!("/fhir/Task/{missing}"))
                .await
                .status_code(),
            404
        );
    })
    .await;
}
