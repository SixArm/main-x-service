#![warn(clippy::pedantic)]

//! T-32 integration tests (`agents/share/cross-service-linking.md` §5.2,
//! `link-graph-service-with-loco/spec/16-open-questions.md` OQ-9(b)):
//! a `matcher_suggested` `same_identity` `POST /api/persons/{id}/links`
//! surfaces in person's existing review queue; confirming it promotes
//! the underlying edge to `operator`/`1.0` without duplicating it;
//! rejecting it withdraws the edge; and an **ordinary** (within-entity)
//! review decision is completely unaffected by any of this — the T-32
//! promotion path is gated on both `provenance` and `detection_method`,
//! and this last test is the regression pin proving that gate actually
//! holds.
//!
//! These tests drive the real HTTP router end-to-end
//! (`tower::ServiceExt::oneshot`), exactly like
//! `tests/api_integration_test.rs`, rather than calling the crate's
//! `pub(crate)` promotion/rejection helpers directly — this file is a
//! separate test binary and cannot see `pub(crate)` items, which is
//! itself a fine thing to prove through: the only way to reach this
//! behaviour is the public API, the same door the link-graph aggregator
//! and an operator's review-queue client both use.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

use person_service::{
    api::{ApiResponse, rest::links::LinkView},
    db::review_queue::{self, NewReviewItem},
    models::{Person, ReviewQueueItem, ReviewStatus},
};

async fn create_person(app: &axum::Router, family: &str, given: &str) -> Person {
    let person_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": { "family": family, "given": [given] },
        "birth_date": "1980-06-15",
        "gender": "female",
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice::<ApiResponse<Person>>(&body)
        .unwrap()
        .data
        .unwrap()
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// Fetch a person's active outbound links via the real endpoint.
async fn person_links(app: &axum::Router, person_id: Uuid) -> Vec<LinkView> {
    let (status, body) = get_json(app, &format!("/api/persons/{person_id}/links")).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_value::<ApiResponse<Vec<LinkView>>>(body)
        .unwrap()
        .data
        .unwrap()
}

/// Find the pending T-32 cross-service review row for a given
/// `(person_id, worker_id)` pair.
async fn find_cross_service_review_row(
    app: &axum::Router,
    person_id: Uuid,
    worker_id: Uuid,
) -> ReviewQueueItem {
    let (status, body) = get_json(app, "/api/persons/review-queue?status=pending&limit=500").await;
    assert_eq!(status, StatusCode::OK);
    // `ReviewQueueListResponse` is `Serialize`-only (a response-side wire
    // type); pull its `items` array out of the raw envelope and
    // deserialize each element as a `ReviewQueueItem` directly.
    let items: Vec<ReviewQueueItem> =
        serde_json::from_value(body["data"]["items"].clone()).expect("items array");
    items
        .into_iter()
        .find(|item| {
            item.detection_method == "cross_service_same_identity"
                && item.person_id_a == person_id
                && item.person_id_b == worker_id
        })
        .expect("expected a pending cross-service review-queue row for this pair")
}

/// A `matcher_suggested` `same_identity` link POST produces a
/// review-queue row with the right fields: `record_id_a` = person,
/// `record_id_b` = worker (never reordered — OQ-9(b)), the score/quality
/// carried through, `detection_method = "cross_service_same_identity"`,
/// `provenance = "matcher_suggested"`, `status = "pending"`, and the
/// T-29 score breakdown mapped verbatim.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test cross_service_link_review -- --ignored`"]
async fn matcher_suggested_same_identity_link_creates_a_review_row() {
    let app = common::create_test_router().await;
    let family = common::unique_person_name("XSvcReview");
    let person = create_person(&app, &family, "Jane").await;
    let worker_id = Uuid::new_v4();

    let (status, body) = post_json(
        &app,
        &format!("/api/persons/{}/links", person.id),
        json!({
            "kind": "same_identity",
            "to_ref": format!("worker:{worker_id}"),
            "confidence": 0.87,
            "provenance": "matcher_suggested",
            "score_breakdown": {
                "identifier_match": false,
                "name_score": 0.95,
                "dob_score": 1.0,
                "gender_score": 1.0
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "link create response: {body}");
    let link = serde_json::from_value::<ApiResponse<LinkView>>(body)
        .unwrap()
        .data
        .unwrap();
    assert_eq!(link.provenance, "matcher_suggested");
    assert!((link.confidence.unwrap() - 0.87).abs() < 1e-9);

    let row = find_cross_service_review_row(&app, person.id, worker_id).await;
    assert_eq!(
        row.person_id_a, person.id,
        "record_id_a must be the person pid"
    );
    assert_eq!(
        row.person_id_b, worker_id,
        "record_id_b must be the worker pid"
    );
    assert!((row.match_score - 0.87).abs() < 1e-9);
    assert_eq!(row.match_quality, "probable", "0.87 is >= 0.7 and < 0.95");
    assert_eq!(row.detection_method, "cross_service_same_identity");
    assert_eq!(row.provenance, "matcher_suggested");
    assert_eq!(row.status, ReviewStatus::Pending);
    let breakdown = row
        .score_breakdown
        .expect("score breakdown carried through");
    assert_eq!(breakdown["name_score"], 0.95);
    assert_eq!(breakdown["dob_score"], 1.0);
    assert_eq!(breakdown["gender_score"], 1.0);
    assert_eq!(breakdown["identifier_match"], false);
}

/// Confirming a suggested pair promotes the same edge to
/// `operator`/`1.0` — idempotent on the existing `entity_links` key, so
/// there is still exactly one active edge afterward, not a second one.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test cross_service_link_review -- --ignored`"]
async fn confirming_promotes_the_edge_without_duplicating_it() {
    let app = common::create_test_router().await;
    let family = common::unique_person_name("XSvcConfirm");
    let person = create_person(&app, &family, "Jane").await;
    let worker_id = Uuid::new_v4();

    let (status, _) = post_json(
        &app,
        &format!("/api/persons/{}/links", person.id),
        json!({
            "kind": "same_identity",
            "to_ref": format!("worker:{worker_id}"),
            "confidence": 0.91,
            "provenance": "matcher_suggested",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Sanity: before the decision, the suggested edge is not yet operator/1.0.
    let before = person_links(&app, person.id).await;
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].provenance, "matcher_suggested");

    let row = find_cross_service_review_row(&app, person.id, worker_id).await;
    let (status, body) = post_json(
        &app,
        &format!("/api/persons/review-queue/{}/decision", row.id),
        json!({ "status": "confirmed" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "decision response: {body}");
    let decided = serde_json::from_value::<ApiResponse<ReviewQueueItem>>(body)
        .unwrap()
        .data
        .unwrap();
    assert_eq!(decided.status, ReviewStatus::Confirmed);

    let after = person_links(&app, person.id).await;
    assert_eq!(after.len(), 1, "promotion must not create a second edge");
    assert_eq!(after[0].id, before[0].id, "the SAME edge id, not a new one");
    assert_eq!(after[0].provenance, "operator");
    assert_eq!(after[0].confidence, Some(1.0));
    assert_eq!(after[0].to_ref, format!("worker:{worker_id}"));
}

/// Rejecting a suggested pair withdraws (soft-deletes) the edge.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test cross_service_link_review -- --ignored`"]
async fn rejecting_withdraws_the_edge() {
    let app = common::create_test_router().await;
    let family = common::unique_person_name("XSvcReject");
    let person = create_person(&app, &family, "Jane").await;
    let worker_id = Uuid::new_v4();

    let (status, _) = post_json(
        &app,
        &format!("/api/persons/{}/links", person.id),
        json!({
            "kind": "same_identity",
            "to_ref": format!("worker:{worker_id}"),
            "confidence": 0.8,
            "provenance": "matcher_suggested",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(person_links(&app, person.id).await.len(), 1);

    let row = find_cross_service_review_row(&app, person.id, worker_id).await;
    let (status, body) = post_json(
        &app,
        &format!("/api/persons/review-queue/{}/decision", row.id),
        json!({ "status": "rejected" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "decision response: {body}");
    let decided = serde_json::from_value::<ApiResponse<ReviewQueueItem>>(body)
        .unwrap()
        .data
        .unwrap();
    assert_eq!(decided.status, ReviewStatus::Rejected);

    assert!(
        person_links(&app, person.id).await.is_empty(),
        "rejection must withdraw the edge"
    );
}

/// Regression pin: an ordinary (within-entity) review decision — real
/// `provenance = "operator"`, `detection_method = "batch_deduplication"`
/// row, seeded directly the way the batch dedup scan would — is
/// completely unaffected by T-32. The row's `record_id_a`/`record_id_b`
/// are two REAL persons' ids (not a person/worker pair), so if the T-32
/// promotion gate ever fired for an ordinary row by mistake it would
/// manifest as a spurious `same_identity` edge on person A pointing at
/// `worker:<person B's id>` — exactly what this test's final assertion
/// rules out.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test cross_service_link_review -- --ignored`"]
async fn ordinary_review_decision_is_unaffected_by_t32() {
    let app = common::create_test_router().await;
    let db = common::db().await;

    // Genuinely distinct surnames, not just distinct suffixes on a shared
    // base name: a shared long prefix scores high enough under
    // Jaro-Winkler (plus the identical DOB/gender both `create_person`
    // callers get) that real-time duplicate detection legitimately 409s
    // on the second create — the same pitfall
    // `test_list_persons_paginates_every_created_record_exactly_once`
    // documents in `api_integration_test.rs`.
    let run_suffix = chrono::Utc::now().timestamp_micros();
    let family_a = format!("Winterbourne{run_suffix}");
    let family_b = format!("Castellanos{run_suffix}");
    let person_a = create_person(&app, &family_a, "Alpha").await;
    let person_b = create_person(&app, &family_b, "Bravo").await;

    // Seed an ORDINARY review-queue row exactly the way the batch dedup
    // scan does (`api::rest::handlers::batch_deduplicate`):
    // `detection_method = "batch_deduplication"`,
    // `provenance = "operator"`. This deliberately bypasses the real
    // fuzzy matcher (whose thresholds/auto-merge tier are not what this
    // test is about) and seeds the row directly, the same persistence
    // call the scan itself makes.
    let seeded = review_queue::upsert(
        &db,
        &[NewReviewItem {
            record_id_a: person_a.id,
            record_id_b: person_b.id,
            match_score: 0.8,
            match_quality: "probable".to_string(),
            detection_method: "batch_deduplication".to_string(),
            score_breakdown: None,
            status: "pending".to_string(),
            provenance: "operator".to_string(),
        }],
    )
    .await
    .expect("seed the ordinary review row");
    let row_id = seeded[0].id;

    let (status, body) = post_json(
        &app,
        &format!("/api/persons/review-queue/{row_id}/decision"),
        json!({ "status": "confirmed" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "decision response: {body}");
    let decided = serde_json::from_value::<ApiResponse<ReviewQueueItem>>(body)
        .unwrap()
        .data
        .unwrap();
    assert_eq!(
        decided.status,
        ReviewStatus::Confirmed,
        "an ordinary decision must still work exactly as before T-32"
    );

    // Neither person gained a `same_identity` edge — the T-32 promotion
    // path never ran for this row.
    assert!(
        person_links(&app, person_a.id).await.is_empty(),
        "an ordinary within-entity decision must never write an entity_links edge"
    );
    assert!(
        person_links(&app, person_b.id).await.is_empty(),
        "an ordinary within-entity decision must never write an entity_links edge"
    );
}
