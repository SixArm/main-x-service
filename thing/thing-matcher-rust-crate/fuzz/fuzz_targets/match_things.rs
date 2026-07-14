//! SEC-I2 fuzz target: the full matching engine over deserialized input.
//!
//! Interprets the fuzz bytes as a JSON `[thing_a, thing_b]` tuple,
//! then runs `MatchingEngine::match_things`. Pins the two load-bearing
//! invariants on arbitrary (attacker-controlled) input — the engine
//! **never panics**, and the score is **finite and within `[0.0, 1.0]`** —
//! across the whole deserialize → normalize → score path. Complements the
//! crate's `proptest` properties with coverage-guided input generation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use thing_matcher::{MatchConfig, MatchingEngine, Thing};

fuzz_target!(|data: &[u8]| {
    // Only well-formed JSON tuples reach the engine; malformed input is a
    // no-op (the service's HTTP layer rejects it long before the matcher).
    let Ok((a, b)) = serde_json::from_slice::<(Thing, Thing)>(data) else {
        return;
    };

    let engine = MatchingEngine::new(MatchConfig::default());
    // Run both orderings so either argument position is exercised.
    for result in [engine.match_things(&a, &b), engine.match_things(&b, &a)] {
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
});
