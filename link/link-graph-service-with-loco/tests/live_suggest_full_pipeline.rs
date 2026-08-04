//! Live, full-pipeline proof for T-31 (fetch → block → compare → POST),
//! exercising this crate's real production code end to end against two
//! real, separately-running peer services — not a mock. Complements
//! `tests/live_suggest_fetch.rs` (fetch-only, either service alone) by
//! also driving the real `HttpSuggestionSink` POST back to person.
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
use link_graph_service::suggest::job::{
    HttpIdentitySource, HttpSuggestionSink, run_suggestion_pass,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires two real running services sharing a matching person/worker pair; see module docs"]
async fn live_pipeline_posts_a_real_suggested_edge() {
    let person_url = std::env::var("LIVE_PERSON_URL")
        .expect("set LIVE_PERSON_URL, e.g. http://127.0.0.1:5151/api/persons");
    let worker_url = std::env::var("LIVE_WORKER_URL")
        .expect("set LIVE_WORKER_URL, e.g. http://127.0.0.1:5150/api/workers");
    let expect_person_id: Uuid = std::env::var("LIVE_EXPECT_PERSON_ID")
        .expect("set LIVE_EXPECT_PERSON_ID to the seeded matching person's id")
        .parse()
        .expect("LIVE_EXPECT_PERSON_ID must be a UUID");

    let persons = HttpIdentitySource::new(EntityType::Person, person_url.clone(), None);
    let workers = HttpIdentitySource::new(EntityType::Worker, worker_url, None);
    let sink = HttpSuggestionSink::new(person_url.clone(), None);

    let stats = run_suggestion_pass(&persons, &workers, &sink)
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
}
