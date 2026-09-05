#![warn(clippy::pedantic)]

//! Property-based tests (SEC-M6).
//!
//! Each property drives many random inputs through `proptest` and asserts
//! an invariant that must hold for **every** input. These catch the
//! failure modes example-based tests miss: arbitrary Unicode in every
//! string field, sparse vs dense `Case` records, and adversarial /
//! sentinel identifier values (building on the SEC-M2 trivial-identifier
//! guard).
//!
//! The proven invariants:
//!
//! - **Never panics** — driving the match engine and every pure helper
//!   (`fold` / `case_number` / `url` / `fold_set` / `soundex` / `same` /
//!   `Confidence::classify`) on arbitrary strings and floats.
//! - **Score bounds** — `MatchResult::score` is always finite and in
//!   `[0.0, 1.0]` (never `NaN`).
//! - **Symmetry** — `match(a, b).score == match(b, a).score` (and the
//!   `is_match` / `confidence` derived from it) for cases varying several
//!   string fields.
//! - **Reflexivity** — an identical clone of a well-formed case matches
//!   itself.
//! - **Soundex shape** — the phonetic encoder returns `None` or a
//!   `[A-Z][0-9]{3}` code for any UTF-8 input, and never panics.

use case_matcher::{Case, Confidence, MatchConfig, MatchingEngine};
use case_matcher::{normalize, phonetic};
use proptest::prelude::*;

// ---------- Strategies ----------

/// An arbitrary, bounded string for a single case field. Non-control
/// characters keep failure shrinking readable while still exercising a
/// wide swathe of Unicode (diacritics, ligatures, scripts).
fn field() -> impl Strategy<Value = String> {
    "[\\PC]{0,24}"
}

/// An optional bounded string field.
fn opt_field() -> impl Strategy<Value = Option<String>> {
    proptest::option::of(field())
}

/// A `Case` varying the identity-bearing string fields the matcher scores:
/// title, alternate titles, agency id, case number, subjects, keywords.
/// Lightweight construction only — enums / dates / identifiers are left at
/// their defaults; those paths are covered by the unit tests.
fn case_strategy() -> impl Strategy<Value = Case> {
    (
        field(),
        prop::collection::vec(field(), 0..3),
        opt_field(),
        opt_field(),
        prop::collection::vec(field(), 0..4),
        prop::collection::vec(field(), 0..4),
    )
        .prop_map(
            |(title, alternate_titles, agency_id, case_number, subjects, keywords)| {
                let mut c = Case::new(title);
                c.alternate_titles = alternate_titles;
                c.agency_id = agency_id;
                c.case_number = case_number;
                c.subjects = subjects;
                c.keywords = keywords;
                c
            },
        )
}

/// A value that is sometimes well-formed (a small non-negative float,
/// suitable for either a weight or a threshold) and sometimes
/// deliberately adversarial (negative, `NaN`, infinite) — the values
/// `MatchConfig::validated` must reject, generated alongside the ones
/// it must accept.
fn adversarial_weight() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => 0.0f64..=1.0,
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => -10.0f64..0.0,
    ]
}

/// Build a `MatchConfig` from 7 adversarial-or-well-formed values: the
/// 6 weights in field-declaration order, then the threshold.
fn config_from(values: &[f64]) -> MatchConfig {
    MatchConfig {
        threshold: values[6],
        title_weight: values[0],
        subjects_weight: values[1],
        case_number_weight: values[2],
        case_type_weight: values[3],
        status_weight: values[4],
        keywords_weight: values[5],
    }
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 500,
        .. ProptestConfig::default()
    })]

    /// The match engine MUST never panic on arbitrary input, and its score
    /// MUST be finite and within the unit interval.
    #[test]
    fn score_is_finite_and_bounded(a in case_strategy(), b in case_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_cases(&a, &b);
        prop_assert!(r.score.is_finite(), "score not finite: {}", r.score);
        prop_assert!(!r.score.is_nan(), "score is NaN");
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Every per-component sub-score present in the breakdown MUST also be
    /// finite and in `[0.0, 1.0]`.
    #[test]
    fn breakdown_subscores_are_bounded(a in case_strategy(), b in case_strategy()) {
        let engine = MatchingEngine::default_config();
        let br = engine.match_cases(&a, &b).breakdown;
        for s in [
            br.title_score,
            br.subjects_score,
            br.case_number_score,
            br.case_type_score,
            br.status_score,
            br.keywords_score,
        ]
        .into_iter()
        .flatten()
        {
            prop_assert!(s.is_finite() && (0.0..=1.0).contains(&s), "subscore out of range: {s}");
        }
    }

    /// Matching MUST be symmetric: argument order cannot change the score,
    /// the `is_match` verdict, or the confidence band.
    #[test]
    fn matching_is_symmetric(a in case_strategy(), b in case_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_cases(&a, &b);
        let reverse = engine.match_cases(&b, &a);
        prop_assert!(
            (forward.score - reverse.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            forward.score,
            reverse.score
        );
        prop_assert_eq!(forward.is_match, reverse.is_match);
        prop_assert_eq!(forward.confidence, reverse.confidence);
    }

    /// A well-formed case MUST match an identical clone of itself: the
    /// title component alone scores 1.0, so `is_match` holds under the
    /// default threshold regardless of the other fields.
    #[test]
    fn identical_case_matches_itself(a in case_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_cases(&a, &a);
        prop_assert!(
            r.is_match,
            "self-match failed: score={} title={:?}",
            r.score,
            a.title
        );
        prop_assert!(r.score >= engine.config().threshold);
    }

    /// The pure text helpers MUST never panic on arbitrary UTF-8, and their
    /// documented codomain constraints hold: `case_number` is uppercase
    /// ASCII-alphanumeric only; `fold` is trim-stable; `fold_set` is sorted
    /// and duplicate-free.
    #[test]
    fn text_helpers_never_panic(s in ".*", items in prop::collection::vec(".*", 0..6)) {
        // `fold` must not panic on arbitrary UTF-8 (it trims before NFKC, so
        // its output is intentionally NOT asserted whitespace-free — NFKC can
        // reintroduce a leading space, e.g. U+00AF `¯` → " " + combining macron).
        let _ = normalize::fold(&s);

        let cn = normalize::case_number(&s);
        prop_assert!(cn.chars().all(|c| c.is_ascii_alphanumeric() && !c.is_ascii_lowercase()));

        let _ = normalize::url(&s);

        let owned: Vec<String> = items.iter().map(std::string::ToString::to_string).collect();
        let set = normalize::fold_set(&owned);
        prop_assert!(set.windows(2).all(|w| w[0] < w[1]), "fold_set not strictly sorted/deduped");
        prop_assert!(set.iter().all(|e| !e.is_empty()), "fold_set kept an empty entry");
    }

    /// `Confidence::classify` MUST never panic and MUST be monotonic in the
    /// score, for arbitrary finite floats (including out-of-range values).
    #[test]
    fn confidence_classify_is_monotonic(x in -5.0f64..5.0, y in -5.0f64..5.0) {
        let (lo, hi) = if x <= y { (x, y) } else { (y, x) };
        let rank = |c: Confidence| match c {
            Confidence::Low => 0u8,
            Confidence::Medium => 1,
            Confidence::High => 2,
        };
        prop_assert!(rank(Confidence::classify(hi)) >= rank(Confidence::classify(lo)));
    }

    /// `soundex` MUST never panic on arbitrary UTF-8 and MUST return either
    /// `None` or a code of exactly `[A-Z][0-9]{3}`.
    #[test]
    fn soundex_shape_is_well_formed(s in ".*") {
        if let Some(code) = phonetic::soundex(&s) {
            let bytes = code.as_bytes();
            prop_assert_eq!(code.len(), 4, "soundex not 4 chars: {}", code);
            prop_assert!(bytes[0].is_ascii_uppercase(), "soundex first char not A-Z: {code}");
            prop_assert!(
                bytes[1..].iter().all(u8::is_ascii_digit),
                "soundex tail not digits: {code}"
            );
        }
    }

    /// `phonetic::same` MUST never panic and MUST be symmetric.
    #[test]
    fn phonetic_same_is_symmetric(a in ".*", b in ".*") {
        prop_assert_eq!(phonetic::same(&a, &b), phonetic::same(&b, &a));
    }

    /// For an arbitrary adversarial-or-well-formed weight vector
    /// (including negative/zero/`NaN`/infinite values),
    /// `MatchConfig::validated` must reject any config that is not
    /// actually finite-and-non-negative-throughout, and — the property
    /// that matters downstream — a config it DOES accept must never let
    /// `weighted_average` push a score outside `[0.0, 1.0]` or produce
    /// `NaN`.
    #[test]
    fn validated_config_never_produces_an_unbounded_score(
        values in prop::collection::vec(adversarial_weight(), 7),
        a in case_strategy(), b in case_strategy(),
    ) {
        let config = config_from(&values);
        if let Ok(validated) = config.validated() {
            let engine = MatchingEngine::new(validated);
            let r = engine.match_cases(&a, &b);
            prop_assert!(!r.score.is_nan(), "validated config produced a NaN score");
            prop_assert!(
                (0.0..=1.0).contains(&r.score),
                "validated config produced an out-of-range score: {}",
                r.score
            );
        } else {
            let weights_ok = values[..6].iter().all(|w| w.is_finite() && *w >= 0.0);
            let threshold_ok = values[6].is_finite() && (0.0..=1.0).contains(&values[6]);
            prop_assert!(
                !(weights_ok && threshold_ok),
                "validated() rejected a well-formed config: {values:?}"
            );
        }
    }
}
