#![warn(clippy::pedantic)]

//! `POST /api/persons/deduplicate` honours a `threshold` below the
//! matcher's configured one. It filtered through `find_matches`, which
//! drops anything under the configured threshold first, so a request for
//! a lower one (the documented default is `0.7`) changed nothing.
//!
//! `#[ignore]`d — run with
//! `DB_URL=… cargo test --test batch_threshold -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use person_service::db::AuditContext;
use person_service::models::{Gender, HumanName, Person};
use serde_json::{Value, json};
use tower::ServiceExt;

fn named(family: &str, given: &str) -> Person {
    Person::new(
        HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec![given.to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Unknown,
    )
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn a_pair_under_the_configured_threshold_is_found_at_a_lower_request_threshold() {
    let state = common::create_test_app_state().await;
    let app = common::create_test_router().await;
    // Different given and family names: a weak pair that scores
    // under the configured threshold on its own. The pair is found by id,
    // so the names need not be unique.
    let a = state
        .person_repository
        .create(&named("Hartley", "Margaret"), &AuditContext::default())
        .await
        .unwrap();
    let b = state
        .person_repository
        .create(&named("Bristow", "Oliver"), &AuditContext::default())
        .await
        .unwrap();

    let score = state.matcher.match_persons(&a, &b).unwrap().score;
    let configured = state.config.matching.threshold_score;
    assert!(
        score < configured && score > 0.1,
        "the fixture must score under the configured {configured}: {score}"
    );

    let body = json!({ "threshold": score - 0.01, "max_candidates": 1000 });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/persons/deduplicate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let scan: Value = serde_json::from_slice(&bytes).unwrap();
    let (ida, idb) = (a.id.to_string(), b.id.to_string());
    let found = scan["review_items"].as_array().unwrap().iter().any(|i| {
        let ids = [i["person_id_a"].as_str(), i["person_id_b"].as_str()];
        ids.contains(&Some(ida.as_str())) && ids.contains(&Some(idb.as_str()))
    });
    assert!(found, "the pair scoring {score} should be reported: {scan}");
}
