#![warn(clippy::pedantic)]

//! Property-based tests (spec §18.4, task T-6).
//!
//! Each property generates many random inputs via `proptest` and checks an
//! invariant that should hold for **every** input. The point is to catch
//! the failure modes that example-based tests miss: weird Unicode in
//! names, edge-case dates, sparse / dense `Person` records, and so on.
//!
//! The case count is pinned at 1000 per spec §23 T-6. If a property
//! shrinks to a regression vector, copy that vector into a normal
//! `#[test]` in `tests/integration_tests.rs` so it lives forever.

use jiff::civil::{Date, date};
use person_matcher::{Confidence, Gender, MatchConfig, MatchingEngine, Normalizer, Person};
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

/// A bounded date so we never trip date construction on implausible
/// years.
fn date_strategy() -> impl Strategy<Value = Date> {
    (1900i16..=2100, 1i8..=12, 1i8..=28).prop_map(|(y, m, d)| date(y, m, d))
}

fn gender_strategy() -> impl Strategy<Value = Gender> {
    prop_oneof![
        Just(Gender::Male),
        Just(Gender::Female),
        Just(Gender::Other),
        Just(Gender::Unknown),
    ]
}

/// A `Person` carrying enough data to make `validate()` pass and the
/// matching engine produce a non-trivial score.
fn person_strategy() -> impl Strategy<Value = Person> {
    (
        prop::option::of(name_strategy()),
        prop::option::of(name_strategy()),
        prop::option::of(date_strategy()),
        prop::option::of(gender_strategy()),
    )
        .prop_filter(
            "must have at least a given or family name",
            |(g, f, _, _)| g.is_some() || f.is_some(),
        )
        .prop_map(|(given, family, dob, gender)| {
            let mut b = Person::builder();
            if let Some(g) = given {
                b = b.given_name(g);
            }
            if let Some(f) = family {
                b = b.family_name(f);
            }
            if let Some(d) = dob {
                b = b.date_of_birth(d);
            }
            if let Some(g) = gender {
                b = b.gender(g);
            }
            b.build()
        })
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        .. ProptestConfig::default()
    })]

    /// `normalize_name` MUST be idempotent — applying it twice yields the
    /// same string as applying it once. This guards against future
    /// normalisation steps that might consume their own output (e.g. a
    /// regex that strips a character class the second pass would also
    /// match).
    #[test]
    fn normalize_name_is_idempotent(s in "\\PC{0,80}") {
        let once = Normalizer::normalize_name(&s);
        let twice = Normalizer::normalize_name(&once);
        prop_assert_eq!(once, twice);
    }

    /// `normalize_name` MUST always be lowercase ASCII whitespace-free —
    /// the pipeline lowercases, NFKD-decomposes, strips combining marks,
    /// removes ASCII punctuation, and collapses whitespace.
    #[test]
    fn normalize_name_has_no_uppercase_or_leading_whitespace(s in "\\PC{0,80}") {
        let n = Normalizer::normalize_name(&s);
        prop_assert!(!n.chars().any(|c| c.is_ascii_uppercase()));
        prop_assert!(!n.starts_with(' '));
        prop_assert!(!n.ends_with(' '));
    }

    /// Probabilistic `score` MUST always land in `[0.0, 1.0]`. This is
    /// the core sanity invariant for downstream consumers — they treat
    /// the value as a probability-like signal and a stray `1.05` would
    /// break their thresholds.
    #[test]
    fn score_is_bounded_unit_interval(p1 in person_strategy(), p2 in person_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_persons(&p1, &p2);
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Self-match MUST produce `is_match == true` for any `Person` that
    /// passes `validate()` under the default config. Matching a record
    /// against itself is the canonical regression case — if it ever
    /// fails, scoring is structurally broken.
    #[test]
    fn self_match_is_true(p in person_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_persons(&p, &p);
        prop_assert!(r.is_match, "self-match failed for {:?}: score={}", p, r.score);
    }

    /// Self-match MUST yield `High` confidence. Self-equality is the
    /// strongest possible signal; if the engine ever lands a clone of a
    /// record in `Medium` or `Low`, the confidence bands are mis-tuned.
    #[test]
    fn self_match_confidence_is_high(p in person_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_persons(&p, &p);
        prop_assert_eq!(r.confidence, Confidence::High);
    }

    /// `match_persons` MUST be symmetric: swapping the two persons
    /// must not change the score (within floating-point tolerance) or
    /// the `is_match` decision. Asymmetry would mean the API leaks
    /// argument-order information into the result, which is a hidden
    /// foot-gun for batch callers.
    #[test]
    fn matching_is_symmetric(p1 in person_strategy(), p2 in person_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_persons(&p1, &p2);
        let reverse = engine.match_persons(&p2, &p1);
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
    fn deterministic_match_is_symmetric(p1 in person_strategy(), p2 in person_strategy()) {
        let engine = MatchingEngine::default_config();
        prop_assert_eq!(
            engine.deterministic_match(&p1, &p2),
            engine.deterministic_match(&p2, &p1)
        );
    }

    /// `MatchConfig` MUST survive a JSON round-trip without value drift.
    /// `MatchConfig` is the API contract a downstream service ships in
    /// version control; if defaults silently mutate through serde, audit
    /// trails break.
    #[test]
    fn match_config_default_round_trips_through_json(_ignored in any::<u8>()) {
        let original = MatchConfig::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        // Compare field-by-field on the public f64 weights.
        prop_assert!((original.match_threshold - back.match_threshold).abs() < 1e-12);
        prop_assert!((original.united_kingdom_national_health_service_number_weight - back.united_kingdom_national_health_service_number_weight).abs() < 1e-12);
        prop_assert!((original.given_name_weight - back.given_name_weight).abs() < 1e-12);
        prop_assert!((original.family_name_weight - back.family_name_weight).abs() < 1e-12);
        prop_assert!((original.date_of_birth_weight - back.date_of_birth_weight).abs() < 1e-12);
        prop_assert!((original.death_date_weight - back.death_date_weight).abs() < 1e-12);
        prop_assert!((original.death_place_weight - back.death_place_weight).abs() < 1e-12);
        prop_assert!((original.birth_place_weight - back.birth_place_weight).abs() < 1e-12);
        prop_assert_eq!(original.strict_mode, back.strict_mode);
    }

    /// `Person` MUST survive a JSON round-trip. Legacy consumers stash
    /// records in JSON columns; field renames or new required fields
    /// would silently corrupt their archives.
    #[test]
    fn person_round_trips_through_json(p in person_strategy()) {
        let json = serde_json::to_string(&p).expect("serialise");
        let back: Person = serde_json::from_str(&json).expect("deserialise");
        prop_assert_eq!(p, back);
    }

    /// DOB transposition heuristic MUST be commutative on its inputs.
    /// `score_date_of_birth(a, b)` should equal `score_date_of_birth(b, a)`.
    /// This is implied by `matching_is_symmetric` for the overall score
    /// but worth pinning directly on the breakdown so a future refactor
    /// of just the DOB scorer is also covered.
    #[test]
    fn dob_subscore_is_symmetric(
        d1 in date_strategy(),
        d2 in date_strategy(),
    ) {
        let engine = MatchingEngine::default_config();
        let p1 = Person::builder().given_name("X").date_of_birth(d1).build();
        let p2 = Person::builder().given_name("X").date_of_birth(d2).build();
        let forward = engine.match_persons(&p1, &p2).breakdown.date_of_birth_score;
        let reverse = engine.match_persons(&p2, &p1).breakdown.date_of_birth_score;
        prop_assert_eq!(forward, reverse);
    }

    /// `Confidence::from_score` MUST be monotonic — never down-band a
    /// higher score relative to a lower one. This is a structural
    /// guarantee of the band table and the most common place a future
    /// "fix" might silently break the contract.
    #[test]
    fn confidence_is_monotonic(a in 0.0f64..=1.0, b in 0.0f64..=1.0) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        // Explicit rank: Low < Medium < High. The enum's declaration
        // order is the inverse, so we cannot just compare discriminants.
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
