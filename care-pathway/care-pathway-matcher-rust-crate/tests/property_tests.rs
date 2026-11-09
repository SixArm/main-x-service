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
    CarePathway, CodeSystem, ConditionCode, IdentifierScheme, MAX_ARRAY_LEN, MatchConfig,
    MatchingEngine, PathwayIdentifier, RelationKind, RelationshipRef, normalize, phonetic,
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

/// A relation kind — the five fixed variants plus a free-form `Custom`
/// label (CPM-T2).
fn relation_kind() -> impl Strategy<Value = RelationKind> {
    prop_oneof![
        Just(RelationKind::PrecededBy),
        Just(RelationKind::FollowedBy),
        Just(RelationKind::SimilarTo),
        Just(RelationKind::Supersedes),
        Just(RelationKind::SupersededBy),
        text().prop_map(RelationKind::Custom),
    ]
}

/// A relationship reference with an arbitrary (possibly empty or
/// blank-folding) `pathway_id`, since `RelationshipRef` has no
/// validating constructor to route around (CPM-T2).
fn relationship() -> impl Strategy<Value = RelationshipRef> {
    (relation_kind(), text()).prop_map(|(relation, pathway_id)| RelationshipRef {
        relation,
        pathway_id,
    })
}

/// A `CarePathway` varying only the few string-bearing fields the task
/// calls out — name, pathway code, provider, condition codes, keywords,
/// relationships, tags — via lightweight direct field assignment (no
/// builder). Everything else stays at its `Default` so construction is
/// cheap.
fn pathway() -> impl Strategy<Value = CarePathway> {
    (
        text(),                   // name
        prop::option::of(text()), // pathway_code
        prop::option::of(text()), // provider_id
        prop::collection::vec(condition_code(), 0..3),
        prop::collection::vec(text(), 0..3),         // keywords
        prop::collection::vec(relationship(), 0..3), // relationships (CPM-T2)
        prop::collection::vec(text(), 0..3),         // tags (CPM-T2)
    )
        .prop_map(
            |(name, code, provider, conditions, keywords, relationships, tags)| {
                let mut p = CarePathway::new(name);
                p.pathway_code = code;
                p.provider_id = provider;
                p.condition_codes = conditions;
                p.keywords = keywords;
                p.relationships = relationships;
                p.tags = tags;
                p
            },
        )
}

/// A value that is sometimes well-formed (a small non-negative float,
/// suitable for either a weight or a threshold) and sometimes
/// deliberately adversarial (negative, `NaN`, infinite) — the values
/// `MatchConfig::validated` (CPM-T1) must reject, generated alongside
/// the ones it must accept.
fn adversarial_weight() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => 0.0f64..=1.0,
        1 => Just(f64::NAN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => -10.0f64..0.0,
    ]
}

/// Build a `MatchConfig` from 9 adversarial-or-well-formed values: the
/// 8 weights in field-declaration order, then the threshold.
fn config_from(values: &[f64]) -> MatchConfig {
    MatchConfig {
        threshold: values[8],
        name_weight: values[0],
        condition_weight: values[1],
        pathway_code_weight: values[2],
        care_setting_weight: values[3],
        interventions_weight: values[4],
        keywords_weight: values[5],
        relationships_weight: values[6],
        tags_weight: values[7],
    }
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

    /// CPM-T1: for an arbitrary adversarial-or-well-formed weight
    /// vector (including negative/zero/`NaN`/infinite values),
    /// `MatchConfig::validated` must reject any config that is not
    /// actually finite-and-non-negative-throughout, and — the property
    /// that matters downstream — a config it DOES accept must never
    /// let `weighted_average` push a score outside `[0.0, 1.0]` or
    /// produce `NaN`.
    #[test]
    fn validated_config_never_produces_an_unbounded_score(
        values in prop::collection::vec(adversarial_weight(), 9),
        a in pathway(), b in pathway(),
    ) {
        let config = config_from(&values);
        if let Ok(validated) = config.validated() {
            let engine = MatchingEngine::new(validated);
            let r = engine.match_care_pathways(&a, &b);
            prop_assert!(!r.score.is_nan(), "validated config produced a NaN score");
            prop_assert!(
                (0.0..=1.0).contains(&r.score),
                "validated config produced an out-of-range score: {}",
                r.score
            );
        } else {
            let weights_ok = values[..8].iter().all(|w| w.is_finite() && *w >= 0.0);
            let threshold_ok = values[8].is_finite() && (0.0..=1.0).contains(&values[8]);
            prop_assert!(
                !(weights_ok && threshold_ok),
                "validated() rejected a well-formed config: {values:?}"
            );
        }
    }
}

// A dedicated block with far fewer cases: each iteration below builds
// seven `n`-element `Vec`s (n up to ~2000) and runs real matches, so
// running it at the default 400 cases would make the suite slow without
// adding coverage — the property only needs a handful of large `n`
// samples to prove the cap, not hundreds.
proptest! {
    #![proptest_config(ProptestConfig { cases: 20, ..ProptestConfig::default() })]

    /// CPM-T3: no element past [`MAX_ARRAY_LEN`] reaches scoring, in any
    /// of the unbounded array fields (`identifiers`, `same_as`,
    /// `condition_codes`, `interventions`, `keywords`, `relationships`,
    /// `tags`), so pairwise cost is bounded by the cap however large the
    /// input. Proved by what the result depends on, not by a stopwatch
    /// (repo `tasks.md` LCI-9): the first [`MAX_ARRAY_LEN`] elements
    /// differ by side, and every element after them is **shared**,
    /// including a DOI identifier and a `same_as` URL. An engine that
    /// read past the cap would take the R-0 or R-2 short-circuit, or
    /// score the shared tails into every Jaccard; one that honours it
    /// returns exactly what the truncated records return.
    #[test]
    fn elements_past_the_cap_never_reach_scoring(n in (MAX_ARRAY_LEN + 1)..2000) {
        let a = oversized_pathway("a", n);
        let b = oversized_pathway("b", n);
        let engine = MatchingEngine::default_config();
        let r = engine.match_care_pathways(&a, &b);
        prop_assert!(
            !r.breakdown.deterministic_match,
            "a shared identifier or same_as URL past MAX_ARRAY_LEN ({MAX_ARRAY_LEN}) \
             fired a short-circuit for n={n}: the cap is not applied"
        );
        let truncated = engine.match_care_pathways(&truncate(&a), &truncate(&b));
        prop_assert_eq!(
            format!("{r:?}"),
            format!("{truncated:?}"),
            "elements past MAX_ARRAY_LEN ({}) changed the result for n={}",
            MAX_ARRAY_LEN,
            n
        );
    }
}

/// A pathway with `n` elements in every unbounded array field. The first
/// [`MAX_ARRAY_LEN`] are unique to `side`; the rest are the same on
/// every side, and the first identifier among them is a DOI, a
/// globally unique scheme that R-0 short-circuits on.
fn oversized_pathway(side: &str, n: usize) -> CarePathway {
    let tag = |i: usize| {
        if i < MAX_ARRAY_LEN {
            format!("{side}-{i}")
        } else {
            format!("shared-{i}")
        }
    };
    let mut p = CarePathway::new(format!("{side} Stroke Care Pathway"));
    p.condition_codes = (0..n)
        .map(|i| ConditionCode {
            system: CodeSystem::Icd10,
            code: format!("code-{}", tag(i)),
        })
        .collect();
    p.interventions = (0..n).map(|i| format!("intervention-{}", tag(i))).collect();
    p.keywords = (0..n).map(|i| format!("keyword-{}", tag(i))).collect();
    p.tags = (0..n).map(|i| format!("tag-{}", tag(i))).collect();
    p.relationships = (0..n)
        .map(|i| RelationshipRef {
            relation: RelationKind::SimilarTo,
            pathway_id: format!("pid-{}", tag(i)),
        })
        .collect();
    p.identifiers = (0..n)
        .map(|i| PathwayIdentifier {
            scheme: if i < MAX_ARRAY_LEN {
                IdentifierScheme::Custom("registry".into())
            } else {
                IdentifierScheme::Doi
            },
            value: format!("10.1234/{}", tag(i)),
        })
        .collect();
    p.same_as = (0..n)
        .map(|i| format!("https://example.org/{}", tag(i)))
        .collect();
    p
}

/// `p` with every unbounded array field cut to [`MAX_ARRAY_LEN`].
fn truncate(p: &CarePathway) -> CarePathway {
    let mut t = p.clone();
    t.condition_codes.truncate(MAX_ARRAY_LEN);
    t.interventions.truncate(MAX_ARRAY_LEN);
    t.keywords.truncate(MAX_ARRAY_LEN);
    t.tags.truncate(MAX_ARRAY_LEN);
    t.relationships.truncate(MAX_ARRAY_LEN);
    t.identifiers.truncate(MAX_ARRAY_LEN);
    t.same_as.truncate(MAX_ARRAY_LEN);
    t
}

/// The wall-clock form of the property above, kept as an opt-in check
/// (`cargo test -- --ignored`) because a loaded machine fails it: it
/// took 965 ms against a 500 ms ceiling in a full-tree run and passed
/// alone (repo `tasks.md` LCI-9). Values differ by side throughout, so
/// neither short-circuit fires and every field reaches scoring.
#[test]
#[ignore = "wall-clock bound; flaky under load, run on demand"]
fn oversized_arrays_score_in_bounded_time() {
    let n = 2000;
    let a = oversized_pathway_unshared("a", n);
    let b = oversized_pathway_unshared("b", n);
    let engine = MatchingEngine::default_config();
    let start = std::time::Instant::now();
    let r = engine.match_care_pathways(&a, &b);
    let elapsed = start.elapsed();
    assert!(!r.breakdown.deterministic_match);
    assert!(
        elapsed.as_secs_f64() < 0.5,
        "match_care_pathways took {elapsed:?} for n={n}"
    );
}

/// Like [`oversized_pathway`], with no element shared between sides.
fn oversized_pathway_unshared(side: &str, n: usize) -> CarePathway {
    let mut p = oversized_pathway(side, n);
    for (i, id) in p.identifiers.iter_mut().enumerate() {
        id.scheme = IdentifierScheme::Custom("registry".into());
        id.value = format!("{side}-id-{i}");
    }
    for (i, url) in p.same_as.iter_mut().enumerate() {
        *url = format!("https://{side}.example.org/{i}");
    }
    p
}
