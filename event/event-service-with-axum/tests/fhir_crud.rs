//! FHIR R5 `Appointment`: the capability statement, a create → read → search
//! → update → delete round trip, and the `OperationOutcome` each
//! malformed, invalid or unknown request gets. The crate had no FHIR
//! request tests before this.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test fhir_crud -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn app() -> axum::Router {
    common::create_test_router().await
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Vec<u8>>,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/fhir+json")
        .body(body.map_or_else(Body::empty, Body::from))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn appointment(name: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "resourceType": "Appointment",
        "status": "booked",
        "description": name,
        "start": "2026-03-01T09:00:00Z",
        "end": "2026-03-01T10:00:00Z"
    }))
    .unwrap()
}

fn word() -> String {
    uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .map(|c| char::from(b'a' + u8::try_from(c.to_digit(16).unwrap()).unwrap()))
        .collect()
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn fhir_appointment_metadata_crud_and_errors() {
    let app = app().await;

    let (status, meta) = call(&app, "GET", "/fhir/metadata", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(meta["resourceType"], "CapabilityStatement");
    assert_eq!(meta["fhirVersion"], "5.0.0");
    assert!(meta.to_string().contains("Appointment"), "{meta}");

    let name = format!("Roundtrip {}", word());
    let (status, created) = call(&app, "POST", "/fhir/Appointment", Some(appointment(&name))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["resourceType"], "Appointment");
    let id = created["id"].as_str().unwrap().to_string();

    let (status, read) = call(&app, "GET", &format!("/fhir/Appointment/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["description"], name);

    let (status, bundle) = call(&app, "GET", &format!("/fhir/Appointment?_id={id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{bundle}");
    assert_eq!(bundle["resourceType"], "Bundle");
    assert!(bundle.to_string().contains(&id), "{bundle}");

    let (status, updated) = call(
        &app,
        "PUT",
        &format!("/fhir/Appointment/{id}"),
        Some(appointment("Renamed appointment")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["description"], "Renamed appointment");

    let (status, _) = call(&app, "DELETE", &format!("/fhir/Appointment/{id}"), None).await;
    assert!(status.is_success(), "{status}");
    let (status, gone) = call(&app, "GET", &format!("/fhir/Appointment/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(gone["resourceType"], "OperationOutcome");

    for (method, uri) in [
        ("POST", "/fhir/Appointment".to_string()),
        ("PUT", format!("/fhir/Appointment/{id}")),
    ] {
        let (status, outcome) = call(&app, method, &uri, Some(b"not json".to_vec())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method}: {outcome}");
        assert_eq!(outcome["resourceType"], "OperationOutcome");
    }
    let undescribed =
        serde_json::to_vec(&json!({ "resourceType": "Appointment", "status": "active" })).unwrap();
    let (status, outcome) = call(&app, "POST", "/fhir/Appointment", Some(undescribed)).await;
    assert!(status.is_client_error(), "{status} {outcome}");
    assert_eq!(outcome["resourceType"], "OperationOutcome");

    // The native validation applies on FHIR writes: an over-long name
    // (SEC-M1 caps text at 1024) is a `422`, not a stored record.
    let (status, outcome) = call(
        &app,
        "POST",
        "/fhir/Appointment",
        Some(appointment(&"x".repeat(5000))),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{outcome}");
    let (status, _) = call(
        &app,
        "PUT",
        &format!("/fhir/Appointment/{id}"),
        Some(appointment(&"x".repeat(5000))),
    )
    .await;
    assert!(status.is_client_error(), "update: {status}");

    let missing = uuid::Uuid::new_v4();
    for (method, body) in [
        ("GET", None),
        ("PUT", Some(appointment("Nobody"))),
        ("DELETE", None),
    ] {
        let (status, outcome) =
            call(&app, method, &format!("/fhir/Appointment/{missing}"), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}: {outcome}");
    }
}
