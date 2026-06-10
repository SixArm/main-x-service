#![warn(clippy::pedantic)]

//! Property-based tests.
//!
//! Each property generates many random inputs via `proptest` and checks an
//! invariant that should hold for **every** input. The point is to catch
//! the failure modes that example-based tests miss: weird Unicode in
//! names, sparse / dense `Thing` records.

use proptest::prelude::*;
use thing_matcher::{Confidence, MatchConfig, MatchingEngine, Normalizer, Thing};

// ---------- Strategies ----------

/// A reasonable name string for proptest: arbitrary Unicode constrained
/// to lengths we'd plausibly see in practice. Skips strings that
/// normalise to empty so `validate()` will still pass on builders that
/// only carry a name.
fn name_strategy() -> impl Strategy<Value = String> {
    "[\\PC]{1,40}".prop_filter("normalises to empty", |s| {
        !Normalizer::normalize_name(s).is_empty()
    })
}

/// A short URL-like string for proptest.
fn url_strategy() -> impl Strategy<Value = String> {
    "[a-z]{1,8}".prop_map(|s| format!("https://example.org/{s}"))
}

/// A `Thing` carrying enough data to make `validate()` pass and the
/// matching engine produce a non-trivial score.
fn thing_strategy() -> impl Strategy<Value = Thing> {
    (
        name_strategy(),
        prop::collection::vec(name_strategy(), 0..3),
        prop::option::of(url_strategy()),
        prop::collection::vec(url_strategy(), 0..3),
    )
        .prop_map(|(name, alts, url, sames)| {
            let mut b = Thing::builder().name(name).alternate_names(alts);
            if let Some(u) = url {
                b = b.url(u);
            }
            for s in sames {
                b = b.add_same_as(s);
            }
            b.build()
        })
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 500,
        .. ProptestConfig::default()
    })]

    /// `normalize_name` MUST be idempotent.
    #[test]
    fn normalize_name_is_idempotent(s in "\\PC{0,80}") {
        let once = Normalizer::normalize_name(&s);
        let twice = Normalizer::normalize_name(&once);
        prop_assert_eq!(once, twice);
    }

    /// `normalize_name` MUST always be lowercase and whitespace-trimmed.
    #[test]
    fn normalize_name_has_no_uppercase_or_leading_whitespace(s in "\\PC{0,80}") {
        let n = Normalizer::normalize_name(&s);
        prop_assert!(!n.chars().any(|c| c.is_ascii_uppercase()));
        prop_assert!(!n.starts_with(' '));
        prop_assert!(!n.ends_with(' '));
    }

    /// `normalize_text` MUST be idempotent.
    #[test]
    fn normalize_text_is_idempotent(s in "\\PC{0,80}") {
        let once = Normalizer::normalize_text(&s);
        let twice = Normalizer::normalize_text(&once);
        prop_assert_eq!(once, twice);
    }

    /// `normalize_url` MUST be idempotent.
    #[test]
    fn normalize_url_is_idempotent(s in "\\PC{0,80}") {
        let once = Normalizer::normalize_url(&s);
        let twice = Normalizer::normalize_url(&once);
        prop_assert_eq!(once, twice);
    }

    /// Probabilistic `score` MUST always land in `[0.0, 1.0]`.
    #[test]
    fn score_is_bounded_unit_interval(p1 in thing_strategy(), p2 in thing_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_things(&p1, &p2);
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Self-match MUST produce `is_match == true` for any validating `Thing`.
    #[test]
    fn self_match_is_true(t in thing_strategy()) {
        prop_assume!(t.validate().is_ok());
        let r = MatchingEngine::default_config().match_things(&t, &t);
        prop_assert!(r.is_match, "self-match failed for {:?}: score={}", t, r.score);
    }

    /// Self-match MUST yield `High` confidence.
    #[test]
    fn self_match_confidence_is_high(t in thing_strategy()) {
        prop_assume!(t.validate().is_ok());
        let r = MatchingEngine::default_config().match_things(&t, &t);
        prop_assert_eq!(r.confidence, Confidence::High);
    }

    /// `match_things` MUST be symmetric.
    #[test]
    fn matching_is_symmetric(p1 in thing_strategy(), p2 in thing_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_things(&p1, &p2);
        let reverse = engine.match_things(&p2, &p1);
        prop_assert!(
            (forward.score - reverse.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            forward.score,
            reverse.score
        );
        prop_assert_eq!(forward.is_match, reverse.is_match);
        prop_assert_eq!(forward.confidence, reverse.confidence);
    }

    /// `deterministic_match` MUST also be symmetric.
    #[test]
    fn deterministic_match_is_symmetric(p1 in thing_strategy(), p2 in thing_strategy()) {
        let engine = MatchingEngine::default_config();
        prop_assert_eq!(
            engine.deterministic_match(&p1, &p2),
            engine.deterministic_match(&p2, &p1)
        );
    }

    /// `MatchConfig` MUST survive a JSON round-trip without value drift.
    #[test]
    fn match_config_default_round_trips_through_json(_ignored in any::<u8>()) {
        let original = MatchConfig::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        prop_assert!((original.match_threshold - back.match_threshold).abs() < 1e-12);
        prop_assert!((original.name_weight - back.name_weight).abs() < 1e-12);
        prop_assert!((original.identifiers_weight - back.identifiers_weight).abs() < 1e-12);
        prop_assert!((original.same_as_weight - back.same_as_weight).abs() < 1e-12);
        prop_assert_eq!(original.strict_mode, back.strict_mode);
    }

    /// `Thing` MUST survive a JSON round-trip.
    #[test]
    fn thing_round_trips_through_json(t in thing_strategy()) {
        let json = serde_json::to_string(&t).expect("serialise");
        let back: Thing = serde_json::from_str(&json).expect("deserialise");
        prop_assert_eq!(&t.name, &back.name);
        prop_assert_eq!(&t.alternate_names, &back.alternate_names);
        prop_assert_eq!(&t.description, &back.description);
        prop_assert_eq!(&t.url, &back.url);
        prop_assert_eq!(&t.image, &back.image);
        prop_assert_eq!(&t.same_as, &back.same_as);
        prop_assert_eq!(&t.additional_types, &back.additional_types);
        prop_assert_eq!(&t.identifiers, &back.identifiers);
        prop_assert_eq!(&t.owner, &back.owner);
        prop_assert_eq!(&t.local_id, &back.local_id);
    }

    /// `Confidence::from_score` MUST be monotonic.
    #[test]
    fn confidence_is_monotonic(a in 0.0f64..=1.0, b in 0.0f64..=1.0) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let rank = |c: Confidence| match c {
            Confidence::Low => 0u8,
            Confidence::Medium => 1,
            Confidence::High => 2,
        };
        let ra = rank(Confidence::from_score(lo));
        let rb = rank(Confidence::from_score(hi));
        prop_assert!(
            rb >= ra,
            "score {} -> {:?}, score {} -> {:?}",
            lo,
            Confidence::from_score(lo),
            hi,
            Confidence::from_score(hi)
        );
    }
}
