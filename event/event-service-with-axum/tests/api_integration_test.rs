#![warn(clippy::pedantic)]

//! REST API integration tests. Requires a running `PostgreSQL`
//! reachable via `DB_URL`.
//!
//! All tests are marked `#[ignore = "requires a running PostgreSQL via DB_URL"]` so they are skipped by a bare
//! `cargo test`; run them explicitly with
//! `DB_URL=… cargo test --test api_integration_test -- --ignored`.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use event_service::models::Event;
use serde_json::json;
use tower::ServiceExt;

/// `GET /api/health` returns 200 and names the service.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn health_check_returns_healthy() {
    let app = common::create_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("healthy"));
    assert!(s.contains("event-service"));
}

/// Creating an event mints a fresh id and the record reads back
/// identically via `GET /api/events/{id}`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn create_event_round_trip() {
    let app = common::create_test_router().await;
    let title = common::unique_event_name("CreateRoundTrip");

    let payload = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": title,
        "start_date": "2026-06-01T18:00:00Z",
        "end_date": "2026-06-01T20:00:00Z",
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "conference",
        "all_day": false,
    });

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Read the body *before* asserting the status, so a failure reports
    // why the server refused rather than just "422 != 201".
    let status = create.status();
    let body = axum::body::to_bytes(create.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create failed: {}",
        String::from_utf8_lossy(&body)
    );

    let event: Event = serde_json::from_slice(&body).unwrap();
    assert_eq!(event.name, title);
    assert_ne!(event.id.to_string(), "00000000-0000-0000-0000-000000000000");

    let get = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/events/{}", event.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body = axum::body::to_bytes(get.into_body(), usize::MAX)
        .await
        .unwrap();
    let fetched: Event = serde_json::from_slice(&body).unwrap();
    assert_eq!(fetched.id, event.id);
    assert_eq!(fetched.name, title);
}

/// The FHIR R5 `Appointment` surface is live (the same handlers the axa
/// production router serves): `GET /fhir/metadata` returns a `200`
/// `CapabilityStatement`, and an unknown `GET /fhir/Appointment/{id}`
/// returns `404` with an `OperationOutcome` body — no more `501` stub.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn fhir_appointment_surface_is_live() {
    let app = common::create_test_router().await;

    // Capability discovery is a real CapabilityStatement.
    let meta = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fhir/metadata")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(meta.status(), StatusCode::OK);
    let meta_body = axum::body::to_bytes(meta.into_body(), usize::MAX)
        .await
        .unwrap();
    let meta_s = String::from_utf8(meta_body.to_vec()).unwrap();
    assert!(
        meta_s.contains("CapabilityStatement"),
        "expected CapabilityStatement, got {meta_s}"
    );

    // An unknown Appointment id is a real 404 OperationOutcome (not 501).
    let response = app
        .oneshot(
            Request::builder()
                .uri("/fhir/Appointment/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        s.contains("OperationOutcome"),
        "expected OperationOutcome, got {s}"
    );
}

/// An empty `name` is rejected with `422 Unprocessable Entity`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn validation_rejects_missing_name() {
    let app = common::create_test_router().await;
    let payload = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": "",
        "start_date": "2026-06-01T18:00:00Z",
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "generic",
        "all_day": false,
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// A hand-built body carrying only the two fields the server does NOT
/// own — `name` and `start_date` — must succeed. Before this fix every
/// other field (`id`, `active`, `event_status`, `event_attendance_mode`,
/// `event_type`, and every `Option<T>` property) had no `#[serde]`
/// default, so the JSON extractor refused a real client's minimal body
/// with a cascading `422 missing field …` before the handler — including
/// its own validation and its own nil-`id` mint — ever ran. Same defect
/// class as the person/thing/place/worker services' own
/// QA-SERVER-FIELDS fix, just never landed here.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn create_event_from_a_minimal_hand_written_body_succeeds() {
    let app = common::create_test_router().await;
    let title = common::unique_event_name("MinimalBody");

    let payload = json!({
        "name": title,
        "start_date": "2026-06-01T18:00:00Z",
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create failed: {}",
        String::from_utf8_lossy(&body)
    );

    let event: Event = serde_json::from_slice(&body).unwrap();
    assert_eq!(event.name, title);

    // The id is server-minted, not the nil sentinel the omitted field
    // defaulted to.
    assert_ne!(event.id, uuid::Uuid::nil());

    // Every field the server owns or that is genuinely optional reads
    // back its documented default rather than having failed the
    // extractor.
    assert!(
        event.active,
        "a newly created event should default to active"
    );
    assert_eq!(
        event.event_status,
        event_service::models::EventStatus::Scheduled
    );
    assert_eq!(
        event.event_attendance_mode,
        event_service::models::EventAttendanceMode::Offline
    );
    assert_eq!(event.event_type, event_service::models::EventType::Generic);
    assert!(event.description.is_none());
    assert!(event.end_date.is_none());
    assert!(event.super_event.is_none());
}

/// Send `request` to `app` and return the status and JSON body.
async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    )
}

/// Whether `/api/records/verify` names the record `id` as mismatched.
async fn is_mismatched(app: &axum::Router, id: &str) -> bool {
    let (status, report) = send(
        app,
        Request::builder()
            .uri("/api/records/verify?limit=1000")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "verify failed: {report}");
    report["mismatched"]
        .as_array()
        .expect("a mismatched list")
        .iter()
        .any(|m| m["id"] == id)
}

/// A record verifies the moment it is written, and again after an update
/// (tasks.md EVT-V1). Its digests used to cover the event the caller sent,
/// whose timestamps the stored row replaces, so every fresh record failed
/// verification and a real edit could not be told from a new row. Child
/// rows and a numeric price are included, since the database normalises
/// both.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn a_written_record_verifies() {
    let app = common::create_test_router().await;
    let title = common::unique_event_name("VerifiesWhenWritten");
    let payload = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": title,
        "start_date": "2026-06-01T18:00:00.123456789Z",
        "end_date": "2026-06-01T20:00:00Z",
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "conference",
        "all_day": false,
        "keywords": ["integrity", "audit"],
        "offers": [{ "name": "Standard", "price": "12.50", "price_currency": "GBP" }],
        "created_at": "2001-01-01T00:00:00.987654321Z",
        "updated_at": "2001-01-01T00:00:00.987654321Z",
    });
    let (status, created) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/events")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&payload).unwrap()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {created}");
    let id = created["id"].as_str().expect("an id").to_string();
    assert!(
        !is_mismatched(&app, &id).await,
        "a fresh record must verify"
    );

    let mut changed = created.clone();
    changed["description"] = json!("Edited after creation");
    let (status, updated) = send(
        &app,
        Request::builder()
            .method("PUT")
            .uri(format!("/api/events/{id}"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&changed).unwrap()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {updated}");
    assert!(
        !is_mismatched(&app, &id).await,
        "an updated record must verify"
    );

    // And verification still catches a change made behind the service's
    // back, so the two checks above cannot pass by verifying nothing.
    let pool = sqlx::PgPool::connect(&std::env::var("DB_URL").expect("DB_URL"))
        .await
        .expect("connect for the tamper step");
    sqlx::query("UPDATE events SET name = name || ' (edited in SQL)' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("tamper with the row");
    assert!(
        is_mismatched(&app, &id).await,
        "a row edited in SQL must be flagged"
    );
}
