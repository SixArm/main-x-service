//! Match evidence through the service (spec §13 T-18,
//! `agents/share/match-evidence.md`): the batch scan stores and returns
//! each pair's evidence, auto-merge requires `proven` evidence, and the
//! `check-duplicates` and create-time `409` bodies carry it.
//!
//! `#[ignore]`d — run explicitly with
//! `DB_URL=… cargo test --test review_queue_evidence -- --ignored`.

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use thing_service::app::App;
use thing_service::config::Config;
use thing_service::db::{PgThingRepository, ThingRepository, create_connection};
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;
use tower::ServiceExt;

async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    App::boot_test()
        .await
        .expect("boot the app: set DB_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
}

async fn repo() -> PgThingRepository {
    let config = Config::from_env().expect("load config from env");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    PgThingRepository::new(db)
}

async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn is_pair(item: &Value, a: &str, b: &str) -> bool {
    let ia = item["thing_id_a"].as_str().unwrap_or_default();
    let ib = item["thing_id_b"].as_str().unwrap_or_default();
    (ia == a && ib == b) || (ia == b && ib == a)
}

/// Seed `a` and `b` through the repository (bypassing the create-time
/// 409), scan, and return the pair's item from the scan response and
/// from a fresh `GET /review-queue`.
async fn scan_pair(app: &axum::Router, a: Thing, b: Thing, scan: Value) -> (Value, Value) {
    let repo = repo().await;
    let a = repo.create(&a).await.expect("seed A").id.to_string();
    let b = repo.create(&b).await.expect("seed B").id.to_string();
    let (status, body) = send(app, "POST", "/api/things/deduplicate", Some(&scan)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let scanned = body["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| is_pair(i, &a, &b))
        .cloned()
        .unwrap_or_else(|| panic!("seeded pair missing from the scan: {body}"));
    let (status, body) = send(app, "GET", "/api/things/review-queue?limit=500", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stored = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| is_pair(i, &a, &b))
        .cloned()
        .unwrap_or_else(|| panic!("seeded pair missing from the stored queue: {body}"));
    (scanned, stored)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DB_URL=… cargo test --test review_queue_evidence -- --ignored"]
async fn a_name_only_pair_above_the_auto_merge_threshold_stays_pending_and_consistent() {
    let app = test_router().await;
    let name = format!("Evidence Name Only {}", uuid::Uuid::new_v4());
    let (scanned, stored) = scan_pair(
        &app,
        Thing::new(&name),
        Thing::new(&name),
        json!({ "threshold": 0.5, "max_candidates": 1000, "auto_merge_threshold": 0.95 }),
    )
    .await;
    for item in [&scanned, &stored] {
        assert!(item["match_score"].as_f64().unwrap() >= 0.95, "{item}");
        assert_eq!(item["status"], "pending", "{item}");
        assert_eq!(item["evidence"]["strength"], "consistent", "{item}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DB_URL=… cargo test --test review_queue_evidence -- --ignored"]
async fn a_shared_unique_identifier_is_automerged_and_proven() {
    let app = test_router().await;
    let unique = uuid::Uuid::new_v4();
    let doi = format!("10.1000/evidence-{unique}");
    let mut a = Thing::new(&format!("Proven Fixture {unique}"));
    a.identifiers = vec![ThingIdentifier::doi(&doi)];
    let mut b = Thing::new(&format!("Proven Fixture Reprint {unique}"));
    b.identifiers = vec![ThingIdentifier::doi(&doi)];
    let (scanned, stored) = scan_pair(
        &app,
        a,
        b,
        json!({ "threshold": 0.5, "max_candidates": 1000 }),
    )
    .await;
    for item in [&scanned, &stored] {
        assert_eq!(item["status"], "automerged", "{item}");
        assert_eq!(item["evidence"]["strength"], "proven", "{item}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DB_URL=… cargo test --test review_queue_evidence -- --ignored"]
async fn conflicting_identifiers_stay_pending_and_contradicted() {
    let app = test_router().await;
    let unique = uuid::Uuid::new_v4();
    let name = format!("Contradicted Fixture {unique}");
    let mut a = Thing::new(&name);
    a.identifiers = vec![ThingIdentifier::doi(&format!("10.1000/a-{unique}"))];
    let mut b = Thing::new(&name);
    b.identifiers = vec![ThingIdentifier::doi(&format!("10.1000/b-{unique}"))];
    // An auto-merge threshold under the pair's score, so only the
    // evidence gate keeps it out of `automerged`.
    let (scanned, stored) = scan_pair(
        &app,
        a,
        b,
        json!({ "threshold": 0.5, "max_candidates": 1000, "auto_merge_threshold": 0.5 }),
    )
    .await;
    for item in [&scanned, &stored] {
        assert!(item["match_score"].as_f64().unwrap() >= 0.5, "{item}");
        assert_eq!(item["status"], "pending", "{item}");
        assert_eq!(item["evidence"]["strength"], "contradicted", "{item}");
        assert_eq!(
            item["evidence"]["contradictions"][0]["field"], "identifier:doi",
            "{item}"
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DB_URL=… cargo test --test review_queue_evidence -- --ignored"]
async fn check_duplicates_and_the_409_body_carry_evidence() {
    let app = test_router().await;
    let name = format!("Evidence Probe {}", uuid::Uuid::new_v4());
    let thing = json!({ "name": name });
    let (status, body) = send(&app, "POST", "/api/things", Some(&thing)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, body) = send(
        &app,
        "POST",
        "/api/things/check-duplicates?mask_sensitive=true",
        Some(&thing),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["duplicates_found"], true, "{body}");
    let candidate = &body["candidates"][0];
    assert_eq!(candidate["evidence"]["strength"], "consistent", "{body}");
    assert!(candidate["evidence"]["weight_bits"].is_number(), "{body}");

    let (status, body) = send(&app, "POST", "/api/things", Some(&thing)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(
        body["errors"][0]["evidence"]["strength"], "consistent",
        "{body}"
    );
}
