//! Live, full-pipeline proof for T-31 (fetch → block → compare → POST)
//! plus T-32's review-queue bridge, exercising this crate's real
//! production code end to end against two real, separately-running peer
//! services — not a mock. Complements `tests/live_suggest_fetch.rs`
//! (fetch-only, either service alone) by also driving the real
//! `HttpSuggestionSink` POST back to person, and — since T-32 — checking
//! that the POST's `score_breakdown` field lands on person's own
//! `review_queue` row (`agents/share/cross-service-linking.md` §5.2,
//! `spec/16-open-questions.md` OQ-9(b)), not merely that the edge itself
//! was created.
//!
//! `#[ignore]`d and not part of any automated CI stage — see module docs
//! in `live_suggest_fetch.rs` for the general pattern. This test
//! additionally requires the two running services to share at least one
//! person/worker pair with matching identity data (e.g. the same coded
//! identifier), so `generate_candidates` actually produces something to
//! POST — seed one before running, then pass its ids so the test can
//! verify the specific edge landed (rather than merely that the run
//! didn't error).
//!
//! ```sh
//! LIVE_PERSON_URL=http://127.0.0.1:5151/api/persons \
//! LIVE_WORKER_URL=http://127.0.0.1:5150/api/workers \
//! LIVE_EXPECT_PERSON_ID=<uuid of a seeded matching person> \
//!   cargo test --test live_suggest_full_pipeline -- --ignored --nocapture
//! ```

use entity_ref::EntityType;
use link_graph_service::suggest::DEFAULT_MAX_CANDIDATES;
use link_graph_service::suggest::job::{
    DEFAULT_MAX_EDGES_PER_RUN, HttpIdentitySource, HttpSuggestionSink, run_suggestion_pass,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires two real running services sharing a matching person/worker pair; see module docs"]
async fn live_pipeline_posts_a_real_suggested_edge() {
    // Skip rather than panic when the target is not configured — see the
    // sibling note in `live_suggest_fetch.rs`: `#[ignore]` does not keep
    // these out of CI, because `ci-check.sh test-db` runs
    // `cargo test -- --ignored`. Absent configuration means "not this
    // run", not "broken".
    let (Ok(person_url), Ok(worker_url), Ok(expect_id)) = (
        std::env::var("LIVE_PERSON_URL"),
        std::env::var("LIVE_WORKER_URL"),
        std::env::var("LIVE_EXPECT_PERSON_ID"),
    ) else {
        eprintln!(
            "skipping: set LIVE_PERSON_URL, LIVE_WORKER_URL and \
             LIVE_EXPECT_PERSON_ID, and run both peer services first — \
             see this file's module docs"
        );
        return;
    };
    let expect_person_id: Uuid = expect_id
        .parse()
        .expect("LIVE_EXPECT_PERSON_ID must be a UUID");

    let persons = HttpIdentitySource::new(EntityType::Person, person_url.clone(), None);
    let workers = HttpIdentitySource::new(EntityType::Worker, worker_url, None);
    let sink = HttpSuggestionSink::new(person_url.clone(), None);

    let stats = run_suggestion_pass(
        &persons,
        &workers,
        &sink,
        DEFAULT_MAX_CANDIDATES,
        DEFAULT_MAX_EDGES_PER_RUN,
    )
    .await
    .expect("full pipeline run against real services");
    println!("live_pipeline_posts_a_real_suggested_edge: {stats:?}");

    assert!(
        stats.candidates > 0,
        "expected at least one candidate — is the seeded pair a real match?"
    );
    assert!(stats.posted > 0, "expected at least one successful POST");
    assert_eq!(stats.failed, 0, "no POST should have failed");

    // Confirm the edge actually landed on person's own `entity_links` —
    // not just that the sink's POST returned 2xx.
    let links_url = format!("{person_url}/{expect_person_id}/links");
    let response = reqwest::get(&links_url)
        .await
        .expect("GET the seeded person's links")
        .error_for_status()
        .expect("links GET should be 200");
    let body: serde_json::Value = response.json().await.expect("parse links response");
    let links = body["data"].as_array().expect("data is an array of links");
    let has_same_identity = links
        .iter()
        .any(|l| l["kind"] == "same_identity" && l["provenance"] == "matcher_suggested");
    assert!(
        has_same_identity,
        "expected a matcher_suggested same_identity link on person {expect_person_id}, got: {body}"
    );

    // T-32: the same POST also queued a review-queue row on person, with
    // the T-29 score breakdown carried through — not just the edge.
    // `person_url` is the collection base (e.g. `.../api/persons`); the
    // review-queue endpoint is a sibling path under the same prefix.
    let review_url = format!(
        "{}/persons/review-queue?status=pending&limit=500",
        person_url.trim_end_matches("/persons")
    );
    let response = reqwest::get(&review_url)
        .await
        .expect("GET the review queue")
        .error_for_status()
        .expect("review-queue GET should be 200");
    let body: serde_json::Value = response.json().await.expect("parse review-queue response");
    let items = body["data"]["items"]
        .as_array()
        .expect("data.items is an array");
    let row = items
        .iter()
        .find(|item| {
            item["detection_method"] == "cross_service_same_identity"
                && item["person_id_a"] == expect_person_id.to_string()
        })
        .unwrap_or_else(|| {
            panic!("expected a pending cross-service review-queue row for {expect_person_id}, got: {body}")
        });
    assert_eq!(row["provenance"], "matcher_suggested");
    assert!(
        row["score_breakdown"].is_object(),
        "expected the T-29 score breakdown to have landed on the review-queue row, got: {row}"
    );
}
