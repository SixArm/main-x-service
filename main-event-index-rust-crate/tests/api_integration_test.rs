//! Integration tests for REST API endpoints

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for `oneshot` and `ready`
use serde_json::json;

use main_event_index::{
    models::Event,
    api::ApiResponse,
};

#[tokio::test]
async fn test_health_check() {
    let app = common::create_test_router();

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
    assert!(body_str.contains("main-event-index"));
}

#[tokio::test]
async fn test_create_event() {
    let app = common::create_test_router();

    let family_name = common::unique_event_name("Create");

    let event_json = json!({
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
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&event_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let api_response: ApiResponse<Event> = serde_json::from_slice(&body).unwrap();
    assert!(api_response.success);

    let event = api_response.data.unwrap();
    assert_eq!(event.name.family, family_name);
    assert_eq!(event.name.given, vec!["Integration", "Test"]);
    assert!(event.id.to_string() != "00000000-0000-0000-0000-000000000000");
}

#[tokio::test]
async fn test_create_and_get_event() {
    let app = common::create_test_router();

    let family_name = common::unique_event_name("CreateGet");

    // Create event
    let event_json = json!({
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
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&event_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Event> = serde_json::from_slice(&create_body).unwrap();
    let created_event = create_api_response.data.unwrap();
    let event_id = created_event.id;

    // Get event by ID
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/events/{}", event_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let get_api_response: ApiResponse<Event> = serde_json::from_slice(&get_body).unwrap();
    assert!(get_api_response.success);

    let retrieved_event = get_api_response.data.unwrap();
    assert_eq!(retrieved_event.id, event_id);
    assert_eq!(retrieved_event.name.family, family_name);
}

#[tokio::test]
async fn test_update_event() {
    let app = common::create_test_router();

    let family_name = common::unique_event_name("Update");

    // Create event
    let event_json = json!({
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
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&event_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Event> = serde_json::from_slice(&create_body).unwrap();
    let mut event = create_api_response.data.unwrap();

    // Update event
    event.name.given = vec!["Update".to_string(), "Modified".to_string()];

    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/events/{}", event.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&event).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let update_api_response: ApiResponse<Event> = serde_json::from_slice(&update_body).unwrap();
    let updated_event = update_api_response.data.unwrap();

    assert_eq!(updated_event.name.given, vec!["Update", "Modified"]);
}

#[tokio::test]
async fn test_delete_event() {
    let app = common::create_test_router();

    let family_name = common::unique_event_name("Delete");

    // Create event
    let event_json = json!({
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
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&event_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Event> = serde_json::from_slice(&create_body).unwrap();
    let event = create_api_response.data.unwrap();

    // Delete event
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/events/{}", event.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Try to get deleted event - should return None (or 404 depending on implementation)
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/events/{}", event.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Soft delete means event is not returned
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_search_events() {
    let app = common::create_test_router();

    let family_name = common::unique_event_name("Search");

    // Create a event to search for
    let event_json = json!({
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
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&event_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Give search engine time to index (in production this would be async)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Search for the event
    let search_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/events/search?q={}&limit=10", family_name))
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

#[tokio::test]
async fn test_get_event_not_found() {
    let app = common::create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/events/00000000-0000-0000-0000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
