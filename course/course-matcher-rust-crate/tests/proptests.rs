#![warn(clippy::pedantic)]

//! Property-based tests (SEC-M6).
//!
//! Each property generates many random inputs via `proptest` and checks
//! an invariant that must hold for **every** input. The point is to
//! catch the adversarial failure modes that example-based tests miss:
//! weird Unicode in names, blank / whitespace-only codes, sparse and
//! dense `Course` records, and arbitrary UTF-8 fed straight at the pure
//! helper functions.
//!
//! All tests drive the crate's **public** surface only
//! (`use course_matcher::…`) — the same contract the service embeds.

use course_matcher::{Course, MatchConfig, MatchingEngine};
use course_matcher::{normalize, phonetic};
use proptest::prelude::*;

// ---------- Strategies ----------

/// A course name for proptest: arbitrary non-control Unicode, bounded to
/// lengths we'd plausibly see. Filtered so it does not fold to empty, so
/// records built from it are "well-formed" (carry a real name signal).
fn name_strategy() -> impl Strategy<Value = String> {
    "\\PC{1,40}".prop_filter("folds to empty", |s| !normalize::fold(s).is_empty())
}

/// Build a lightweight `Course` by varying only a handful of string
/// fields (name, alternate names, `course_code`, `provider_id`, keywords).
/// Deeply-nested types (identifiers, educational level, …) are left at
/// their defaults on purpose — we want cheap construction, not a full
/// arbitrary `Course` strategy. `provider_id`, when present, may be an
/// empty string — this is the adversarial form used by the panic /
/// bounds properties.
fn course_strategy() -> impl Strategy<Value = Course> {
    course_strategy_with_provider("\\PC{0,12}")
}

fn course_strategy_with_provider(provider_regex: &'static str) -> impl Strategy<Value = Course> {
    (
        name_strategy(),
        prop::collection::vec(name_strategy(), 0..3),
        prop::option::of("\\PC{0,12}"),
        prop::option::of(provider_regex),
        prop::collection::vec("\\PC{0,16}", 0..4),
    )
        .prop_map(|(name, alts, code, provider, keywords)| {
            let mut c = Course::new(name);
            c.alternate_names = alts;
            c.course_code = code;
            c.provider_id = provider;
            c.keywords = keywords;
            c
        })
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// The engine MUST NOT panic on any pair of arbitrary-string courses.
    /// (proptest fails the case if the body panics; the assert also pins
    /// the score is a real number.)
    #[test]
    fn match_never_panics(a in course_strategy(), b in course_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_courses(&a, &b);
        prop_assert!(r.score.is_finite(), "non-finite score: {}", r.score);
    }

    /// The pure helpers MUST NOT panic on ANY arbitrary UTF-8 (including
    /// control characters and lone combining marks).
    #[test]
    fn pure_helpers_never_panic(s in any::<String>(), t in any::<String>()) {
        let _ = normalize::fold(&s);
        let _ = normalize::course_code(&s);
        let _ = normalize::fold_set(&[s.clone(), t.clone()]);
        let _ = phonetic::soundex(&s);
        let _ = phonetic::same(&s, &t);
    }

    /// `score` MUST always lie in `[0.0, 1.0]` and never be NaN.
    #[test]
    fn score_is_bounded_unit_interval(a in course_strategy(), b in course_strategy()) {
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        prop_assert!(!r.score.is_nan(), "score is NaN");
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Matching MUST be symmetric over the varied string fields:
    /// `match(a,b) == match(b,a)` on score, `is_match`, and confidence.
    /// The general strategy allows a blank `provider_id` on one side —
    /// previously that made `provider_score` asymmetric (SEC-M6 fix in
    /// `matcher.rs::provider_score`); this property now holds for it too.
    #[test]
    fn matching_is_symmetric(a in course_strategy(), b in course_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_courses(&a, &b);
        let reverse = engine.match_courses(&b, &a);
        prop_assert!(
            (forward.score - reverse.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            forward.score,
            reverse.score
        );
        prop_assert_eq!(forward.is_match, reverse.is_match);
        prop_assert_eq!(forward.confidence, reverse.confidence);
    }

    /// An identical clone of a well-formed course MUST match itself:
    /// `is_match` is true and the score clears the configured threshold.
    /// (Asserted as a threshold, NOT an exact 1.0 — the probabilistic
    /// path need not pin identity to 1.0.)
    #[test]
    fn self_match_is_a_match(a in course_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_courses(&a, &a.clone());
        prop_assert!(
            r.is_match && r.score >= MatchConfig::default().threshold,
            "self-match failed: score={} is_match={}",
            r.score,
            r.is_match
        );
    }

    /// `soundex` MUST be `None`, or a 4-char `[A-Z][0-9]{3}` code —
    /// on arbitrary UTF-8, and never panics.
    #[test]
    fn soundex_shape_is_well_formed(s in any::<String>()) {
        if let Some(code) = phonetic::soundex(&s) {
            let bytes = code.as_bytes();
            prop_assert_eq!(bytes.len(), 4, "code not 4 chars: {:?}", code);
            prop_assert!(
                bytes[0].is_ascii_uppercase(),
                "first char not [A-Z]: {:?}",
                code
            );
            prop_assert!(
                bytes[1..].iter().all(u8::is_ascii_digit),
                "trailing chars not digits: {:?}",
                code
            );
        }
    }
}
