//! SEC-I2 fuzz target: the full matching engine over deserialized input.
//!
//! Interprets the fuzz bytes as a JSON `[care_pathway_a, care_pathway_b]` tuple,
//! then runs `MatchingEngine::match_care_pathways`. Pins the two load-bearing
//! invariants on arbitrary (attacker-controlled) input — the engine
//! **never panics**, and the score is **finite and within `[0.0, 1.0]`** —
//! across the whole deserialize → normalize → score path. Complements the
//! crate's `proptest` properties with coverage-guided input generation.

#![no_main]

use care_pathway_matcher::{
    CarePathway, MatchConfig, MatchResult, MatchingEngine, RelationKind, RelationshipRef,
};
use libfuzzer_sys::fuzz_target;

/// The two load-bearing invariants: never panics, score stays finite and
/// within `[0.0, 1.0]`.
fn assert_bounded(result: &MatchResult) {
    assert!(
        result.score.is_finite(),
        "score must be finite, got {}",
        result.score
    );
    assert!(
        (0.0..=1.0).contains(&result.score),
        "score must be in [0,1], got {}",
        result.score
    );
}

fuzz_target!(|data: &[u8]| {
    // Only well-formed JSON tuples reach the engine; malformed input is a
    // no-op (the service's HTTP layer rejects it long before the matcher).
    let Ok((mut a, mut b)) = serde_json::from_slice::<(CarePathway, CarePathway)>(data) else {
        return;
    };

    let engine = MatchingEngine::new(MatchConfig::default());
    // Run both orderings so either argument position is exercised.
    for result in [
        engine.match_care_pathways(&a, &b),
        engine.match_care_pathways(&b, &a),
    ] {
        assert_bounded(&result);
    }

    // CPM-T2: `relationships`/`tags` had zero presence in this harness
    // despite the deserialize path covering every `CarePathway` field —
    // random byte mutation rarely happens to produce a populated
    // `Vec<RelationshipRef>`/`Vec<String>` unseeded. Force those field
    // paths reachable on **every** run, not merely a lucky corpus find,
    // by appending a relationship/tag derived from the raw fuzz bytes
    // (`RelationshipRef` has no validating constructor to route around).
    let text = String::from_utf8_lossy(data).into_owned();
    a.relationships.push(RelationshipRef {
        relation: RelationKind::Supersedes,
        pathway_id: text.clone(),
    });
    a.tags.push(text.clone());
    b.relationships.push(RelationshipRef {
        relation: RelationKind::SupersededBy,
        pathway_id: text.clone(),
    });
    b.tags.push(text);
    for result in [
        engine.match_care_pathways(&a, &b),
        engine.match_care_pathways(&b, &a),
    ] {
        assert_bounded(&result);
    }
});
