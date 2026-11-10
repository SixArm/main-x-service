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

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use thing_service::app::App;
use thing_service::config::Config;
use thing_service::models::thing::Thing;
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. No auth env vars are touched, so
/// `THING_REQUIRE_AUTH` stays at its default (off).
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

    let created: Thing = serde_json::from_value(body.clone()).expect("Thing in response body");
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

    let (status, body) = post_things(&app, &json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["code"], "validation",
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
/// create cannot flag any pair), then walks the collection page by page
/// by following `Link: rel="next"` and asserts every seeded thing is
/// seen. "Every seeded thing", not an exact count, because the suite's
/// other tests share this database and may create rows of their own
/// while the walk runs; a row inserted mid-walk can only push older rows
/// later, never past the walk, so a seeded row is never skipped.
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
        seeded_ids.push(body["id"].as_str().unwrap().to_string());
    }

    // Follow `Link: rel="next"` from the first page of two to the last.
    let mut url = "/api/things?limit=2".to_string();
    let mut returned_ids = std::collections::BTreeSet::new();
    let mut pages = 0;
    loop {
        let (status, headers, body) = get_things(&app, &url).await;
        assert_eq!(status, StatusCode::OK, "{url}: {body:?}");
        for gone in ["x-total-count", "x-limit", "x-offset"] {
            assert!(headers.get(gone).is_none(), "no {gone} header");
        }
        assert!(body.get("total").is_none(), "no total in the body");
        let results = body["results"].as_array().expect("results array");
        assert!(results.len() <= 2, "a page is at most the limit");
        returned_ids.extend(
            results
                .iter()
                .map(|r| r["id"].as_str().unwrap().to_string()),
        );
        pages += 1;
        let Some(link) = headers.get("link") else {
            break;
        };
        let link = link.to_str().expect("ascii link");
        assert!(link.ends_with(r#">; rel="next""#), "{link}");
        url = link[1..link.find('>').expect("<url>")].to_string();
    }
    assert!(
        pages >= 2,
        "three seeded rows need at least two pages of two"
    );
    for id in &seeded_ids {
        assert!(
            returned_ids.contains(id),
            "seeded thing {id} missing from the walk; returned {returned_ids:?}"
        );
    }
}

/// A `limit` past the maximum is refused, never clamped, and a cursor
/// this service did not sign is refused rather than read: both `422`,
/// coded `invalid_page`. There is no offset to bound any more, because a
/// position comes only from a signed cursor.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn listing_with_a_bad_limit_or_cursor_is_refused() {
    let app = test_router().await;
    for uri in [
        "/api/things?limit=501",
        "/api/things?limit=0",
        "/api/things?cursor=AQAAAAAAAAAK",
    ] {
        let (status, _headers, body) = get_things(&app, uri).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body:?}");
        assert_eq!(body["code"], "invalid_page", "{uri}");
    }
}

/// A record verifies the moment it is written, and again after an update,
/// and a row edited in SQL is still caught (the check event's EVT-V1
/// failed; INT-V1 here). A digest computed from a value other than the
/// one stored, for instance before Postgres rounds a timestamp to
/// microseconds, flags every fresh record. Child rows (alternate names,
/// an identifier, `sameAs` links) are included, since they are in the
/// digested record but not in the `things` row.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_written_record_verifies() {
    let app = test_router().await;
    let name = uuid::Uuid::new_v4().simple().to_string();
    let (status, body) = post_things(
        &app,
        &json!({
            "name": name,
            "alternate_names": ["integrity", "audit"],
            "identifiers": [{ "property_id": "Sku", "value": format!("SKU-{name}") }],
            "same_as": ["https://example.org/integrity"],
            "created_at": "2001-01-01T00:00:00.987654321Z",
            "updated_at": "2001-01-01T00:00:00.987654321Z",
        }),
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
    let (status, _, report) = get_things(&app, "/api/records/verify?limit=1000").await;
    assert_eq!(status, StatusCode::OK, "verify failed: {report}");
    assert!(!mismatched(&report), "a fresh record must verify: {report}");

    let mut changed = body.clone();
    changed["description"] = json!("Edited after creation");
    changed["alternate_names"] = json!(["integrity", "audit", "edited"]);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/things/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&changed).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "update failed: {}",
        String::from_utf8_lossy(&bytes)
    );
    let (_, _, report) = get_things(&app, "/api/records/verify?limit=1000").await;
    assert!(
        !mismatched(&report),
        "an updated record must verify: {report}"
    );

    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL"))
        .await
        .expect("connect for the tamper step");
    sqlx::query("UPDATE things SET name = name || ' (edited in SQL)' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("tamper with the row");
    let (_, _, report) = get_things(&app, "/api/records/verify?limit=1000").await;
    assert!(
        mismatched(&report),
        "a row edited in SQL must be flagged: {report}"
    );
}
