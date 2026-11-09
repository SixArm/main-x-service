#![warn(clippy::pedantic)]

//! `POST /api/workers/deduplicate` honours a `threshold` below the
//! matcher's configured one. It filtered through `find_matches`, which
//! drops anything under the configured threshold first, so a request for
//! a lower one (the documented default is `0.7`) changed nothing.
//!
//! `#[ignore]`d — run with
//! `DATABASE_URL=… cargo test --test batch_threshold -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use worker_service::db::AuditContext;
use worker_service::models::{Gender, HumanName, Worker};

fn named(family: &str, given: &str) -> Worker {
    Worker::new(
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
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_pair_under_the_configured_threshold_is_found_at_a_lower_request_threshold() {
    let state = common::create_test_app_state().await;
    let app = common::create_test_router().await;
    let family = format!("Thresholdson{}", uuid::Uuid::new_v4().simple());
    // The same family name, unrelated given names: a plausible pair that
    // scores under the configured threshold on its own. Since T-23 the
    // score is the matcher's, renormalised over what both workers hold,
    // so near-identical given names ("Margaret" / "Margarethe") now score
    // above the threshold (about 0.97); the given names have to differ
    // properly for the pair to sit under it (about 0.71).
    let a = state
        .worker_repository
        .create(&named(&family, "Margaret"), &AuditContext::default())
        .await
        .unwrap();
    let b = state
        .worker_repository
        .create(&named(&family, "Theodora"), &AuditContext::default())
        .await
        .unwrap();

    let score = state.matcher.match_workers(&a, &b).unwrap().score;
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
                .uri("/api/workers/deduplicate")
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
        let ids = [i["worker_id_a"].as_str(), i["worker_id_b"].as_str()];
        ids.contains(&Some(ida.as_str())) && ids.contains(&Some(idb.as_str()))
    });
    assert!(found, "the pair scoring {score} should be reported: {scan}");
}
