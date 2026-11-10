#![warn(clippy::pedantic)]

//! `GET /api/things/review-queue` pages by signed cursor (spec §13 T-19):
//! following `Link: rel="next"` reaches every stored pair, each once, even
//! past the 500 rows a single page may hold; a `limit` above the maximum
//! and a cursor the service did not sign are each a `422`.
//!
//! The run seeds pairs of random ids through the repository, so it can
//! find exactly its own rows in a shared database.
//!
//! `#[ignore]`d — run explicitly with
//! `DB_URL=… cargo test --test review_queue_paging -- --ignored`.

use std::collections::HashMap;

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use thing_service::app::App;
use thing_service::config::Config;
use thing_service::db::create_connection;
use thing_service::db::review_queue::{NewReviewItem, upsert};
use tower::ServiceExt;
use uuid::Uuid;

/// More than one maximum page, so the walk must follow a cursor.
const SEEDED: usize = 520;

async fn app() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    App::boot_test()
        .await
        .expect("boot the app: set DB_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
}

/// Seed `SEEDED` pending pairs, returning their `record_id_a` values,
/// which are unique to this run.
async fn seed() -> Vec<String> {
    let config = Config::from_env().expect("load config from env");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    let items: Vec<NewReviewItem> = (0..SEEDED)
        .map(|_| {
            // Order the pair so `record_id_a` is the one stored as `a`.
            let (a, b) = {
                let (x, y) = (Uuid::new_v4(), Uuid::new_v4());
                if x <= y { (x, y) } else { (y, x) }
            };
            NewReviewItem {
                record_id_a: a,
                record_id_b: b,
                match_score: 0.9,
                match_quality: "probable".into(),
                detection_method: "review_queue_paging".into(),
                score_breakdown: None,
                evidence: None,
                status: "pending".into(),
            }
        })
        .collect();
    let ids = items.iter().map(|i| i.record_id_a.to_string()).collect();
    upsert(&db, &items).await.expect("seed the review queue");
    ids
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Option<String>, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let next = response
        .headers()
        .get(header::LINK)
        .and_then(|v| v.to_str().ok())
        .and_then(|link| link.strip_prefix('<'))
        .and_then(|link| link.split_once('>'))
        .map(|(target, _)| target.to_owned());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        next,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn following_next_reaches_every_pair_past_one_page() {
    let app = app().await;
    let seeded = seed().await;

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut pages = 0;
    let mut next = Some("/api/things/review-queue?status=pending&limit=500".to_owned());
    while let Some(uri) = next.take() {
        let (status, link, body) = get(&app, &uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        let items = body["items"].as_array().expect("items");
        assert!(
            items.len() <= 500,
            "a page holds at most 500: {}",
            items.len()
        );
        assert_eq!(body["total"].as_u64(), Some(items.len() as u64));
        for item in items {
            let a = item["thing_id_a"].as_str().unwrap().to_owned();
            *seen.entry(a).or_default() += 1;
        }
        pages += 1;
        next = link;
    }

    assert!(pages >= 2, "{SEEDED} rows need at least two pages of 500");
    for id in &seeded {
        assert_eq!(
            seen.get(id),
            Some(&1),
            "pair {id} seen once across the walk"
        );
    }
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn a_limit_above_the_maximum_or_a_forged_cursor_is_a_422() {
    let app = app().await;
    let (status, _, body) = get(&app, "/api/things/review-queue?limit=501").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "invalid_page", "{body}");

    let (status, _, body) = get(&app, "/api/things/review-queue?cursor=AAAA").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let (status, _, body) = get(&app, "/api/things/review-queue?limit=0").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
}
