//! HTTP tests for REST handlers the other suites do not reach: search,
//! match, update, merge, the per-event, recent and per-user audit
//! listings, the masked view, the integrity verifiers, batch
//! deduplication, and the `404` each single-event endpoint gives an
//! unknown id.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test handlers_http -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(b) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(b).unwrap())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn word() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(10)
        .map(|c| char::from(b'a' + u8::try_from(c.to_digit(16).unwrap()).unwrap()))
        .collect()
}

fn event(name: &str, start: &str) -> Value {
    json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": name,
        "start_date": start,
        "end_date": start,
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "conference",
        "all_day": false,
    })
}

async fn create(app: &axum::Router, body: &Value) -> Uuid {
    let (status, created) = call(app, "POST", "/api/events", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn search_match_update_and_audit() {
    let app = common::create_test_router().await;
    let token = word();
    let id = create(
        &app,
        &event(&format!("Symposium {token}"), "2026-07-01T09:00:00Z"),
    )
    .await;

    let (status, found) = call(&app, "GET", &format!("/api/events/search?q={token}"), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    assert!(
        found.to_string().contains(&id.to_string()),
        "search finds it: {found}"
    );

    // The probe event is flattened into the body beside the options.
    let mut probe = event(&format!("Symposium {token}"), "2026-07-01T09:00:00Z");
    probe["threshold"] = json!(0.5);
    probe["limit"] = json!(5);
    let (status, matched) = call(&app, "POST", "/api/events/match", Some(&probe)).await;
    assert_eq!(status, StatusCode::OK, "{matched}");
    assert!(
        matched.to_string().contains(&id.to_string()),
        "match scores it: {matched}"
    );

    let mut changed = event(
        &format!("Renamed symposium {token}"),
        "2026-07-01T09:00:00Z",
    );
    changed["id"] = json!(id);
    let (status, updated) = call(&app, "PUT", &format!("/api/events/{id}"), Some(&changed)).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], format!("Renamed symposium {token}"));

    let (status, masked) = call(&app, "GET", &format!("/api/events/{id}/masked"), None).await;
    assert_eq!(status, StatusCode::OK, "{masked}");
    let (status, audit) = call(&app, "GET", &format!("/api/events/{id}/audit"), None).await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert!(
        audit.as_array().is_some_and(|a| a.len() >= 2),
        "create and update are audited: {audit}"
    );
    for uri in [
        "/api/audit/recent?limit=5",
        "/api/audit/user?user_id=nobody&limit=5",
        "/api/records/verify?limit=50",
        "/api/audit/verify?limit=50",
    ] {
        let (status, body) = call(&app, "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    }
    let (status, dedup) = call(&app, "POST", "/api/events/deduplicate", Some(&json!({}))).await;
    assert!(status.is_success(), "{status} {dedup}");
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn merge_retires_the_duplicate_and_refuses_bad_pairs() {
    let app = common::create_test_router().await;
    let main = create(
        &app,
        &event(&format!("Mainevent {}", word()), "2026-08-01T09:00:00Z"),
    )
    .await;
    let duplicate = create(
        &app,
        &event(&format!("Dupevent {}", word()), "2026-09-01T09:00:00Z"),
    )
    .await;

    let (status, merged) = call(
        &app,
        "POST",
        "/api/events/merge",
        Some(&json!({ "main_event_id": main, "duplicate_event_id": duplicate, "merge_reason": "test" })),
    )
    .await;
    assert!(status.is_success(), "{status} {merged}");
    assert_eq!(
        call(&app, "GET", &format!("/api/events/{main}"), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "GET", &format!("/api/events/{duplicate}"), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );

    for (why, a, b) in [
        ("self-merge", main, main),
        ("unknown main", Uuid::new_v4(), main),
        ("unknown duplicate", main, Uuid::new_v4()),
    ] {
        let (status, body) = call(
            &app,
            "POST",
            "/api/events/merge",
            Some(&json!({ "main_event_id": a, "duplicate_event_id": b })),
        )
        .await;
        assert!(status.is_client_error(), "{why}: {status} {body}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn unknown_events_are_404_everywhere() {
    let app = common::create_test_router().await;
    let id = Uuid::new_v4();
    let body = event("Nothing here", "2026-01-01T09:00:00Z");
    for (method, uri, payload) in [
        ("GET", format!("/api/events/{id}"), None),
        ("PUT", format!("/api/events/{id}"), Some(&body)),
        ("DELETE", format!("/api/events/{id}"), None),
        ("GET", format!("/api/events/{id}/masked"), None),
        ("GET", format!("/api/events/{id}/export"), None),
    ] {
        let (status, got) = call(&app, method, &uri, payload).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}: {got}");
    }
}
