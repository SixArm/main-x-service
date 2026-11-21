//! The `CourseInstance` sub-resource over HTTP: read, replace and delete
//! one instance. Create and list are in `api_integration_test.rs`.
//!
//! What matters most: an instance is reached only through its own course.
//! Naming it under another course's id is a `404`, so a caller cannot read,
//! change or delete an instance by guessing its id under a course they can
//! reach.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test instances_http -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn course(app: &axum::Router, suffix: &str) -> String {
    let (status, body) = call(
        app,
        "POST",
        "/api/courses",
        Some(&common::course_json(suffix)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().to_string()
}

fn instance(course_id: &str, name: &str) -> Value {
    json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "course_id": course_id,
        "name": name,
        "status": "scheduled",
    })
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn an_instance_is_reached_through_its_own_course_only() {
    let app = common::create_test_router().await;
    let owner = course(&app, "InstanceOwner").await;
    let other = course(&app, "InstanceOther").await;
    let (status, created) = call(
        &app,
        "POST",
        &format!("/api/courses/{owner}/instances"),
        Some(&instance(&owner, "Spring cohort")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id = created["id"].as_str().unwrap().to_string();
    let mine = format!("/api/courses/{owner}/instances/{id}");
    let theirs = format!("/api/courses/{other}/instances/{id}");

    let (status, read) = call(&app, "GET", &mine, None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["name"], "Spring cohort");
    assert_eq!(
        call(&app, "GET", &theirs, None).await.0,
        StatusCode::NOT_FOUND,
        "read through another course"
    );

    let (status, _) = call(&app, "PUT", &theirs, Some(&instance(&other, "Hijacked"))).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "update through another course"
    );
    assert_eq!(
        call(&app, "DELETE", &theirs, None).await.0,
        StatusCode::NOT_FOUND,
        "delete through another course"
    );
    assert_eq!(
        call(&app, "GET", &mine, None).await.1["name"],
        "Spring cohort",
        "and it is unchanged"
    );

    let (status, updated) =
        call(&app, "PUT", &mine, Some(&instance(&owner, "Autumn cohort"))).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        call(&app, "GET", &mine, None).await.1["name"],
        "Autumn cohort"
    );

    let mut bad = instance(&owner, "Bad language");
    bad["in_language"] = json!(["not a language tag!"]);
    let (status, body) = call(&app, "PUT", &mine, Some(&bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let unknown = format!("/api/courses/{owner}/instances/{}", uuid::Uuid::new_v4());
    assert_eq!(
        call(&app, "GET", &unknown, None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "PUT", &unknown, Some(&instance(&owner, "Nobody")))
            .await
            .0,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        call(&app, "DELETE", &mine, None).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, "GET", &mine, None).await.0,
        StatusCode::NOT_FOUND,
        "a deleted instance is not served"
    );
    assert_eq!(
        call(&app, "DELETE", &mine, None).await.0,
        StatusCode::NOT_FOUND,
        "nor deleted twice"
    );
}

/// A deleted course's instances go with it. Listing them was already a
/// `404`, but one instance stayed readable and writable by id.
#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn a_deleted_courses_instances_are_not_served() {
    let app = common::create_test_router().await;
    let parent = course(&app, "DeletedParent").await;
    let (status, created) = call(
        &app,
        "POST",
        &format!("/api/courses/{parent}/instances"),
        Some(&instance(&parent, "Orphan cohort")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id = created["id"].as_str().unwrap().to_string();
    let uri = format!("/api/courses/{parent}/instances/{id}");
    assert_eq!(
        call(&app, "DELETE", &format!("/api/courses/{parent}"), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );

    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/courses/{parent}/instances"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "GET", &uri, None).await.0,
        StatusCode::NOT_FOUND,
        "read"
    );
    assert_eq!(
        call(&app, "PUT", &uri, Some(&instance(&parent, "Changed")))
            .await
            .0,
        StatusCode::NOT_FOUND,
        "update"
    );
    assert_eq!(
        call(&app, "DELETE", &uri, None).await.0,
        StatusCode::NOT_FOUND,
        "delete"
    );
}
