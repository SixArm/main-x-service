//! The bulk import/export endpoints over HTTP. The bulk pipeline, codecs
//! and memory bound are tested in `src/bulk` and `bulk_streaming_memory.rs`;
//! this pins the handlers: a JSONL import creates, an export carries what it
//! created, a missing file and an unknown masking profile or format are
//! `400`, a dry run writes nothing, a repeated `Idempotency-Key` is the
//! same job, and an unknown or wrong-kind job id is `404`.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test bulk_http -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use person_service::bulk::jsonl;
use person_service::models::{Gender, HumanName, Person};
use serde_json::{Value, json};
use tower::ServiceExt;

const BOUNDARY: &str = "bulk-http-test-boundary";

fn multipart(fields: &[(&str, &str)], file: Option<&[u8]>) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    if let Some(bytes) = file {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"import.jsonl\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
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

async fn import(app: &axum::Router, body: Vec<u8>, key: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::post("/api/persons/import").header(
        "content-type",
        format!("multipart/form-data; boundary={BOUNDARY}"),
    );
    if let Some(k) = key {
        builder = builder.header("idempotency-key", k);
    }
    send(app, builder.body(Body::from(body)).unwrap()).await
}

async fn export(app: &axum::Router, body: &Value, key: Option<&str>) -> (StatusCode, Value) {
    let mut builder =
        Request::post("/api/persons/export").header("content-type", "application/json");
    if let Some(k) = key {
        builder = builder.header("idempotency-key", k);
    }
    send(
        app,
        builder
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap(),
    )
    .await
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    send(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

fn letters() -> String {
    uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(10)
        .map(|c| char::from(b'a' + u8::try_from(c.to_digit(16).unwrap()).unwrap()))
        .collect()
}

fn person(family: &str) -> Person {
    Person::new(
        HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec!["Bulk".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    )
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn import_export_refusals_dry_run_and_idempotency() {
    let app = common::create_test_router().await;
    let family = format!("Bulkhttp{}", letters());
    let file = jsonl::encode(&[person(&family)]).unwrap();

    // Import, then the status of the job it made.
    let (status, accepted) =
        import(&app, multipart(&[("format", "jsonl")], Some(&file)), None).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{accepted}");
    let job_id = accepted["job_id"].as_str().unwrap().to_string();
    let (status, job) = get(&app, &format!("/api/persons/import/{job_id}")).await;
    assert_eq!(status, StatusCode::OK, "{job}");
    assert_eq!(job["kind"], "import", "{job}");
    assert_eq!(
        job["status"], "completed",
        "jobs run in the foreground under test: {job}"
    );
    assert_eq!(job["rows_created"], 1, "{job}");

    // An export job is accepted and reports itself.
    let (status, accepted) = export(
        &app,
        &json!({ "format": "jsonl", "q": family, "masking_profile": "full" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{accepted}");
    let export_id = accepted["job_id"].as_str().unwrap().to_string();
    let (status, job) = get(&app, &format!("/api/persons/export/{export_id}")).await;
    assert_eq!(status, StatusCode::OK, "{job}");
    assert_eq!(job["kind"], "export");
    assert_eq!(job["status"], "completed", "{job}");
    let url = job["download_url"].as_str().expect("download_url");
    let written =
        std::fs::read_to_string(url.strip_prefix("file://").expect("a file:// reference")).unwrap();
    assert!(
        written.contains(&family),
        "the export carries the imported person"
    );

    // Refusals.
    let (status, body) = import(&app, multipart(&[("format", "jsonl")], None), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.to_string().contains("missing_file"), "{body}");
    let (status, body) = import(&app, multipart(&[("format", "xml")], Some(&file)), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = export(
        &app,
        &json!({ "format": "jsonl", "masking_profile": "partial" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body.to_string().contains("unsupported_masking_profile"),
        "{body}"
    );

    // A dry run is accepted and is its own job.
    let (status, dry) = import(
        &app,
        multipart(&[("format", "jsonl"), ("dry_run", "true")], Some(&file)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{dry}");

    // The same Idempotency-Key is the same job, for both kinds.
    let key = format!("idem-{}", uuid::Uuid::new_v4());
    let body = || multipart(&[("format", "jsonl"), ("dry_run", "true")], Some(&file));
    let (_, first) = import(&app, body(), Some(&key)).await;
    let (_, second) = import(&app, body(), Some(&key)).await;
    assert_eq!(
        first["job_id"], second["job_id"],
        "a retried import is the same job"
    );
    let (_, e1) = export(&app, &json!({ "format": "jsonl" }), Some(&key)).await;
    let (_, e2) = export(&app, &json!({ "format": "jsonl" }), Some(&key)).await;
    assert_eq!(
        e1["job_id"], e2["job_id"],
        "a retried export is the same job"
    );

    let (status, listed) = get(&app, "/api/persons/bulk-jobs").await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert!(listed.to_string().contains(&job_id), "{listed}");

    let unknown = uuid::Uuid::new_v4();
    for uri in [
        format!("/api/persons/import/{unknown}"),
        format!("/api/persons/export/{unknown}"),
        format!("/api/persons/export/{job_id}"),
    ] {
        assert_eq!(get(&app, &uri).await.0, StatusCode::NOT_FOUND, "{uri}");
    }
}
