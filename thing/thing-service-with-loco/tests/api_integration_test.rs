//! REST API integration tests requiring a running `PostgreSQL`.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test api_integration_test -- --ignored`.
//!
//! This file's reason to exist is **QA-SERVER-FIELDS**: `POST /api/things`
//! used to require every field the [`Thing`] model declared without a
//! serde default — `id`, `is_deleted`, `created_at`, `updated_at`,
//! `alternate_names`, `identifiers`, `images`, `same_as` — all of which
//! the server owns and now overwrites/mints. A hand-written create body
//! (the way a real API client writes one) omitting those fields used to
//! be refused by the JSON extractor with `422 missing field …` before
//! the handler ever ran, even though the value it demanded was
//! discarded. Same defect, same fix, as the event service's
//! `created_at`/`updated_at` fix (2026-08-01).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use thing_service::api::rest::{AppState, create_router};
use thing_service::config::Config;
use thing_service::db::create_connection;
use thing_service::matching::ProbabilisticMatcher;
use thing_service::models::thing::Thing;
use thing_service::search::SearchEngine;
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. No auth env vars are touched, so
/// `THING_REQUIRE_AUTH` stays at its default (off).
async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let search_engine = SearchEngine::new(&config.search.index_path).expect("search engine");
    let matcher = ProbabilisticMatcher::new(&config.matching);
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection — set DATABASE_URL to a running, migrated database");
    create_router(AppState::new(db, search_engine, matcher, config))
}

/// `POST` a hand-built JSON body, returning the status **and** the parsed
/// body, so a refusal reports the server's reason rather than only a
/// status mismatch.
async fn post_things(app: &axum::Router, body: &Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/things")
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
async fn create_thing_from_a_minimal_hand_written_body_succeeds() {
    let app = test_router().await;
    // A pure random token, no fixed literal words: a shared literal
    // suffix (even something like "Thing") scores deceptively high on
    // Jaro-Winkler's similarity and can be flagged as a duplicate against
    // leftover rows from an earlier run of this same test against a
    // database that was not reset in between (CI always starts fresh —
    // see `scripts/ci-check.sh test-db` — but a repeated local run
    // against the same `test-db.sh up` container would not).
    let name = uuid::Uuid::new_v4().simple().to_string();

    let (status, body) = post_things(&app, &json!({ "name": name.clone() })).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create failed: {}",
        serde_json::to_string(&body).unwrap()
    );

    let created: Thing =
        serde_json::from_value(body["data"].clone()).expect("Thing in response body");
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
    assert!(created.alternate_names.is_empty());
    assert!(created.identifiers.is_empty());
    assert!(created.images.is_empty());
    assert!(created.same_as.is_empty());
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

    let (status_a, body_a) = post_things(&app, &json!({ "name": a })).await;
    assert_eq!(status_a, StatusCode::CREATED, "{body_a:?}");

    let (status_b, body_b) = post_things(&app, &json!({ "name": b })).await;
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

    let (status, body) = post_things(&app, &json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["error"]["code"], "validation_error",
        "expected the validation-layer error code, got: {body}"
    );
}

/// `GET` a path, returning the status, response headers, and parsed body.
async fn get_things(app: &axum::Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, parsed)
}

/// T-15 — `GET /api/things` (no `q`) is a genuine "enumerate the
/// collection" endpoint, distinct from `/things/search` (which returns
/// zero hits for `q="*"` or an empty `q`, regardless of how many
/// records exist — the front-end's stated assumption that a bare `*`
/// lists everything did not hold). Seeds three things carrying no
/// shared literal prefix (so the fuzzy matcher's duplicate-check on
/// create cannot flag any pair) and asserts a plain, term-free list
/// call returns at least those three — "at least" because the suite's
/// other tests share this database and may have created rows of their
/// own; the pagination headers on this call name the true collection
/// size regardless.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn listing_with_no_query_term_enumerates_seeded_things() {
    let app = test_router().await;
    let names: Vec<String> = (0..3)
        .map(|_| uuid::Uuid::new_v4().simple().to_string())
        .collect();
    let mut seeded_ids = Vec::new();
    for name in &names {
        let (status, body) = post_things(&app, &json!({ "name": name })).await;
        assert_eq!(status, StatusCode::CREATED, "{body:?}");
        seeded_ids.push(body["data"]["id"].as_str().unwrap().to_string());
    }

    // A page large enough to cover this test's own seeded rows alongside
    // whatever the rest of the suite has already created.
    let (status, headers, body) = get_things(&app, "/api/things?limit=500").await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    let results = body["data"]["results"].as_array().expect("results array");
    let returned_ids: Vec<&str> = results.iter().map(|r| r["id"].as_str().unwrap()).collect();
    for id in &seeded_ids {
        assert!(
            returned_ids.contains(&id.as_str()),
            "seeded thing {id} missing from the unfiltered list; returned {returned_ids:?}"
        );
    }

    // The family pagination headers (`agents/share/restful.md`): the
    // total is the true collection size (at least the 3 just seeded),
    // never the empty answer `/things/search?q=*` gives today.
    let total_count: u64 = headers["x-total-count"].to_str().unwrap().parse().unwrap();
    assert!(
        total_count >= 3,
        "X-Total-Count should count the whole collection, got {total_count}"
    );
    assert_eq!(headers["x-limit"], "500");
    assert_eq!(headers["x-offset"], "0");
}

/// An `offset` beyond the SEC-G7 bound is a `400`, not a database query
/// that materialises and discards ten thousand-plus rows.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn listing_beyond_the_offset_bound_is_rejected() {
    let app = test_router().await;
    let (status, _headers, body) = get_things(&app, "/api/things?offset=10001").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
    assert_eq!(body["error"]["code"], "OFFSET_TOO_LARGE");
}
