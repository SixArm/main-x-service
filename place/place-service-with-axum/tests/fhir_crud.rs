//! FHIR R5 `Location`: the capability statement, a create → read → search
//! → update → delete round trip, and the `OperationOutcome` each
//! malformed, invalid or unknown request gets. The crate had no FHIR
//! request tests before this.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test fhir_crud -- --ignored`.

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::app::App;
use place_service::config::Config;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn app() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    App::boot_test()
        .await
        .expect("boot the app: set DB_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
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

fn location(name: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "resourceType": "Location",
        "status": "active",
        "name": name,
        "address": { "city": "Cardiff", "postalCode": "CF10 1AA", "country": "GB" }
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
async fn fhir_location_metadata_crud_and_errors() {
    let app = app().await;

    let (status, meta) = call(&app, "GET", "/fhir/metadata", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(meta["resourceType"], "CapabilityStatement");
    assert_eq!(meta["fhirVersion"], "5.0.0");
    assert!(meta.to_string().contains("Location"), "{meta}");

    let name = format!("Roundtrip {}", word());
    let (status, created) = call(&app, "POST", "/fhir/Location", Some(location(&name))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["resourceType"], "Location");
    let id = created["id"].as_str().unwrap().to_string();

    let (status, read) = call(&app, "GET", &format!("/fhir/Location/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["name"], name);

    let (status, bundle) = call(&app, "GET", &format!("/fhir/Location?_id={id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{bundle}");
    assert_eq!(bundle["resourceType"], "Bundle");
    assert!(bundle.to_string().contains(&id), "{bundle}");

    let (status, updated) = call(
        &app,
        "PUT",
        &format!("/fhir/Location/{id}"),
        Some(location("Renamed place")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], "Renamed place");

    let (status, _) = call(&app, "DELETE", &format!("/fhir/Location/{id}"), None).await;
    assert!(status.is_success(), "{status}");
    let (status, gone) = call(&app, "GET", &format!("/fhir/Location/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(gone["resourceType"], "OperationOutcome");

    for (method, uri) in [
        ("POST", "/fhir/Location".to_string()),
        ("PUT", format!("/fhir/Location/{id}")),
    ] {
        let (status, outcome) = call(&app, method, &uri, Some(b"not json".to_vec())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method}: {outcome}");
        assert_eq!(outcome["resourceType"], "OperationOutcome");
    }
    let unnamed =
        serde_json::to_vec(&json!({ "resourceType": "Location", "status": "active" })).unwrap();
    let (status, outcome) = call(&app, "POST", "/fhir/Location", Some(unnamed)).await;
    assert!(status.is_client_error(), "{status} {outcome}");
    assert_eq!(outcome["resourceType"], "OperationOutcome");

    // The native validation applies on FHIR writes: an over-long name
    // (SEC-M1 caps text at 1024) is a `422`, not a stored record.
    let (status, outcome) = call(
        &app,
        "POST",
        "/fhir/Location",
        Some(location(&"x".repeat(5000))),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{outcome}");
    let (status, _) = call(
        &app,
        "PUT",
        &format!("/fhir/Location/{id}"),
        Some(location(&"x".repeat(5000))),
    )
    .await;
    assert!(status.is_client_error(), "update: {status}");

    let missing = uuid::Uuid::new_v4();
    for (method, body) in [
        ("GET", None),
        ("PUT", Some(location("Nobody"))),
        ("DELETE", None),
    ] {
        let (status, outcome) =
            call(&app, method, &format!("/fhir/Location/{missing}"), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}: {outcome}");
    }
}
