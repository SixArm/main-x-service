//! Integration tests driving the public re-exported surface only.
//!
//! Unit tests (in `src/`) pin component-level behaviour against private
//! functions. This suite is the contract test for the crate's public
//! API — everything reachable via `use course_matcher::…`. A rename of
//! any re-export breaks it.

use course_matcher::{
    Confidence, Course, CourseIdentifier, EducationalLevel, IdentifierScheme, LearningResourceType,
    MatchConfig, MatchingEngine,
};

/// Test helper: build a `CourseIdentifier` from a scheme + value,
/// exercising only the public re-exported types.
fn ident(scheme: IdentifierScheme, value: &str) -> CourseIdentifier {
    CourseIdentifier {
        scheme,
        value: value.into(),
    }
}

// ─── Deterministic short-circuits (R-0 / R-1 / R-2) ──────────────────

#[test]
fn worked_example_r1_same_provider_course_code() {
    // The worked example from AGENTS/matching-algorithm.md.
    let engine = MatchingEngine::new(MatchConfig::default());

    let mut a = Course::new("Introduction to Computer Science");
    a.course_code = Some("CS101".into());
    a.provider_id = Some("ror-021nxhr62".into());
    a.keywords = vec!["computer science".into(), "programming".into()];

    let mut b = Course::new("CS101");
    b.course_code = Some("cs 101".into());
    b.provider_id = Some("ror-021nxhr62".into());

    let r = engine.match_courses(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
    assert_eq!(r.confidence, Confidence::High);
    assert!(r.is_match);
}

// Pins R-0 via the public API: a shared DOI wins over unrelated names.
#[test]
fn r0_doi_identifier_short_circuit_beats_unrelated_names() {
    let engine = MatchingEngine::default_config();
    let mut a = Course::new("Quantum Field Theory");
    let mut b = Course::new("Introductory Basket Weaving");
    a.identifiers
        .push(ident(IdentifierScheme::Doi, "10.1234/intro-cs"));
    b.identifiers
        .push(ident(IdentifierScheme::Doi, "10.1234/intro-cs"));
    let r = engine.match_courses(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

// Pins that EVERY deterministic scheme short-circuits to 1.0 — guards
// against one being accidentally dropped from `is_deterministic`.
#[test]
fn r0_fires_for_every_deterministic_scheme() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Doi,
        IdentifierScheme::Wikidata,
        IdentifierScheme::Lom,
        IdentifierScheme::Oer,
        IdentifierScheme::Uri,
        IdentifierScheme::Uuid,
    ] {
        let mut a = Course::new("Left");
        let mut b = Course::new("Right");
        a.identifiers
            .push(ident(scheme.clone(), "shared-value-123"));
        b.identifiers
            .push(ident(scheme.clone(), "shared-value-123"));
        let r = engine.match_courses(&a, &b);
        assert!(
            r.breakdown.deterministic_match,
            "{scheme:?} should short-circuit"
        );
        assert_eq!(r.score, 1.0);
    }
}

// Pins the inverse: provider-scoped schemes must NOT short-circuit —
// a false 1.0 here is the worst-case bug.
#[test]
fn provider_scoped_identifier_does_not_short_circuit() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::LmsCourseId,
        IdentifierScheme::PlatformSlug,
        IdentifierScheme::Isced,
        IdentifierScheme::Ror,
        IdentifierScheme::Custom("Khan".into()),
    ] {
        let mut a = Course::new("Left");
        let mut b = Course::new("Right");
        a.identifiers
            .push(ident(scheme.clone(), "shared-value-123"));
        b.identifiers
            .push(ident(scheme.clone(), "shared-value-123"));
        let r = engine.match_courses(&a, &b);
        assert!(
            !r.breakdown.deterministic_match,
            "{scheme:?} must not short-circuit"
        );
    }
}

// Pins R-2 via the public API: overlapping same_as URLs pin to 1.0.
#[test]
fn r2_same_as_url_overlap_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = Course::new("Anything");
    let mut b = Course::new("Anything Else");
    a.same_as = vec!["https://oercommons.org/courses/60132".into()];
    b.same_as = vec!["https://oercommons.org/courses/60132".into()];
    let r = engine.match_courses(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

// ─── Probabilistic path ──────────────────────────────────────────────

// Pins that a fully-populated clone is High confidence (here via R-1).
#[test]
fn identical_fully_populated_courses_are_high_confidence() {
    let engine = MatchingEngine::default_config();
    let mut a = Course::new("Data Structures and Algorithms");
    a.course_code = Some("CS201".into());
    a.provider_id = Some("prov-1".into());
    a.educational_level = Some(EducationalLevel::Undergraduate);
    a.learning_resource_type = Some(LearningResourceType::Lecture);
    a.keywords = vec!["algorithms".into(), "data structures".into()];
    a.teaches = vec!["sorting".into(), "graphs".into()];
    let b = a.clone();

    let r = engine.match_courses(&a, &b);
    // Identical provider + course_code → R-1 short-circuits to 1.0.
    assert_eq!(r.score, 1.0);
    assert_eq!(r.confidence, Confidence::High);
    assert!(r.is_match);
}

#[test]
fn renormalisation_name_and_provider_only() {
    // Only name + provider_id present; both identical. The absent
    // components must not dilute the score below 1.0.
    let engine = MatchingEngine::default_config();
    let mut a = Course::new("Thermodynamics");
    let mut b = Course::new("Thermodynamics");
    a.provider_id = Some("prov-7".into());
    b.provider_id = Some("prov-7".into());
    let r = engine.match_courses(&a, &b);
    assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
    assert!(r.breakdown.name_score.is_some());
    assert_eq!(r.breakdown.provider_score, Some(1.0));
    assert!(r.breakdown.keywords_score.is_none());
    assert!(r.breakdown.teaches_score.is_none());
    assert!(r.breakdown.educational_level_score.is_none());
}

// Pins the negative path: unrelated courses are Low, non-match,
// non-deterministic, and still in-range.
#[test]
fn unrelated_courses_are_low_confidence_non_matches() {
    let engine = MatchingEngine::default_config();
    let a = Course::new("Linear Algebra");
    let b = Course::new("Tudor History 1485-1603");
    let r = engine.match_courses(&a, &b);
    assert!(!r.is_match);
    assert_eq!(r.confidence, Confidence::Low);
    assert!(!r.breakdown.deterministic_match);
    assert!((0.0..=1.0).contains(&r.score));
}

// Pins that a small singular/plural typo still clears the 0.85 default.
#[test]
fn typo_variant_clears_default_threshold() {
    let engine = MatchingEngine::default_config();
    let a = Course::new("Introduction to Computer Science");
    let b = Course::new("Introduction to Computer Sciences");
    let r = engine.match_courses(&a, &b);
    assert!(r.score > 0.85, "expected probable match, got {}", r.score);
    assert!(r.is_match);
}

// ─── Threshold presets ───────────────────────────────────────────────

// Pins that presets move only `is_match` (via the threshold), never
// the underlying score, and that strict ⊆ default ⊆ lenient.
#[test]
fn strict_and_lenient_change_is_match_not_score() {
    let a = Course::new("Introduction to Computer Science");
    let b = Course::new("Introduction to Computer Sciences");

    let default = MatchingEngine::new(MatchConfig::default()).match_courses(&a, &b);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_courses(&a, &b);
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_courses(&a, &b);

    // The underlying score is identical regardless of threshold.
    assert!((default.score - strict.score).abs() < 1e-9);
    assert!((default.score - lenient.score).abs() < 1e-9);

    // Lenient is the most permissive; strict the least.
    assert!(lenient.is_match || !default.is_match);
    if strict.is_match {
        assert!(default.is_match && lenient.is_match);
    }
}

// ─── One-to-many surface ─────────────────────────────────────────────

// Pins that one-to-many keeps candidate order (the exact match stays
// at its original index, not floated to the top).
#[test]
fn match_one_to_many_preserves_order() {
    let engine = MatchingEngine::default_config();
    let query = Course::new("Organic Chemistry");
    let cands = vec![
        Course::new("Medieval Poetry"),
        Course::new("Organic Chemistry"),
        Course::new("Inorganic Chemistry"),
    ];
    let out = engine.match_one_to_many(&query, &cands);
    assert_eq!(out.len(), 3);
    // Index 1 is the exact match — its score is the highest.
    assert!(out[1].score > out[0].score);
    assert!(out[1].score > out[2].score);
}

// Pins that `rank` sorts descending while keeping original indices in
// the returned tuples.
#[test]
fn rank_sorts_descending_and_preserves_indices() {
    let engine = MatchingEngine::default_config();
    let query = Course::new("Organic Chemistry");
    let cands = vec![
        Course::new("Medieval Poetry"),
        Course::new("Organic Chemistry"),
        Course::new("Inorganic Chemistry"),
    ];
    let ranked = engine.rank(&query, &cands);
    assert_eq!(ranked.len(), 3);
    assert_eq!(ranked[0].0, 1); // exact match, original index 1
    // Scores are sorted descending.
    assert!(ranked[0].1.score >= ranked[1].1.score);
    assert!(ranked[1].1.score >= ranked[2].1.score);
}

// Pins that `find_matches` drops below-threshold candidates entirely.
#[test]
fn find_matches_returns_only_passing_candidates() {
    let engine = MatchingEngine::default_config();
    let query = Course::new("Organic Chemistry");
    let cands = vec![
        Course::new("Medieval Poetry"),
        Course::new("Organic Chemistry"),
        Course::new("Quantum Computing"),
    ];
    let matches = engine.find_matches(&query, &cands);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, 1);
    assert!(matches[0].1.is_match);
}

// Pins that all three one-to-many entry points handle an empty slice.
#[test]
fn one_to_many_handles_empty_candidates() {
    let engine = MatchingEngine::default_config();
    let query = Course::new("Anything");
    assert!(engine.match_one_to_many(&query, &[]).is_empty());
    assert!(engine.rank(&query, &[]).is_empty());
    assert!(engine.find_matches(&query, &[]).is_empty());
}

// ─── Result serialisation ────────────────────────────────────────────

// Pins that a `MatchResult` serialises with its key fields present —
// the service layer relies on this JSON shape.
#[test]
fn match_result_serialises_to_json() {
    let engine = MatchingEngine::default_config();
    let a = Course::new("Introduction to Computer Science");
    let b = Course::new("Intro to Computer Science");
    let r = engine.match_courses(&a, &b);
    let json = serde_json::to_string(&r).expect("serialize MatchResult");
    assert!(json.contains("score"));
    assert!(json.contains("breakdown"));
    assert!(json.contains("confidence"));
}
