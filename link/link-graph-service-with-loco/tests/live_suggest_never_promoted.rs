//! Live governance capstone (T-33, closing LNK-4): even a **near-ceiling**
//! identifier-match score is never auto-promoted. A shared coded
//! identifier drives [`super::compare_identity`]'s deterministic
//! short-circuit to [`link_graph_service::suggest::IDENTIFIER_MATCH_CEILING`]
//! (`0.99`) — the strongest evidence this comparator can produce, well
//! above the family's own within-entity `auto_merge_threshold` (`0.95`,
//! `person-service-with-loco::models::review_queue::BatchDeduplicationRequest`).
//! OQ-9(a) pins that **no such auto-merge tier exists for cross-service
//! identity**: this test proves that live, against this crate's real
//! production code, rather than trusting the doc comment.
//!
//! Drives the real fetch→block→compare→POST pipeline against **one** real
//! running person-service (`HttpIdentitySource` fetch,
//! `HttpSuggestionSink` write) paired with a **synthetic** worker-side
//! `IdentityProbe` constructed in-test. A live worker-service is not
//! needed to prove this invariant: the property under test — that the
//! resulting edge stays `matcher_suggested` (never `operator`/`1.0`) and
//! the review-queue row stays `pending` (never `confirmed`/`automerged`)
//! — lives entirely on person's write/review-queue side, which the
//! worker side never touches. `tests/live_suggest_full_pipeline.rs`
//! remains the two-real-services test for the fetch side itself.
//!
//! `#[ignore]`d and not part of any automated CI stage — see
//! `tests/live_suggest_fetch.rs`'s module docs for the general pattern.
//!
//! ```sh
//! # 1. Seed a person carrying a coded identifier, e.g.:
//! curl -s -X POST http://127.0.0.1:5151/api/persons \
//!   -H 'content-type: application/json' \
//!   -d '{
//!     "id": "00000000-0000-0000-0000-000000000000",
//!     "name": { "family": "Governance", "given": ["Test"] },
//!     "birth_date": "1980-06-15", "gender": "female",
//!     "identifiers": [{
//!       "identifier_type": "NHS",
//!       "system": "https://fhir.nhs.uk/Id/nhs-number",
//!       "value": "GOV1234567"
//!     }]
//!   }'
//! # note the returned "id"
//!
//! # 2. Run this test against it:
//! LIVE_PERSON_URL=http://127.0.0.1:5151/api/persons \
//! LIVE_PERSON_ID=<the id from step 1> \
//! LIVE_IDENTIFIER_SYSTEM=https://fhir.nhs.uk/Id/nhs-number \
//! LIVE_IDENTIFIER_VALUE=GOV1234567 \
//!   cargo test --test live_suggest_never_promoted -- --ignored --nocapture
//! ```

use async_trait::async_trait;
use entity_ref::{EntityRef, EntityType};
use link_graph_service::suggest::job::{
    DEFAULT_MAX_EDGES_PER_RUN, HttpIdentitySource, HttpSuggestionSink, IdentitySource,
    run_suggestion_pass,
};
use link_graph_service::suggest::{
    DEFAULT_MAX_CANDIDATES, IDENTIFIER_MATCH_CEILING, IdentityProbe, ProbeIdentifier, ProbeName,
};
use loco_rs::prelude::ModelResult;
use uuid::Uuid;

/// A fixed single-worker source carrying a synthetic probe that shares
/// the seeded person's coded identifier — the "synthetic worker, real
/// person" shape this test needs (see module docs for why a live
/// worker-service is not required here).
struct FixedWorkerSource(EntityRef, IdentityProbe);

#[async_trait]
impl IdentitySource for FixedWorkerSource {
    fn label(&self) -> &'static str {
        "worker (synthetic, in-test)"
    }

    async fn fetch_all(&self) -> ModelResult<Vec<(EntityRef, IdentityProbe)>> {
        Ok(vec![(self.0, self.1.clone())])
    }
}

#[tokio::test]
#[ignore = "requires one real running person-service with a seeded identifier-bearing person; see module docs"]
async fn near_ceiling_identifier_match_is_never_auto_promoted() {
    let person_url = std::env::var("LIVE_PERSON_URL")
        .expect("set LIVE_PERSON_URL, e.g. http://127.0.0.1:5151/api/persons");
    let person_id: Uuid = std::env::var("LIVE_PERSON_ID")
        .expect("set LIVE_PERSON_ID to the seeded person's id")
        .parse()
        .expect("LIVE_PERSON_ID must be a UUID");
    let identifier_system = std::env::var("LIVE_IDENTIFIER_SYSTEM")
        .unwrap_or_else(|_| "https://fhir.nhs.uk/Id/nhs-number".to_string());
    let identifier_value =
        std::env::var("LIVE_IDENTIFIER_VALUE").expect("set LIVE_IDENTIFIER_VALUE");

    let persons = HttpIdentitySource::new(EntityType::Person, person_url.clone(), None);
    let worker_ref = EntityRef::new(EntityType::Worker, Uuid::new_v4());
    let worker_probe = IdentityProbe {
        name: Some(ProbeName {
            family: "Governance".to_string(),
            given: "Sibling".to_string(),
        }),
        birth_date: chrono::NaiveDate::from_ymd_opt(1980, 6, 15),
        gender: Some(person_matcher::Gender::Female),
        identifiers: vec![
            ProbeIdentifier::new(&identifier_system, &identifier_value)
                .expect("identifier system/value must be non-blank"),
        ],
    };
    let workers = FixedWorkerSource(worker_ref, worker_probe);
    let sink = HttpSuggestionSink::new(person_url.clone(), None);

    let stats = run_suggestion_pass(
        &persons,
        &workers,
        &sink,
        DEFAULT_MAX_CANDIDATES,
        DEFAULT_MAX_EDGES_PER_RUN,
    )
    .await
    .expect("full pipeline run against the real person-service");
    println!("near_ceiling_identifier_match_is_never_auto_promoted: pass 1: {stats:?}");

    assert!(
        stats.candidates > 0,
        "expected the seeded person to match the synthetic worker on the shared identifier"
    );
    assert!(stats.posted > 0, "expected a successful POST");
    assert_eq!(stats.failed, 0, "no POST should have failed");

    // The edge landed at the identifier ceiling (0.99), matcher_suggested —
    // never operator/1.0, no matter how close to certain the score is.
    let links_url = format!("{person_url}/{person_id}/links");
    let body: serde_json::Value = reqwest::get(&links_url)
        .await
        .expect("GET the seeded person's links")
        .error_for_status()
        .expect("links GET should be 200")
        .json()
        .await
        .expect("parse links response");
    let links = body["data"].as_array().expect("data is an array of links");
    let edge = links
        .iter()
        .find(|l| l["kind"] == "same_identity" && l["to_ref"] == worker_ref.to_string())
        .unwrap_or_else(|| {
            panic!(
                "expected a same_identity edge to {worker_ref} on person {person_id}, got: {body}"
            )
        });
    assert_eq!(
        edge["provenance"], "matcher_suggested",
        "must never be operator-provenance without an explicit confirm decision"
    );
    let confidence = edge["confidence"].as_f64().expect("confidence is a number");
    assert!(
        (confidence - IDENTIFIER_MATCH_CEILING).abs() < 1e-9,
        "expected the identifier ceiling {IDENTIFIER_MATCH_CEILING}, got {confidence}"
    );
    assert!(
        confidence < 1.0,
        "a suggestion must never reach 1.0 confidence without operator confirmation"
    );

    // The capstone assertion: the review-queue row is `pending` — never
    // `confirmed` / `automerged` — even though 0.99 is well above the
    // family's own within-entity auto_merge_threshold (0.95). OQ-9(a)
    // pins that no such auto-merge tier exists for cross-service identity.
    let review_url = format!(
        "{}/persons/review-queue?status=pending&limit=500",
        person_url.trim_end_matches("/persons")
    );
    let review_body: serde_json::Value = reqwest::get(&review_url)
        .await
        .expect("GET the review queue")
        .error_for_status()
        .expect("review-queue GET should be 200")
        .json()
        .await
        .expect("parse review-queue response");
    let items = review_body["data"]["items"]
        .as_array()
        .expect("data.items is an array");
    let row = items
        .iter()
        .find(|item| {
            item["detection_method"] == "cross_service_same_identity"
                && item["person_id_a"] == person_id.to_string()
        })
        .unwrap_or_else(|| {
            panic!(
                "expected a PENDING cross-service review row for {person_id}, got: {review_body}"
            )
        });
    assert_eq!(
        row["status"], "pending",
        "a 0.99 identifier-ceiling match must still be pending, never auto-promoted"
    );
    assert!(
        (row["match_score"].as_f64().unwrap() - IDENTIFIER_MATCH_CEILING).abs() < 1e-9,
        "the review row's score must be the identifier ceiling, not 1.0"
    );

    // Re-running the pass a second time (idempotent fetch + upsert) must
    // not change any of this — proving there is no background path that
    // promotes a pending suggestion on its own over time or repetition.
    // Only an explicit operator `POST .../review-queue/{id}/decision`
    // call promotes it — already regression-pinned on person's own side
    // by `cross_service_link_review.rs`'s
    // `confirming_promotes_the_edge_without_duplicating_it` test, which
    // this test deliberately does not call.
    let stats2 = run_suggestion_pass(
        &persons,
        &workers,
        &sink,
        DEFAULT_MAX_CANDIDATES,
        DEFAULT_MAX_EDGES_PER_RUN,
    )
    .await
    .expect("second pass also succeeds");
    println!("near_ceiling_identifier_match_is_never_auto_promoted: pass 2: {stats2:?}");
    assert_eq!(
        stats2.posted, 1,
        "idempotent upsert: the same edge is reasserted, not duplicated"
    );

    let review_body_after: serde_json::Value = reqwest::get(&review_url)
        .await
        .expect("GET the review queue again")
        .error_for_status()
        .expect("review-queue GET should be 200")
        .json()
        .await
        .expect("parse review-queue response");
    let items_after = review_body_after["data"]["items"]
        .as_array()
        .expect("data.items is an array");
    let row_after = items_after
        .iter()
        .find(|item| {
            item["detection_method"] == "cross_service_same_identity"
                && item["person_id_a"] == person_id.to_string()
        })
        .expect("still present and pending after a second identical pass");
    assert_eq!(
        row_after["status"], "pending",
        "a second identical suggestion pass must not have silently promoted the pending row"
    );
}
