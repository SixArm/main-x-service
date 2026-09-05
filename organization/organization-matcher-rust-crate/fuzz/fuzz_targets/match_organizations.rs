//! SEC-I2 fuzz target: the full matching engine over deserialized input.
//!
//! Interprets the fuzz bytes as a JSON `[organization_a, organization_b]` tuple,
//! then runs `MatchingEngine::match_organizations`. Pins the two load-bearing
//! invariants on arbitrary (attacker-controlled) input — the engine
//! **never panics**, and the score is **finite and within `[0.0, 1.0]`** —
//! across the whole deserialize → normalize → score path. Complements the
//! crate's `proptest` properties with coverage-guided input generation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use organization_matcher::{
    MatchConfig, MatchResult, MatchingEngine, Organization, RelationKind, RelationshipRef,
};

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
    let Ok((mut a, mut b)) = serde_json::from_slice::<(Organization, Organization)>(data) else {
        return;
    };

    let engine = MatchingEngine::new(MatchConfig::default());
    // Run both orderings so either argument position is exercised.
    for result in [
        engine.match_organizations(&a, &b),
        engine.match_organizations(&b, &a),
    ] {
        assert_bounded(&result);
    }

    // ORGM-T2: `relationships`/`tags` had zero presence in this harness
    // despite the deserialize path covering every `Organization` field —
    // random byte mutation rarely happens to produce a populated
    // `Vec<RelationshipRef>`/`Vec<String>` unseeded. Force those field
    // paths reachable on **every** run, not merely a lucky corpus find,
    // by appending a relationship/tag derived from the raw fuzz bytes —
    // via a struct literal rather than `RelationshipRef::new`, so a
    // possibly-empty/malformed `organization_id` that constructor would
    // have rejected still reaches `relationships_score`.
    let text = String::from_utf8_lossy(data).into_owned();
    a.relationships.push(RelationshipRef {
        relation: RelationKind::SuccessorOf,
        organization_id: text.clone(),
    });
    a.tags.push(text.clone());
    b.relationships.push(RelationshipRef {
        relation: RelationKind::PredecessorOf,
        organization_id: text.clone(),
    });
    b.tags.push(text);
    for result in [
        engine.match_organizations(&a, &b),
        engine.match_organizations(&b, &a),
    ] {
        assert_bounded(&result);
    }
});
