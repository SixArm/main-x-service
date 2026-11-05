//! `check-duplicates` / create's `409` candidates honour `?mask_sensitive=`
//! the same way `search` does (spec §13 T-13).
//!
//! Per `agents/share/security.md` invariant 5, a bulk/aggregate read must
//! never reveal more than the equivalent single `GET` would — without
//! this, a caller unable to see a thing's full record via `GET` could
//! still recover it by posting a near-duplicate probe.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test masking_check_duplicates -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use thing_service::api::rest::{AppState, create_router};
use thing_service::config::Config;
use thing_service::db::create_connection;
use thing_service::matching::ProbabilisticMatcher;
use thing_service::search::SearchEngine;
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. Same construction `api_integration_test.rs` uses.
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

async fn post(app: &axum::Router, uri: &str, body: &Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
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

/// A thing with an owner and a globally-unique ISBN identifier, which
/// deterministically short-circuits the matcher to near-1.0 (see
/// `tests/duplicate_detection.rs`), so a probe sharing the ISBN is a
/// reliable duplicate hit regardless of name-index blocking specifics.
fn owned_book(name: &str) -> Value {
    json!({
        "name": name,
        "owner": "Acme Library Trust",
        "identifiers": [{ "property_id": "Isbn", "value": "9780141439518" }],
    })
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test masking_check_duplicates -- --ignored"]
async fn check_duplicates_masks_candidates_when_requested() {
    let app = test_router().await;

    let (status, _) = post(&app, "/api/things", &owned_book("Pride and Prejudice")).await;
    assert_eq!(status, StatusCode::CREATED, "seed thing should create");

    // Default (no ?mask_sensitive): unmasked, matching search's own default.
    let (status, body) = post(
        &app,
        "/api/things/check-duplicates",
        &owned_book("Pride and Prejudice (duplicate probe)"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let candidates = body["data"]["candidates"].as_array().expect("candidates");
    assert!(
        !candidates.is_empty(),
        "expected a duplicate hit, got {body}"
    );
    assert_eq!(
        candidates[0]["thing"]["owner"], "Acme Library Trust",
        "unmasked by default: {body}"
    );

    // `?mask_sensitive=true`: the candidate's owner/identifier are
    // redacted the same way `mask_thing`/`/masked` redact them.
    let (status, body) = post(
        &app,
        "/api/things/check-duplicates?mask_sensitive=true",
        &owned_book("Pride and Prejudice (masked probe)"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let candidates = body["data"]["candidates"].as_array().expect("candidates");
    assert!(
        !candidates.is_empty(),
        "expected a duplicate hit, got {body}"
    );
    let candidate = &candidates[0]["thing"];
    assert_eq!(candidate["owner"], "[owner withheld]", "{body}");
    let identifiers = candidate["identifiers"].as_array().expect("identifiers");
    assert!(
        identifiers[0]["value"]
            .as_str()
            .expect("identifier value")
            .starts_with("****"),
        "identifier value should be masked: {body}"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test masking_check_duplicates -- --ignored"]
async fn create_409_masks_the_duplicate_candidate_when_requested() {
    let app = test_router().await;

    let (status, _) = post(&app, "/api/things", &owned_book("Emma")).await;
    assert_eq!(status, StatusCode::CREATED, "seed thing should create");

    // The 409 duplicate body respects the same query flag as create's
    // own request, since the caller posting the create IS the caller
    // who would otherwise recover the hidden record from the 409 body.
    let (status, body) = post(
        &app,
        "/api/things?mask_sensitive=true",
        &owned_book("Emma (duplicate probe)"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let details = body["error"]["details"].as_array().expect("details");
    assert!(
        !details.is_empty(),
        "expected a duplicate candidate, got {body}"
    );
    assert_eq!(details[0]["thing"]["owner"], "[owner withheld]", "{body}");
}
