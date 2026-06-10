#![warn(clippy::pedantic)]

//! Integration tests for the REST API endpoints.
//!
//! These tests build a real [`Router`](axum::Router) via
//! [`common::create_test_router`] (which opens a database connection and
//! search index from the environment config) and drive it with
//! `tower::ServiceExt::oneshot` requests, asserting on status codes and
//! decoded [`ApiResponse`] bodies. They require a reachable PostgreSQL
//! instance (see `docker-compose.test.yml`) and exercise the full
//! create / get / update / delete / search lifecycle plus the
//! not-found path.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for `oneshot` and `ready`
use serde_json::json;

use person_service::{
    models::Person,
    api::ApiResponse,
};

/// `GET /api/v1/health` returns 200 and identifies the service.
#[tokio::test]
#[ignore]
async fn test_health_check() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("healthy"));
    assert!(body_str.contains("person-service"));
}

/// `POST /api/v1/persons` creates a person and assigns a fresh UUID
/// (ignoring the all-zero id in the payload).
#[tokio::test]
#[ignore]
async fn test_create_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Create");

    let person_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Integration", "Test"]
        },
        "birth_date": "1990-05-15",
        "gender": "female"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let api_response: ApiResponse<Person> = serde_json::from_slice(&body).unwrap();
    assert!(api_response.success);

    let person = api_response.data.unwrap();
    assert_eq!(person.name.family, family_name);
    assert_eq!(person.name.given, vec!["Integration", "Test"]);
    assert!(person.id.to_string() != "00000000-0000-0000-0000-000000000000");
}

/// Create a person, then `GET /api/v1/persons/{id}` returns the same
/// record.
#[tokio::test]
#[ignore]
async fn test_create_and_get_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("CreateGet");

    // Create person
    let person_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Get", "Test"]
        },
        "birth_date": "1985-03-20",
        "gender": "male"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Person> = serde_json::from_slice(&create_body).unwrap();
    let created_person = create_api_response.data.unwrap();
    let person_id = created_person.id;

    // Get person by ID
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/persons/{}", person_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let get_api_response: ApiResponse<Person> = serde_json::from_slice(&get_body).unwrap();
    assert!(get_api_response.success);

    let retrieved_person = get_api_response.data.unwrap();
    assert_eq!(retrieved_person.id, person_id);
    assert_eq!(retrieved_person.name.family, family_name);
}

/// Create a person, then `PUT /api/v1/persons/{id}` persists a changed
/// given-name list.
#[tokio::test]
#[ignore]
async fn test_update_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Update");

    // Create person
    let person_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Update"]
        },
        "birth_date": "1975-11-10",
        "gender": "other"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Person> = serde_json::from_slice(&create_body).unwrap();
    let mut person = create_api_response.data.unwrap();

    // Update person
    person.name.given = vec!["Update".to_string(), "Modified".to_string()];

    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/persons/{}", person.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let update_api_response: ApiResponse<Person> = serde_json::from_slice(&update_body).unwrap();
    let updated_person = update_api_response.data.unwrap();

    assert_eq!(updated_person.name.given, vec!["Update", "Modified"]);
}

/// Create a person, soft-delete it (204), then confirm a subsequent GET
/// returns 404.
#[tokio::test]
#[ignore]
async fn test_delete_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Delete");

    // Create person
    let person_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Delete"]
        },
        "birth_date": "1988-07-25",
        "gender": "unknown"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Person> = serde_json::from_slice(&create_body).unwrap();
    let person = create_api_response.data.unwrap();

    // Delete person
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/persons/{}", person.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Try to get deleted person - should return None (or 404 depending on implementation)
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/persons/{}", person.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Soft delete means person is not returned
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

/// Create a person, then `GET /api/v1/persons/search` finds it by family
/// name (after a brief indexing delay).
#[tokio::test]
#[ignore]
async fn test_search_persons() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Search");

    // Create a person to search for
    let person_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Searchable"]
        },
        "birth_date": "1992-04-18",
        "gender": "female"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Give search engine time to index (in production this would be async)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Search for the person
    let search_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/persons/search?q={}&limit=10", family_name))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body = axum::body::to_bytes(search_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let body_str = String::from_utf8(search_body.to_vec()).unwrap();

    // Should contain the search term
    assert!(body_str.contains(&family_name));
}

/// `GET /api/v1/persons/{id}` for an unknown id returns 404.
#[tokio::test]
#[ignore]
async fn test_get_person_not_found() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/persons/00000000-0000-0000-0000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
