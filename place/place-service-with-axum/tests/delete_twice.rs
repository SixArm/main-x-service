//! SEC-B12: deleting a place that is already deleted, or never existed, is a
//! `404`. It used to answer `204` and re-stamp `deleted_at`, losing when
//! the record was really deleted, and publish another `deleted` event.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test delete_twice -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn app() -> axum::Router {
    let config = place_service::config::Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    <place_service::app::App as axa_test::TestApp>::boot_test()
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

async fn deleted_at(id: uuid::Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    let url = std::env::var("DB_URL").expect("DB_URL");
    let db = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT deleted_at FROM places WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn a_second_delete_is_404_and_keeps_the_first_deletion_time() {
    let app = app().await;
    let name = format!("Deletetwice {}", uuid::Uuid::new_v4().simple());
    let (status, created) = call(&app, "POST", "/api/places", Some(&json!({ "name": name }))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id: uuid::Uuid = created["id"].as_str().unwrap().parse().unwrap();
    let uri = format!("/api/places/{id}");

    let (status, _) = call(&app, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let first = deleted_at(id).await.expect("deleted_at is stamped");

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let (status, body) = call(&app, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        deleted_at(id).await,
        Some(first),
        "the deletion time is not rewritten"
    );

    let (status, _) = call(
        &app,
        "DELETE",
        &format!("/api/places/{}", uuid::Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "an unknown id");
}
