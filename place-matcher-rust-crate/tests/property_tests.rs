#![warn(clippy::pedantic)]

//! Property-based tests.
//!
//! Each property generates many random inputs via `proptest` and checks an
//! invariant that should hold for **every** input. The point is to catch
//! the failure modes that example-based tests miss: weird Unicode in
//! names, edge-case coordinates, sparse / dense `Place` records.

use place_matcher::{Confidence, MatchConfig, MatchingEngine, Normalizer, Place, PlaceCategory};
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

/// A latitude in the conventional range.
fn lat_strategy() -> impl Strategy<Value = f64> {
    -90.0_f64..=90.0
}

/// A longitude in the conventional range.
fn lon_strategy() -> impl Strategy<Value = f64> {
    -180.0_f64..=180.0
}

/// A reasonable category for proptest.
fn category_strategy() -> impl Strategy<Value = PlaceCategory> {
    prop_oneof![
        Just(PlaceCategory::Hotel),
        Just(PlaceCategory::Cafe),
        Just(PlaceCategory::Park),
        Just(PlaceCategory::Museum),
        Just(PlaceCategory::Monument),
        Just(PlaceCategory::City),
    ]
}

/// A `Place` carrying enough data to make `validate()` pass and the
/// matching engine produce a non-trivial score.
fn place_strategy() -> impl Strategy<Value = Place> {
    (
        name_strategy(),
        prop::collection::vec(name_strategy(), 0..3),
        prop::option::of(lat_strategy()),
        prop::option::of(lon_strategy()),
        prop::option::of(category_strategy()),
    )
        .prop_map(|(name, alts, lat, lon, cat)| {
            let mut b = Place::builder().name(name).alternate_names(alts);
            if let Some(l) = lat {
                b = b.latitude(l);
            }
            if let Some(l) = lon {
                b = b.longitude(l);
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
    fn score_is_bounded_unit_interval(p1 in place_strategy(), p2 in place_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_places(&p1, &p2);
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Self-match MUST produce `is_match == true` for any validating `Place`.
    #[test]
    fn self_match_is_true(p in place_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_places(&p, &p);
        prop_assert!(r.is_match, "self-match failed for {:?}: score={}", p, r.score);
    }

    /// Self-match MUST yield `High` confidence.
    #[test]
    fn self_match_confidence_is_high(p in place_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_places(&p, &p);
        prop_assert_eq!(r.confidence, Confidence::High);
    }

    /// `match_places` MUST be symmetric.
    #[test]
    fn matching_is_symmetric(p1 in place_strategy(), p2 in place_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_places(&p1, &p2);
        let reverse = engine.match_places(&p2, &p1);
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
    fn deterministic_match_is_symmetric(p1 in place_strategy(), p2 in place_strategy()) {
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
        prop_assert!((original.coordinates_weight - back.coordinates_weight).abs() < 1e-12);
        prop_assert!((original.address_weight - back.address_weight).abs() < 1e-12);
        prop_assert_eq!(original.strict_mode, back.strict_mode);
    }

    /// `Place` MUST survive a JSON round-trip up to f64 representation
    /// precision: the carried floats may drift by one ULP through
    /// `serde_json` formatting, but every other field must compare equal.
    #[test]
    fn place_round_trips_through_json(p in place_strategy()) {
        let json = serde_json::to_string(&p).expect("serialise");
        let back: Place = serde_json::from_str(&json).expect("deserialise");
        prop_assert_eq!(&p.name, &back.name);
        prop_assert_eq!(&p.alternate_names, &back.alternate_names);
        prop_assert_eq!(&p.category, &back.category);
        prop_assert_eq!(&p.place_ids, &back.place_ids);
        prop_assert_eq!(&p.address, &back.address);
        prop_assert_eq!(&p.phone, &back.phone);
        prop_assert_eq!(&p.email, &back.email);
        prop_assert_eq!(&p.local_id, &back.local_id);
        prop_assert_eq!(&p.country_code_as_iso_3166_1_alpha_2, &back.country_code_as_iso_3166_1_alpha_2);
        prop_assert_eq!(p.maximum_capacity_count, back.maximum_capacity_count);
        for (a, b) in [
            (p.latitude, back.latitude),
            (p.longitude, back.longitude),
            (p.altitude_as_metre, back.altitude_as_metre),
            (p.elevation_as_metre, back.elevation_as_metre),
            (p.area_as_metre_2, back.area_as_metre_2),
        ] {
            match (a, b) {
                (Some(x), Some(y)) => {
                    let tol = (x.abs().max(y.abs()) * 1e-12).max(1e-12);
                    prop_assert!((x - y).abs() <= tol, "{} vs {}", x, y);
                }
                (None, None) => {}
                _ => prop_assert!(false, "asymmetric Option"),
            }
        }
    }

    /// Coordinates sub-score MUST be commutative.
    #[test]
    fn coordinates_subscore_is_symmetric(
        lat1 in lat_strategy(),
        lon1 in lon_strategy(),
        lat2 in lat_strategy(),
        lon2 in lon_strategy(),
    ) {
        let engine = MatchingEngine::default_config();
        let p1 = Place::builder().name("X").latitude(lat1).longitude(lon1).build();
        let p2 = Place::builder().name("X").latitude(lat2).longitude(lon2).build();
        let forward = engine.match_places(&p1, &p2).breakdown.coordinates_score;
        let reverse = engine.match_places(&p2, &p1).breakdown.coordinates_score;
        match (forward, reverse) {
            (Some(a), Some(b)) => prop_assert!((a - b).abs() < 1e-9),
            (None, None) => {}
            _ => prop_assert!(false, "asymmetric None"),
        }
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
