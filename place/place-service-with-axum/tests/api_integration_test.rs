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

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::app::App;
use place_service::config::Config;
use place_service::models::place::Place;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. No auth env vars are touched, so
/// `PLACE_REQUIRE_AUTH` stays at its default (off).
async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
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

    let created: Place = serde_json::from_value(body.clone()).expect("Place in response body");
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

    let id_a = body_a["id"].as_str().unwrap();
    let id_b = body_b["id"].as_str().unwrap();
    assert_ne!(id_a, id_b, "each create must mint its own id");
    assert_ne!(id_a, "00000000-0000-0000-0000-000000000000");
    assert_ne!(id_b, "00000000-0000-0000-0000-000000000000");
}

/// Omitting `name` — a genuinely client-required field the server does
/// NOT own — still fails, but now via the normal validation path
/// (`422 validation`, field `name`), not via the JSON extractor's
/// generic "missing field" error before any handler code runs.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn omitting_name_fails_validation_not_the_json_extractor() {
    let app = test_router().await;

    let (status, body) = post_places(&app, &json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["code"], "validation",
        "expected the validation-layer error code, got: {body}"
    );
}

/// `GET` a path, returning the status, response headers, and parsed body.
async fn get_places(app: &axum::Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, Value) {
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

/// Mirrors thing-service's own T-15 fix: `GET /api/places` (no `q`) is a
/// genuine "enumerate the collection" endpoint, distinct from
/// `/places/search` (which returns zero hits for `q="*"` or an empty
/// `q`, regardless of how many records exist — the same
/// `tokenise` behaviour thing-service's search index has). Seeds three
/// places carrying no shared literal prefix (so the fuzzy matcher's
/// duplicate-check on create cannot flag any pair) and walks the list by
/// following `Link: rel="next"`. Places are listed newest-first, so the
/// three just-seeded rows land on the first two pages of two regardless
/// of how many other fixtures the rest of this suite has created.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn listing_with_no_query_term_enumerates_seeded_places() {
    let app = test_router().await;
    let names: Vec<String> = (0..3)
        .map(|_| uuid::Uuid::new_v4().simple().to_string())
        .collect();
    let mut seeded_ids = Vec::new();
    for name in &names {
        let (status, body) = post_places(&app, &json!({ "name": name })).await;
        assert_eq!(status, StatusCode::CREATED, "{body:?}");
        seeded_ids.push(body["id"].as_str().unwrap().to_string());
    }

    let mut url = "/api/places?limit=2".to_string();
    let mut returned_ids = Vec::new();
    for _ in 0..2 {
        let (status, headers, body) = get_places(&app, &url).await;
        assert_eq!(status, StatusCode::OK, "{body:?}");
        assert!(headers.get("x-total-count").is_none(), "no total");
        assert!(body.get("total").is_none(), "no total in the body");
        let results = body["results"].as_array().expect("results array");
        assert_eq!(results.len(), 2, "a full page while more follow");
        returned_ids.extend(
            results
                .iter()
                .map(|r| r["id"].as_str().unwrap().to_string()),
        );
        let link = headers["link"].to_str().unwrap();
        assert!(link.ends_with(r#">; rel="next""#), "{link}");
        url = link[1..link.find('>').unwrap()].to_string();
    }
    for id in &seeded_ids {
        assert!(
            returned_ids.contains(id),
            "seeded place {id} missing from the first two pages; returned {returned_ids:?}"
        );
    }
    let mut unique = returned_ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), returned_ids.len(), "no row twice");

    // Omitting `limit` is the old default page of 10.
    let (status, _headers, body) = get_places(&app, "/api/places").await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    assert!(body["results"].as_array().unwrap().len() <= 10);
}

/// A `limit` past `LIST_MAX_LIMIT` (100) is refused rather than clamped,
/// and so is a cursor this service did not sign: both `422`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn listing_with_a_bad_limit_or_cursor_is_refused() {
    let app = test_router().await;
    for uri in ["/api/places?limit=500", "/api/places?cursor=AQAAAAAAAAAK"] {
        let (status, _headers, body) = get_places(&app, uri).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body:?}");
        assert_eq!(body["code"], "invalid_page");
    }
}

/// A record verifies the moment it is written, and a row edited in SQL is
/// still caught (the check event's EVT-V1 failed). A digest computed from
/// a value other than the one stored, for instance before Postgres rounds
/// a timestamp to microseconds, flags every fresh record.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_written_record_verifies() {
    let app = test_router().await;
    let name = uuid::Uuid::new_v4().simple().to_string();
    let (status, body) = post_places(
        &app,
        &json!({ "name": name, "keywords": ["integrity", "audit"] }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {body}");
    let id = body["id"].as_str().expect("an id").to_string();

    let mismatched = |report: &Value| {
        report["mismatched"]
            .as_array()
            .expect("a mismatched list")
            .iter()
            .any(|m| m["id"] == id.as_str())
    };
    let (status, _, report) = get_places(&app, "/api/records/verify?limit=1000").await;
    assert_eq!(status, StatusCode::OK, "verify failed: {report}");
    assert!(!mismatched(&report), "a fresh record must verify: {report}");

    // An update rewrites the row and every child table, so it must stamp
    // too. Coordinates and an identifier put a NUMERIC column and a
    // second child table under the digest.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/places/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "name": format!("{name} updated"),
                        "keywords": ["integrity"],
                        "identifiers": [{ "identifier_type": "OpenStreetMap", "value": "n240109189" }],
                        "geo": { "latitude_as_decimal_degrees": 51.5, "longitude_as_decimal_degrees": -0.12 }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "update failed");
    let (status, _, report) = get_places(&app, "/api/records/verify?limit=1000").await;
    assert_eq!(status, StatusCode::OK, "verify failed: {report}");
    assert!(
        !mismatched(&report),
        "an updated record must verify: {report}"
    );

    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL"))
        .await
        .expect("connect for the tamper step");
    sqlx::query("UPDATE places SET name = name || ' (edited in SQL)' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("tamper with the row");
    let (_, _, report) = get_places(&app, "/api/records/verify?limit=1000").await;
    assert!(
        mismatched(&report),
        "a row edited in SQL must be flagged: {report}"
    );
}
