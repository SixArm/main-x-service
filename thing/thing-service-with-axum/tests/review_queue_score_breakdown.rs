//! `POST /api/things/deduplicate` persists the matcher's own
//! per-component `score_breakdown`, and `GET /api/things/review-queue`
//! returns it (spec §13 T-12).
//!
//! Before this, `handlers::deduplicate` always wrote
//! `NewReviewItem { score_breakdown: None, .. }` even though the
//! `MatchResult` computed for each pair carries a real breakdown, and
//! the wire type `ReviewQueueItem` had no `score_breakdown` field at
//! all — so the stored `JSONB` column existed but nothing ever wrote or
//! read it.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test review_queue_score_breakdown -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use thing_service::api::rest::{AppState, create_router};
use thing_service::config::Config;
use thing_service::db::{ThingRepository, create_connection};
use thing_service::matching::ProbabilisticMatcher;
use thing_service::models::thing::Thing;
use thing_service::search::SearchEngine;
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index. Same construction the sibling `tests/*.rs`
/// integration binaries use.
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

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

/// True when a review item's pair (order-insensitive) is exactly `{a, b}`.
/// Selecting by pair membership rather than by response array position,
/// since the test database is shared across every test binary in a
/// `test-db` run and a batch scan sees every active row, not just this
/// test's own two.
fn is_pair(item: &Value, a: &str, b: &str) -> bool {
    let ia = item["thing_id_a"].as_str().unwrap_or_default();
    let ib = item["thing_id_b"].as_str().unwrap_or_default();
    (ia == a && ib == b) || (ia == b && ib == a)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test review_queue_score_breakdown -- --ignored"]
async fn deduplicate_persists_and_review_queue_returns_the_score_breakdown() {
    let app = test_router().await;

    // Seed two near-duplicate things directly through the repository
    // (bypassing `POST /api/things`, whose own real-time duplicate check
    // would reject the second one with 409 before this test ever gets to
    // exercise the batch scan). Same unique-per-run name/description
    // pair, so the scan finds exactly this pair regardless of what any
    // other test binary already seeded into the shared database — and no
    // shared identifier, so the deterministic short-circuit does not
    // fire and a real weighted per-component breakdown is computed.
    let config = Config::from_env().expect("load config from env");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    let repo = thing_service::db::SeaOrmThingRepository::new(db);
    let unique = uuid::Uuid::new_v4();
    let name = format!("Score Breakdown Fixture {unique}");
    let mut a = Thing::new(&name);
    a.description = Some("A novel by Jane Austen".to_string());
    let mut b = Thing::new(&name);
    b.description = Some("A novel by Jane Austen".to_string());
    let stored_a = repo.create(&a).await.expect("seed thing A");
    let stored_b = repo.create(&b).await.expect("seed thing B");
    let id_a = stored_a.id.to_string();
    let id_b = stored_b.id.to_string();

    // A low threshold so this exact-name/description pair is guaranteed
    // to be found regardless of the default matcher threshold.
    let (status, body) = post(
        &app,
        "/api/things/deduplicate",
        &json!({ "threshold": 0.5, "max_candidates": 1000 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let review_items = body["data"]["review_items"].as_array().unwrap();
    let scanned = review_items
        .iter()
        .find(|i| is_pair(i, &id_a, &id_b))
        .unwrap_or_else(|| panic!("expected our seeded pair among the scan results: {body}"));
    let breakdown = &scanned["score_breakdown"];
    assert!(
        !breakdown.is_null(),
        "deduplicate's own response should already carry a breakdown: {body}"
    );
    // Exact-match name/description ⇒ both component scores are 1.0.
    assert_eq!(breakdown["name_score"], 1.0, "{breakdown}");
    assert_eq!(breakdown["description_score"], 1.0, "{breakdown}");

    // The stored review queue — a fresh GET, not the scan response —
    // carries the same breakdown, proving it was actually persisted.
    let (status, body) = get(&app, "/api/things/review-queue?limit=500").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let items = body["data"]["items"].as_array().unwrap();
    let stored = items
        .iter()
        .find(|i| is_pair(i, &id_a, &id_b))
        .unwrap_or_else(|| panic!("expected our pair in the stored review queue: {body}"));
    let stored_breakdown = &stored["score_breakdown"];
    assert!(
        !stored_breakdown.is_null(),
        "GET /review-queue's score_breakdown must be non-null: {body}"
    );
    assert_eq!(
        stored_breakdown["name_score"], 1.0,
        "stored breakdown must match the matcher's own component scores: {stored_breakdown}"
    );
    assert_eq!(
        stored_breakdown["description_score"], 1.0,
        "{stored_breakdown}"
    );
}
