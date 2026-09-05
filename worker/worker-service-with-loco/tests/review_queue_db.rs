//! DB-gated round-trip for the stored review-queue persistence module
//! (`src/db/review_queue.rs`: normalized-pair upsert / list /
//! first-writer-wins decide). T-18: this module carried no test of its
//! own — "byte-identical to the person crate's tested module" (the
//! 2026-07-19 task's acceptance note) is not "tested here", and worker's
//! own copy has in fact since drifted from person's (no `provenance`
//! column, unboxed `DecideOutcome::Decided`), so a worker-specific
//! migration or query regression would go undetected by this crate's
//! own suite without this file.
//!
//! Run with a migrated Postgres (`scripts/test-db.sh up
//! worker/worker-service-with-loco`, then `scripts/ci-check.sh test-db
//! worker/worker-service-with-loco`, or directly):
//!
//! ```text
//! DATABASE_URL=postgres://…/worker_service_test cargo test --test review_queue_db -- --ignored
//! ```

use sea_orm::{ConnectionTrait, Database, Statement};
use uuid::Uuid;
use worker_service::db::review_queue::{self, DecideOutcome, NewReviewItem};

#[tokio::test]
#[ignore = "requires DATABASE_URL to a migrated Postgres"]
async fn review_queue_round_trip() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let db = Database::connect(&url).await.expect("connect");

    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    // Fresh ids each run (SEC-B pattern elsewhere in this crate's
    // suites), but clean up defensively in case a prior run crashed
    // mid-test — scoped to these two ids, never a blanket table wipe.
    let cleanup = || async {
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM review_queue WHERE record_id_a IN ($1, $2) OR record_id_b IN ($1, $2)",
            [a.into(), b.into()],
        ))
        .await
        .expect("cleanup");
    };
    cleanup().await;

    let item = |score: f64| NewReviewItem {
        record_id_a: a,
        record_id_b: b,
        match_score: score,
        match_quality: "probable".to_string(),
        detection_method: "batch_deduplication".to_string(),
        score_breakdown: Some(serde_json::json!({ "name": score })),
        status: "pending".to_string(),
    };

    // First scan inserts; the stored row normalizes the pair order.
    let rows = review_queue::upsert(&db, &[item(0.8)])
        .await
        .expect("insert");
    assert_eq!(rows.len(), 1);
    let id = rows[0].id;
    assert!(rows[0].record_id_a <= rows[0].record_id_b);
    assert_eq!(rows[0].status, "pending");
    assert!(rows[0].score_breakdown.is_some());

    // A re-scan with the pair REVERSED upserts the same row (stable id)
    // and refreshes the score columns.
    let reversed = NewReviewItem {
        record_id_a: b,
        record_id_b: a,
        ..item(0.9)
    };
    let rows = review_queue::upsert(&db, &[reversed])
        .await
        .expect("upsert");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, id);
    assert!((rows[0].match_score - 0.9).abs() < 1e-9);

    // Decide: pending -> confirmed; first writer wins.
    match review_queue::decide(&db, id, "confirmed", Some("tester"))
        .await
        .expect("decide")
    {
        DecideOutcome::Decided(row) => {
            assert_eq!(row.status, "confirmed");
            assert_eq!(row.reviewed_by.as_deref(), Some("tester"));
            assert!(row.reviewed_at.is_some());
        }
        other => panic!("expected Decided, got {other:?}"),
    }
    match review_queue::decide(&db, id, "rejected", None)
        .await
        .expect("second decide")
    {
        DecideOutcome::AlreadyDecided(current) => assert_eq!(current, "confirmed"),
        other => panic!("expected AlreadyDecided, got {other:?}"),
    }
    match review_queue::decide(&db, Uuid::new_v4(), "confirmed", None)
        .await
        .expect("missing id")
    {
        DecideOutcome::NotFound => {}
        other => panic!("expected NotFound, got {other:?}"),
    }

    // A decided row keeps its decision through a re-scan.
    let rows = review_queue::upsert(&db, &[item(0.95)])
        .await
        .expect("re-scan");
    assert_eq!(rows[0].status, "confirmed");

    // List + status filter.
    let all = review_queue::list(&db, None, 100).await.expect("list");
    assert!(all.iter().any(|r| r.id == id));
    assert!(
        review_queue::list(&db, Some("pending"), 100)
            .await
            .expect("filtered")
            .iter()
            .all(|r| r.id != id),
        "the confirmed row must not appear under a pending filter"
    );

    cleanup().await;
}
