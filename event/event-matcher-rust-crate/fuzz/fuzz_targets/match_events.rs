//! SEC-I2 fuzz target: the full matching engine over deserialized input.
//!
//! Interprets the fuzz bytes as a JSON `[event_a, event_b]` tuple,
//! then runs `MatchingEngine::match_events`. Pins the two load-bearing
//! invariants on arbitrary (attacker-controlled) input — the engine
//! **never panics**, and the score is **finite and within `[0.0, 1.0]`** —
//! across the whole deserialize → normalize → score path. Complements the
//! crate's `proptest` properties with coverage-guided input generation.
//!
//! Also exercises `deterministic_match` (spec/10-open-questions.md T-4):
//! the other public infallible entry point (spec §8.6) had no
//! coverage-guided fuzzing at all — only the probabilistic path did.

#![no_main]

use event_matcher::{Event, MatchConfig, MatchingEngine};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only well-formed JSON tuples reach the engine; malformed input is a
    // no-op (the service's HTTP layer rejects it long before the matcher).
    let Ok((a, b)) = serde_json::from_slice::<(Event, Event)>(data) else {
        return;
    };

    let engine = MatchingEngine::new(MatchConfig::default());
    // Run both orderings so either argument position is exercised.
    for result in [engine.match_events(&a, &b), engine.match_events(&b, &a)] {
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

    // `deterministic_match` never panics on the same fuzzed pair, in
    // either order (T-4). It returns a plain `bool`, so there is no
    // range to check — only the never-panic invariant applies.
    let _ = engine.deterministic_match(&a, &b);
    let _ = engine.deterministic_match(&b, &a);
});
