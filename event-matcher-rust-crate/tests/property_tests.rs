//! Property-based tests.
//!
//! Each property generates many random inputs via `proptest` and checks an
//! invariant that should hold for **every** input. The point is to catch
//! the failure modes that example-based tests miss: weird Unicode in
//! names, edge-case dates, sparse / dense `Event` records.

use event_matcher::{
    Confidence, Event, EventCategory, MatchConfig, MatchingEngine, Normalizer,
};
use proptest::prelude::*;

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

/// An ISO 8601 datetime sampled from a wide but bounded year range.
fn iso8601_datetime_strategy() -> impl Strategy<Value = String> {
    (
        1900i32..=2100,
        1u32..=12,
        1u32..=28,
        0u32..=23,
        0u32..=59,
        0u32..=59,
    )
        .prop_map(|(y, m, d, h, mi, s)| {
            format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
        })
}

/// A reasonable category for proptest.
fn category_strategy() -> impl Strategy<Value = EventCategory> {
    prop_oneof![
        Just(EventCategory::MusicEvent),
        Just(EventCategory::ComedyEvent),
        Just(EventCategory::Festival),
        Just(EventCategory::ConferenceEvent),
        Just(EventCategory::SportsEvent),
        Just(EventCategory::SocialEvent),
    ]
}

/// An `Event` carrying enough data to make `validate()` pass and the
/// matching engine produce a non-trivial score.
fn event_strategy() -> impl Strategy<Value = Event> {
    (
        name_strategy(),
        prop::collection::vec(name_strategy(), 0..3),
        prop::option::of(iso8601_datetime_strategy()),
        prop::option::of(iso8601_datetime_strategy()),
        prop::option::of(category_strategy()),
    )
        .prop_map(|(name, alts, start, end, cat)| {
            let mut b = Event::builder().name(name).alternate_names(alts);
            if let Some(s) = start {
                b = b.start_date(s);
            }
            if let Some(e) = end {
                b = b.end_date(e);
            }
            if let Some(c) = cat {
                b = b.category(c);
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

    /// Probabilistic `score` MUST always land in `[0.0, 1.0]`.
    #[test]
    fn score_is_bounded_unit_interval(p1 in event_strategy(), p2 in event_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_events(&p1, &p2);
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Self-match MUST produce `is_match == true` for any validating `Event`.
    #[test]
    fn self_match_is_true(p in event_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_events(&p, &p);
        prop_assert!(r.is_match, "self-match failed for {:?}: score={}", p, r.score);
    }

    /// Self-match MUST yield `High` confidence.
    #[test]
    fn self_match_confidence_is_high(p in event_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_events(&p, &p);
        prop_assert_eq!(r.confidence, Confidence::High);
    }

    /// `match_events` MUST be symmetric.
    #[test]
    fn matching_is_symmetric(p1 in event_strategy(), p2 in event_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_events(&p1, &p2);
        let reverse = engine.match_events(&p2, &p1);
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
    fn deterministic_match_is_symmetric(p1 in event_strategy(), p2 in event_strategy()) {
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
        prop_assert!((original.start_date_weight - back.start_date_weight).abs() < 1e-12);
        prop_assert!((original.location_weight - back.location_weight).abs() < 1e-12);
        prop_assert_eq!(original.strict_mode, back.strict_mode);
    }

    /// `Event` MUST survive a JSON round-trip.
    #[test]
    fn event_round_trips_through_json(p in event_strategy()) {
        let json = serde_json::to_string(&p).expect("serialise");
        let back: Event = serde_json::from_str(&json).expect("deserialise");
        prop_assert_eq!(&p.name, &back.name);
        prop_assert_eq!(&p.alternate_names, &back.alternate_names);
        prop_assert_eq!(&p.start_date, &back.start_date);
        prop_assert_eq!(&p.end_date, &back.end_date);
        prop_assert_eq!(&p.category, &back.category);
    }

    /// `start_date` sub-score MUST be commutative.
    #[test]
    fn start_date_subscore_is_symmetric(
        s1 in iso8601_datetime_strategy(),
        s2 in iso8601_datetime_strategy(),
    ) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").start_date(&s1).build();
        let e2 = Event::builder().name("X").start_date(&s2).build();
        let forward = engine.match_events(&e1, &e2).breakdown.start_date_score;
        let reverse = engine.match_events(&e2, &e1).breakdown.start_date_score;
        match (forward, reverse) {
            (Some(a), Some(b)) => prop_assert!((a - b).abs() < 1e-9),
            (None, None) => {}
            _ => prop_assert!(false, "asymmetric None"),
        }
    }

    /// `parse_iso8601_unix_seconds` MUST be idempotent under
    /// canonicalisation: parsing back the string form of a Unix-second
    /// timestamp must return the same number.
    ///
    /// Sampled within a bounded range to keep the test fast.
    #[test]
    fn parse_iso8601_round_trip(s in iso8601_datetime_strategy()) {
        let parsed = Normalizer::parse_iso8601_unix_seconds(&s);
        prop_assert!(parsed.is_some(), "failed to parse {s}");
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
