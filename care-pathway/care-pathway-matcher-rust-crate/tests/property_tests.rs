#![warn(clippy::pedantic)]

//! Property-based tests (SEC-M6).
//!
//! Each property generates many random inputs via `proptest` and checks an
//! invariant that must hold for **every** input. The point is to catch the
//! failure modes example-based tests miss: weird Unicode in names, blank /
//! punctuation-only codes, sparse vs dense `CarePathway` records. The
//! headline guarantees are that the matcher (and its pure helpers) **never
//! panic** on arbitrary input, and that every `score` is a well-behaved
//! number in `[0.0, 1.0]`.

use care_pathway_matcher::{
    CarePathway, CodeSystem, ConditionCode, MatchingEngine, normalize, phonetic,
};
use proptest::prelude::*;

// ---------- Strategies ----------

/// Arbitrary short-ish text: any non-control Unicode, including diacritics,
/// ligatures, and scripts the normaliser must tolerate.
fn text() -> impl Strategy<Value = String> {
    "\\PC{0,32}"
}

/// A clinical coding system — the three known variants plus a free-form
/// `Custom` label.
fn code_system() -> impl Strategy<Value = CodeSystem> {
    prop_oneof![
        Just(CodeSystem::Icd10),
        Just(CodeSystem::Icd11),
        Just(CodeSystem::Snomed),
        "\\PC{0,8}".prop_map(CodeSystem::Custom),
    ]
}

/// A `ConditionCode` = a coding system plus an arbitrary code value.
fn condition_code() -> impl Strategy<Value = ConditionCode> {
    (code_system(), "\\PC{0,12}").prop_map(|(system, code)| ConditionCode { system, code })
}

/// A `CarePathway` varying only the few string-bearing fields the task
/// calls out — name, pathway code, provider, condition codes, keywords —
/// via lightweight direct field assignment (no builder). Everything else
/// stays at its `Default` so construction is cheap.
fn pathway() -> impl Strategy<Value = CarePathway> {
    (
        text(),                   // name
        prop::option::of(text()), // pathway_code
        prop::option::of(text()), // provider_id
        prop::collection::vec(condition_code(), 0..3),
        prop::collection::vec(text(), 0..3), // keywords
    )
        .prop_map(|(name, code, provider, conditions, keywords)| {
            let mut p = CarePathway::new(name);
            p.pathway_code = code;
            p.provider_id = provider;
            p.condition_codes = conditions;
            p.keywords = keywords;
            p
        })
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 400, ..ProptestConfig::default() })]

    /// The pure normalisation / phonetic helpers MUST never panic on
    /// arbitrary strings, including empty, whitespace-only, and control
    /// characters.
    #[test]
    fn helpers_never_panic(s in any::<String>(), t in any::<String>()) {
        let _ = normalize::fold(&s);
        let _ = normalize::pathway_code(&s);
        let _ = normalize::fold_set(&[s.clone(), t.clone()]);
        let _ = phonetic::soundex(&s);
        let _ = phonetic::same(&s, &t);
    }

    /// Driving the match engine over arbitrary pathways MUST never panic
    /// and MUST yield a finite score in the unit interval (never NaN,
    /// never outside `[0.0, 1.0]`).
    #[test]
    fn score_is_bounded_and_finite(a in pathway(), b in pathway()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_care_pathways(&a, &b);
        prop_assert!(!r.score.is_nan(), "score is NaN");
        prop_assert!((0.0..=1.0).contains(&r.score), "score out of range: {}", r.score);
    }

    /// `match_care_pathways` MUST be symmetric: swapping the arguments
    /// changes neither the score nor the derived `is_match` / `confidence`.
    #[test]
    fn matching_is_symmetric(a in pathway(), b in pathway()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_care_pathways(&a, &b);
        let reverse = engine.match_care_pathways(&b, &a);
        prop_assert!(
            (forward.score - reverse.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            forward.score,
            reverse.score
        );
        prop_assert_eq!(forward.is_match, reverse.is_match);
        prop_assert_eq!(forward.confidence, reverse.confidence);
    }

    /// An identical clone of a well-formed pathway MUST match itself. We
    /// assert `is_match` (score meets the threshold) rather than an exact
    /// `1.0`, since the guarantee is a positive match, not a specific value.
    #[test]
    fn self_match_is_a_match(p in pathway()) {
        let r = MatchingEngine::default_config().match_care_pathways(&p, &p);
        prop_assert!(r.is_match, "self-match not a match: score {} for {:?}", r.score, p);
    }
}
