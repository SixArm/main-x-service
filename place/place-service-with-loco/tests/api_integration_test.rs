//! REST API integration tests requiring a running `PostgreSQL`.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test api_integration_test -- --ignored`.
//!
//! This file's reason to exist is **QA-SERVER-FIELDS**: `POST /api/places`
//! used to require every field the [`Place`] model declared without a
//! serde default — `id`, `is_deleted`, `created_at`, `updated_at`,
//! `keywords`, `identifiers`, `amenity_features`, `opening_hours` — all
//! of which the server owns and now overwrites/mints. A hand-written
//! create body (the way a real API client writes one) omitting those
//! fields used to be refused by the JSON extractor with `422 missing
//! field …` before the handler ever ran, even though the value it
//! demanded was discarded. Same defect, same fix, as the event service's
//! `created_at`/`updated_at` fix (2026-08-01).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::api::rest::{AppState, create_router};
use place_service::config::Config;
use place_service::db::create_connection;
use place_service::matching::PlaceMatcher;
use place_service::models::place::Place;
use place_service::search::SearchEngine;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. No auth env vars are touched, so
/// `PLACE_REQUIRE_AUTH` stays at its default (off).
async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let search_engine = SearchEngine::new(&config.search.index_path).expect("search engine");
    let matcher = PlaceMatcher::new(&config.matching);
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection — set DATABASE_URL to a running, migrated database");
    create_router(AppState::new(db, search_engine, matcher, config))
}

/// `POST` a hand-built JSON body, returning the status **and** the parsed
/// body, so a refusal reports the server's reason rather than only a
/// status mismatch.
async fn post_places(app: &axum::Router, body: &Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/places")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

/// A hand-built body carrying only `name` — the one field the server does
/// not own — must succeed. Before the fix this was refused by the JSON
/// extractor (`422 missing field id`, the first field it hit) without
/// the handler, validation, or the repository ever running.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn create_place_from_a_minimal_hand_written_body_succeeds() {
    let app = test_router().await;
    // A pure random token, no fixed literal words: a shared literal
    // suffix (even something like "Place") scores deceptively high on
    // Jaro-Winkler's similarity and can be flagged as a duplicate against
    // leftover rows from an earlier run of this same test against a
    // database that was not reset in between (CI always starts fresh —
    // see `scripts/ci-check.sh test-db` — but a repeated local run
    // against the same `test-db.sh up` container would not).
    let name = uuid::Uuid::new_v4().simple().to_string();

    let (status, body) = post_places(&app, &json!({ "name": name.clone() })).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create failed: {}",
        serde_json::to_string(&body).unwrap()
    );

    let created: Place =
        serde_json::from_value(body["data"].clone()).expect("Place in response body");
    assert_eq!(created.name, name);

    // The id is server-minted, not the nil sentinel the omitted field
    // defaulted to.
    assert_ne!(created.id, uuid::Uuid::nil());

    // created_at/updated_at are server-stamped to "now", not the Unix
    // epoch `DateTime<Utc>::default()` would otherwise have produced.
    let age = chrono::Utc::now() - created.created_at;
    assert!(
        age.num_seconds().abs() < 60,
        "created_at should be ~now, got {}",
        created.created_at
    );
    assert_eq!(created.created_at, created.updated_at);

    // The soft-delete flag defaults to "active", not an arbitrary value.
    assert!(!created.is_deleted);

    // Every collection field the model declares without a default reads
    // back empty rather than having failed the extractor.
    assert!(created.keywords.is_empty());
    assert!(created.identifiers.is_empty());
    assert!(created.amenity_features.is_empty());
    assert!(created.opening_hours.is_empty());
}

/// Two hand-written creates in a row — the regression the naive fix
/// (making the fields optional without minting a fresh id) would have
/// missed: an omitted `id` defaults to the nil UUID, and if the server
/// trusted it verbatim the second create would collide on the same
/// primary key instead of getting a fresh one.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn two_consecutive_hand_written_creates_do_not_collide() {
    let app = test_router().await;
    // Two fully independent random names, no shared literal prefix and no
    // shared UUID prefix: a shared prefix (even just a constant word like
    // "Test") scores deceptively high on Jaro-Winkler's prefix bonus and
    // can be flagged as a duplicate against leftover rows from an earlier
    // run of this same test — a distinct, already-covered concern, not
    // what this test is pinning.
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();

    let (status_a, body_a) = post_places(&app, &json!({ "name": a })).await;
    assert_eq!(status_a, StatusCode::CREATED, "{body_a:?}");

    let (status_b, body_b) = post_places(&app, &json!({ "name": b })).await;
    assert_eq!(status_b, StatusCode::CREATED, "{body_b:?}");

    let id_a = body_a["data"]["id"].as_str().unwrap();
    let id_b = body_b["data"]["id"].as_str().unwrap();
    assert_ne!(id_a, id_b, "each create must mint its own id");
    assert_ne!(id_a, "00000000-0000-0000-0000-000000000000");
    assert_ne!(id_b, "00000000-0000-0000-0000-000000000000");
}

/// Omitting `name` — a genuinely client-required field the server does
/// NOT own — still fails, but now via the normal validation path
/// (`422 validation_error`, field `name`), not via the JSON extractor's
/// generic "missing field" error before any handler code runs.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn omitting_name_fails_validation_not_the_json_extractor() {
    let app = test_router().await;

    let (status, body) = post_places(&app, &json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["error"]["code"], "validation_error",
        "expected the validation-layer error code, got: {body}"
    );
}
