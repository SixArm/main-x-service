//! HTTP tests for the assessment endpoints a worker's psychometric and
//! selection results are read, changed and removed through:
//! `GET` / `PUT` / `DELETE /api/workers/{id}/assessments/{assessment_id}`
//! and `GET /api/workers/{id}/assessment-profile`. Create and list are
//! covered in `api_integration_test.rs`.
//!
//! The property that matters most here is that an assessment is reached
//! only through the worker it belongs to: naming it under another worker's
//! id is a `404`, so a caller allowed one worker cannot read, change or
//! delete another's results by guessing an id. DB-gated.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

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
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn worker(app: &axum::Router, prefix: &str) -> uuid::Uuid {
    let body = json!({
        "name": { "family": format!("{prefix}{}", uuid::Uuid::new_v4().simple()), "given": ["A"] },
        "gender": "unknown"
    });
    let (status, created) = call(app, "POST", "/api/workers", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created["id"].as_str().unwrap().parse().unwrap()
}

async fn assessment(app: &axum::Router, worker: uuid::Uuid, status: &str) -> uuid::Uuid {
    let body = json!({
        "category": "aptitude", "instrument": "Numerical", "status": status,
        "administered_on": "2026-01-10",
        "results": [{ "scale": "numerical_reasoning", "raw_score": 40.0, "max_score": 50.0 }]
    });
    let (code, created) = call(
        app,
        "POST",
        &format!("/api/workers/{worker}/assessments"),
        Some(&body),
    )
    .await;
    assert!(code.is_success(), "{code} {created}");
    created["id"].as_str().unwrap().parse().unwrap()
}

/// How many assessments the profile counts for `category`.
fn recorded(profile: &Value, category: &str) -> u64 {
    profile["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["category"] == category)
        .and_then(|c| c["recorded"].as_u64())
        .unwrap_or_else(|| panic!("no {category} in {profile}"))
}

fn path(worker: uuid::Uuid, assessment: uuid::Uuid) -> String {
    format!("/api/workers/{worker}/assessments/{assessment}")
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn an_assessment_is_read_through_its_own_worker_only() {
    let app = common::create_test_router().await;
    let owner = worker(&app, "AssessOwner").await;
    let other = worker(&app, "AssessOther").await;
    let id = assessment(&app, owner, "completed").await;

    let (code, got) = call(&app, "GET", &path(owner, id), None).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(got["id"], id.to_string());
    assert_eq!(got["instrument"], "Numerical");

    for (who, uri) in [
        ("another worker's id", path(other, id)),
        ("an unknown worker", path(uuid::Uuid::new_v4(), id)),
        ("an unknown assessment", path(owner, uuid::Uuid::new_v4())),
    ] {
        let (code, _) = call(&app, "GET", &uri, None).await;
        assert_eq!(code, StatusCode::NOT_FOUND, "{who}");
    }

    let conn = common::db().await;
    common::purge_record(&conn, owner).await;
    common::purge_record(&conn, other).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn another_workers_url_cannot_change_or_delete_an_assessment() {
    let app = common::create_test_router().await;
    let owner = worker(&app, "AssessOwner").await;
    let other = worker(&app, "AssessOther").await;
    let id = assessment(&app, owner, "scheduled").await;

    let (code, _) = call(
        &app,
        "PUT",
        &path(other, id),
        Some(&json!({ "notes": "x" })),
    )
    .await;
    assert_eq!(code, StatusCode::NOT_FOUND, "update through another worker");
    let (code, _) = call(&app, "DELETE", &path(other, id), None).await;
    assert_eq!(code, StatusCode::NOT_FOUND, "delete through another worker");

    let (code, still) = call(&app, "GET", &path(owner, id), None).await;
    assert_eq!(code, StatusCode::OK, "the assessment survives");
    assert!(still["notes"].is_null(), "and is unchanged");

    let conn = common::db().await;
    common::purge_record(&conn, owner).await;
    common::purge_record(&conn, other).await;
}

/// A status change must be legal from the stored status; a legal one is
/// stored, an illegal one is a `422` and changes nothing.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn status_moves_follow_the_transition_rules() {
    let app = common::create_test_router().await;
    let owner = worker(&app, "AssessStatus").await;
    let id = assessment(&app, owner, "scheduled").await;

    let (code, moved) = call(
        &app,
        "PUT",
        &path(owner, id),
        Some(&json!({ "status": "in_progress" })),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{moved}");
    assert_eq!(moved["status"], "in_progress");

    let (code, _) = call(
        &app,
        "PUT",
        &path(owner, id),
        Some(&json!({ "status": "expired" })),
    )
    .await;
    assert_eq!(
        code,
        StatusCode::UNPROCESSABLE_ENTITY,
        "in_progress cannot expire"
    );
    let (code, _) = call(
        &app,
        "PUT",
        &path(owner, id),
        Some(&json!({ "status": "nonsense" })),
    )
    .await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "unknown status");

    let (_, now) = call(&app, "GET", &path(owner, id), None).await;
    assert_eq!(
        now["status"], "in_progress",
        "a refused move changes nothing"
    );

    let (code, _) = call(
        &app,
        "PUT",
        &path(owner, uuid::Uuid::new_v4()),
        Some(&json!({ "notes": "x" })),
    )
    .await;
    assert_eq!(code, StatusCode::NOT_FOUND);

    common::purge_record(&common::db().await, owner).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn a_deleted_assessment_is_gone_from_reads_and_the_profile() {
    let app = common::create_test_router().await;
    let owner = worker(&app, "AssessDelete").await;
    let id = assessment(&app, owner, "completed").await;

    let (code, profile) = call(
        &app,
        "GET",
        &format!("/api/workers/{owner}/assessment-profile"),
        None,
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{profile}");
    assert_eq!(profile["worker_id"], owner.to_string());
    assert_eq!(profile["masked"], false);
    assert_eq!(recorded(&profile, "aptitude"), 1, "{profile}");

    let (code, _) = call(&app, "DELETE", &path(owner, id), None).await;
    assert!(code.is_success(), "{code}");
    let (code, _) = call(&app, "GET", &path(owner, id), None).await;
    assert_eq!(
        code,
        StatusCode::NOT_FOUND,
        "a deleted assessment is not served"
    );
    let (code, _) = call(&app, "DELETE", &path(owner, id), None).await;
    assert_eq!(code, StatusCode::NOT_FOUND, "nor deleted twice");

    let (_, after) = call(
        &app,
        "GET",
        &format!("/api/workers/{owner}/assessment-profile"),
        None,
    )
    .await;
    assert_eq!(recorded(&after, "aptitude"), 0, "{after}");

    let (code, _) = call(
        &app,
        "GET",
        &format!("/api/workers/{}/assessment-profile", uuid::Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(code, StatusCode::NOT_FOUND);

    common::purge_record(&common::db().await, owner).await;
}
