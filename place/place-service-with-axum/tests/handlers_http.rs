//! HTTP tests for REST handlers the other suites do not reach: merge,
//! delete, the review-queue decision, the per-place and recent audit
//! listings, the masked view and export, the integrity verifiers, and the
//! `404` each single-place endpoint gives an unknown id.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test handlers_http -- --ignored`.

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::app::App;
use place_service::config::Config;
use place_service::db::create_connection;
use place_service::db::review_queue::{NewReviewItem, upsert};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

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

/// A distinct letters-only name, so no two test places look alike to the
/// create-time duplicate check.
fn name(prefix: &str) -> String {
    let letters: String = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(12)
        .map(|c| char::from(b'a' + u8::try_from(c.to_digit(16).unwrap()).unwrap()))
        .collect();
    format!("{prefix} {letters}")
}

async fn create(app: &axum::Router, prefix: &str) -> Uuid {
    let (status, body) = call(
        app,
        "POST",
        "/api/places",
        Some(&json!({ "name": name(prefix) })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn merge_retires_the_duplicate_and_refuses_bad_pairs() {
    let app = app().await;
    let main = create(&app, "Mergemain").await;
    let duplicate = create(&app, "Mergedup").await;

    let (status, merged) = call(
        &app,
        "POST",
        "/api/places/merge",
        Some(&json!({ "main_place_id": main, "duplicate_place_id": duplicate, "merge_reason": "test" })),
    )
    .await;
    assert!(status.is_success(), "{status} {merged}");
    assert_eq!(
        call(&app, "GET", &format!("/api/places/{main}"), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "GET", &format!("/api/places/{duplicate}"), None)
            .await
            .0,
        StatusCode::NOT_FOUND,
        "the duplicate is no longer served"
    );

    for (why, a, b, want) in [
        ("self-merge", main, main, StatusCode::UNPROCESSABLE_ENTITY),
        ("unknown main", Uuid::new_v4(), main, StatusCode::NOT_FOUND),
        (
            "unknown duplicate",
            main,
            Uuid::new_v4(),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (status, body) = call(
            &app,
            "POST",
            "/api/places/merge",
            Some(&json!({ "main_place_id": a, "duplicate_place_id": b })),
        )
        .await;
        assert_eq!(status, want, "{why}: {body}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn delete_audit_masked_and_export() {
    let app = app().await;
    let id = create(&app, "Lifecycle").await;

    let (status, masked) = call(&app, "GET", &format!("/api/places/{id}/masked"), None).await;
    assert_eq!(status, StatusCode::OK, "{masked}");
    let (status, export) = call(&app, "GET", &format!("/api/places/{id}/export"), None).await;
    assert_eq!(status, StatusCode::OK, "{export}");
    let (status, audit) = call(&app, "GET", &format!("/api/places/{id}/audit"), None).await;
    assert_eq!(status, StatusCode::OK, "{audit}");
    assert!(
        audit.to_string().contains("created") || audit.to_string().contains("create"),
        "{audit}"
    );
    let (status, _) = call(&app, "GET", "/api/audit/recent?limit=5", None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(&app, "DELETE", &format!("/api/places/{id}"), None).await;
    assert!(status.is_success(), "{status}");
    assert_eq!(
        call(&app, "GET", &format!("/api/places/{id}"), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "DELETE", &format!("/api/places/{id}"), None)
            .await
            .0,
        StatusCode::NOT_FOUND,
        "a deleted place is not deleted twice"
    );

    for uri in ["/api/records/verify?limit=50", "/api/audit/verify?limit=50"] {
        let (status, report) = call(&app, "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {report}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn review_queue_decides_once() {
    let app = app().await;
    let config = Config::from_env().unwrap();
    let db = create_connection(&config.database).await.unwrap();
    let rows = upsert(
        &db,
        &[NewReviewItem {
            record_id_a: Uuid::new_v4(),
            record_id_b: Uuid::new_v4(),
            match_score: 0.81,
            match_quality: "probable".to_string(),
            detection_method: "batch_deduplication".to_string(),
            score_breakdown: None,
            evidence: None,
            status: "pending".to_string(),
        }],
    )
    .await
    .unwrap();
    let id = rows[0].id;

    let (status, listed) = call(&app, "GET", "/api/places/review-queue", None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let uri = format!("/api/places/review-queue/{id}/decision");
    let (status, decided) = call(&app, "POST", &uri, Some(&json!({ "status": "rejected" }))).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    let (status, _) = call(&app, "POST", &uri, Some(&json!({ "status": "confirmed" }))).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "first writer wins"
    );
    let unknown = format!("/api/places/review-queue/{}/decision", Uuid::new_v4());
    let (status, _) = call(
        &app,
        "POST",
        &unknown,
        Some(&json!({ "status": "confirmed" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn unknown_places_are_404_everywhere() {
    let app = app().await;
    let id = Uuid::new_v4();
    let body = json!({ "name": "Nowhere at all" });
    for (method, uri, payload) in [
        ("GET", format!("/api/places/{id}"), None),
        ("PUT", format!("/api/places/{id}"), Some(&body)),
        ("DELETE", format!("/api/places/{id}"), None),
        ("GET", format!("/api/places/{id}/masked"), None),
        ("GET", format!("/api/places/{id}/export"), None),
        ("GET", format!("/api/places/{id}/descendants"), None),
    ] {
        let (status, got) = call(&app, method, &uri, payload).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}: {got}");
    }
}
