//! DB-backed round-trip for the stored review queue
//! (`db::review_queue` upsert / list / decide) — the raw-SQL layer the
//! four registry services share verbatim. Gated on
//! `REVIEW_QUEUE_TEST_DATABASE_URL` pointing at a Postgres the test may
//! create tables in; without it the test exits trivially so default
//! `cargo test` runs stay DB-free.

use person_service::db::review_queue::{self, DecideOutcome, NewReviewItem};
use uuid::Uuid;

#[tokio::test]
async fn review_queue_round_trip() {
    let Ok(url) = std::env::var("REVIEW_QUEUE_TEST_DATABASE_URL") else {
        eprintln!("REVIEW_QUEUE_TEST_DATABASE_URL unset; skipping DB round-trip");
        return;
    };
    use sea_orm::ConnectionTrait;
    let db = sea_orm::Database::connect(&url).await.expect("connect");
    db.execute_unprepared(include_str!(
        "../migrations/2026071900000001_create_review_queue/up.sql"
    ))
    .await
    .expect("apply migration");
    db.execute_unprepared("DELETE FROM review_queue")
        .await
        .expect("clean slate");

    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
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
    assert_eq!(all.len(), 1);
    assert!(
        review_queue::list(&db, Some("pending"), 100)
            .await
            .expect("filtered")
            .is_empty()
    );
}
