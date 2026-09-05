//! `check-duplicates` / create's `409` candidates honour `?mask_sensitive=`
//! the same way `search` does (spec §13 T-15).
//!
//! Per `agents/share/security.md` invariant 5, a bulk/aggregate read must
//! never reveal more than the equivalent single `GET` would — without
//! this, a caller unable to see a place's full record via `GET` could
//! still recover it by posting a near-duplicate probe.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test masking_check_duplicates -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::api::rest::{AppState, create_router};
use place_service::config::Config;
use place_service::db::create_connection;
use place_service::matching::PlaceMatcher;
use place_service::models::identifier::PlaceIdentifier;
use place_service::models::place::Place;
use place_service::search::SearchEngine;
use serde_json::Value;
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. Same construction `api_integration_test.rs` uses.
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

async fn post(app: &axum::Router, uri: &str, place: &Place) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(place).unwrap()))
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

/// A place with a telephone number and a globally-unique, check-digit-valid
/// GLN identifier, which deterministically short-circuits the matcher to
/// 1.0 (see `src/validation/mod.rs::gln_is_valid`'s doctest for the same
/// real GS1 example), so a probe sharing the GLN is a reliable duplicate
/// hit regardless of name-index blocking specifics.
fn phoned_place(name: &str) -> Place {
    let mut place = Place::new(name);
    place.telephone = Some("+1-555-867-5309".into());
    place.identifiers = vec![PlaceIdentifier::gln("0614141999996")];
    place
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test masking_check_duplicates -- --ignored"]
async fn check_duplicates_masks_candidates_when_requested() {
    let app = test_router().await;

    let (status, _) = post(&app, "/api/places", &phoned_place("Central Park")).await;
    assert_eq!(status, StatusCode::CREATED, "seed place should create");

    // Default (no ?mask_sensitive): unmasked, matching search's own default.
    let (status, body) = post(
        &app,
        "/api/places/check-duplicates",
        &phoned_place("Central Park (duplicate probe)"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let candidates = body["data"]["candidates"].as_array().expect("candidates");
    assert!(
        !candidates.is_empty(),
        "expected a duplicate hit, got {body}"
    );
    assert_eq!(
        candidates[0]["place"]["telephone"], "+1-555-867-5309",
        "unmasked by default: {body}"
    );

    // `?mask_sensitive=true`: the candidate's telephone is redacted the
    // same way `mask_place`/`/masked` redact it.
    let (status, body) = post(
        &app,
        "/api/places/check-duplicates?mask_sensitive=true",
        &phoned_place("Central Park (masked probe)"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let candidates = body["data"]["candidates"].as_array().expect("candidates");
    assert!(
        !candidates.is_empty(),
        "expected a duplicate hit, got {body}"
    );
    let telephone = candidates[0]["place"]["telephone"]
        .as_str()
        .expect("telephone");
    assert!(
        telephone.ends_with("****") && !telephone.contains("5309"),
        "telephone should be masked: {body}"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test masking_check_duplicates -- --ignored"]
async fn create_409_masks_the_duplicate_candidate_when_requested() {
    let app = test_router().await;

    let (status, _) = post(&app, "/api/places", &phoned_place("Emma's Diner")).await;
    assert_eq!(status, StatusCode::CREATED, "seed place should create");

    // The 409 duplicate body respects the same query flag as create's own
    // request, since the caller posting the create IS the caller who
    // would otherwise recover the hidden record from the 409 body.
    let (status, body) = post(
        &app,
        "/api/places?mask_sensitive=true",
        &phoned_place("Emma's Diner (duplicate probe)"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let details = body["error"]["details"].as_array().expect("details");
    assert!(
        !details.is_empty(),
        "expected a duplicate candidate, got {body}"
    );
    let telephone = details[0]["place"]["telephone"]
        .as_str()
        .expect("telephone");
    assert!(
        telephone.ends_with("****") && !telephone.contains("5309"),
        "telephone should be masked: {body}"
    );
}
