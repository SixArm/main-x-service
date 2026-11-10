//! `GET /api/organizations/review-queue` pages by signed cursor: following
//! `Link: rel="next"` reaches every stored pair, each once, even past the
//! 500 rows one page may hold; a `limit` above the maximum and a cursor the
//! service did not sign are each a `422`. `#[ignore]`d on PostgreSQL like
//! the rest of the suite.

use std::collections::HashMap;

use axa_test::prelude::*;
use organization_service::app::App;
use organization_service::models::review_queue::{NewReviewItem, upsert};
use serial_test::serial;
use uuid::Uuid;

/// More than one maximum page, so the walk must follow a cursor.
const SEEDED: usize = 520;

/// The `next` target of a `Link` header, if there is one.
fn next_of(link: Option<axum::http::HeaderValue>) -> Option<String> {
    link.and_then(|v| v.to_str().ok().map(str::to_owned))
        .and_then(|link| {
            link.strip_prefix('<')
                .and_then(|l| l.split_once('>'))
                .map(|(target, _)| target.to_owned())
        })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn following_next_reaches_every_pair_past_one_page() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let items: Vec<NewReviewItem> = (0..SEEDED)
            .map(|_| {
                let (x, y) = (Uuid::new_v4(), Uuid::new_v4());
                let (a, b) = if x <= y { (x, y) } else { (y, x) };
                NewReviewItem {
                    record_id_a: a,
                    record_id_b: b,
                    match_score: 0.9,
                    match_quality: "probable".into(),
                    detection_method: "review_queue_paging".into(),
                    score_breakdown: None,
                    status: "pending".into(),
                    provenance: "operator".into(),
                    evidence: None,
                }
            })
            .collect();
        let seeded: Vec<String> = items.iter().map(|i| i.record_id_a.to_string()).collect();
        upsert(&ctx.db, &items)
            .await
            .expect("seed the review queue");

        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut pages = 0;
        let mut next = Some("/api/organizations/review-queue?status=pending&limit=500".to_owned());
        while let Some(uri) = next.take() {
            let response = request.get(&uri).await;
            assert_eq!(response.status_code(), 200, "{uri}");
            next = next_of(response.maybe_header("link"));
            let body: serde_json::Value = response.json();
            let page = body["items"].as_array().expect("items");
            assert!(page.len() <= 500, "a page holds at most 500");
            assert_eq!(body["total"].as_u64(), Some(page.len() as u64));
            for item in page {
                let a = item["organization_id_a"].as_str().unwrap().to_owned();
                *seen.entry(a).or_default() += 1;
            }
            pages += 1;
        }

        assert!(pages >= 2, "{SEEDED} rows need at least two pages of 500");
        for id in &seeded {
            assert_eq!(seen.get(id), Some(&1), "pair {id} seen once");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn a_limit_above_the_maximum_or_a_forged_cursor_is_a_422() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for uri in [
            "/api/organizations/review-queue?limit=501",
            "/api/organizations/review-queue?limit=0",
            "/api/organizations/review-queue?cursor=AAAA",
        ] {
            assert_eq!(request.get(uri).await.status_code(), 422, "{uri}");
        }
    })
    .await;
}
