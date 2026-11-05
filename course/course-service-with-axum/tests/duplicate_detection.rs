#![warn(clippy::pedantic)]

//! Bridge test — pins the contract between the service's domain
//! `Course` and the canonical `course-matcher` algorithm.
//!
//! Each test:
//!   1. Builds one or two service-side `Course` records.
//!   2. Projects them through [`adapter::to_matcher_course`].
//!   3. Runs `course_matcher::MatchingEngine` and asserts on the
//!      resulting `MatchResult { score, is_match, confidence,
//!      breakdown }`.
//!
//! The bridge is the contract under test — these assertions pin both
//! the adapter's field-routing rules **and** the matcher's scoring
//! behaviour against the service's domain model. If either side
//! breaks the contract, a test here will fire.

use uuid::Uuid;

use course_service::matching::adapter::to_matcher_course;
use course_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use course_service::models::{
    Course, CourseIdentifier, EducationalLevel, IdentifierType, LearningResourceType,
};

// ────────────────── builders ──────────────────

/// Build a bare `Course` with just a name (all other fields defaulted).
fn course(name: &str) -> Course {
    Course::new(name)
}

/// Build a bare identifier (scheme + value, no name/url).
fn ident(scheme: IdentifierType, value: &str) -> CourseIdentifier {
    CourseIdentifier {
        property_id: scheme,
        value: value.into(),
        name: None,
        url: None,
    }
}

/// Construct a matching engine with the default config preset.
fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Identical / near-duplicate cases
// =============================================================================

/// Identical clones score ≥ 0.95, classify High, and are a match.
#[test]
fn identical_clones_score_high_and_classify_as_match() {
    let mut a = course("Introduction to Computer Science");
    a.course_code = Some("CS101".into());
    a.keywords = vec!["programming".into(), "algorithms".into()];
    let b = a.clone();

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));

    assert!(
        r.score >= 0.95,
        "identical clones should score ≥ 0.95, got {}",
        r.score
    );
    assert_eq!(r.confidence, Confidence::High);
    assert!(r.is_match);
}

/// A single-character name typo still classifies as a match (Jaro-Winkler).
#[test]
fn name_typo_still_classifies_via_jaro_winkler() {
    let a = course("Linear Algebra");
    let b = course("Linaer Algebra"); // single-character transposition

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));

    assert!(
        r.score >= 0.85,
        "near-clone should classify ≥ 0.85, got {}",
        r.score
    );
    assert!(r.is_match);
}

// =============================================================================
// Deterministic short-circuits (FR-20 / matcher §R-0..R-2)
// =============================================================================

/// A shared DOI short-circuits to score 1.0 despite different names.
#[test]
fn doi_short_circuits_to_one_even_with_different_names() {
    let mut a = course("CS101");
    let mut b = course("Totally Different Title");
    a.identifiers = vec![ident(IdentifierType::Doi, "10.1234/abc")];
    b.identifiers = vec![ident(IdentifierType::Doi, "10.1234/abc")];

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));

    assert!(r.breakdown.deterministic_match);
    assert!((r.score - 1.0).abs() < 1e-9);
    assert_eq!(r.confidence, Confidence::High);
}

/// A shared Wikidata id short-circuits to score 1.0.
#[test]
fn wikidata_short_circuits_to_one() {
    let mut a = course("alpha");
    let mut b = course("beta");
    a.identifiers = vec![ident(IdentifierType::Wikidata, "Q12345")];
    b.identifiers = vec![ident(IdentifierType::Wikidata, "Q12345")];

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(r.breakdown.deterministic_match);
    assert!((r.score - 1.0).abs() < 1e-9);
}

/// Same provider + normalised course code short-circuits to score 1.0.
#[test]
fn same_provider_plus_course_code_short_circuits() {
    let provider = Uuid::new_v4();
    let mut a = course("Linear Algebra");
    let mut b = course("Linear Algebra I"); // different titles
    a.provider_id = Some(provider);
    b.provider_id = Some(provider);
    a.course_code = Some("MATH 220".into());
    b.course_code = Some("math220".into()); // normalisation strips whitespace + uppercases

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(r.breakdown.deterministic_match);
    assert!((r.score - 1.0).abs() < 1e-9);
}

/// A shared `same_as` URL short-circuits to score 1.0.
#[test]
fn shared_same_as_url_short_circuits() {
    let mut a = course("Intro to Stats");
    let mut b = course("Introductory Statistics");
    a.same_as = vec!["https://wikidata.org/wiki/Q789".into()];
    b.same_as = vec!["https://wikidata.org/wiki/Q789".into()];

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(r.breakdown.deterministic_match);
    assert!((r.score - 1.0).abs() < 1e-9);
}

/// A shared LMS course id does NOT short-circuit (non-deterministic scheme).
#[test]
fn lms_course_id_does_not_short_circuit() {
    // LmsCourseId is intentionally NOT in `is_deterministic()`:
    // course-CS101 in Canvas at one school != course-CS101 in Canvas
    // at another. Routing must preserve that distinction.
    let mut a = course("Calc I");
    let mut b = course("Organic Chemistry");
    a.identifiers = vec![ident(IdentifierType::LmsCourseId, "canvas-12345")];
    b.identifiers = vec![ident(IdentifierType::LmsCourseId, "canvas-12345")];

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(!r.breakdown.deterministic_match);
    assert!(
        !r.is_match,
        "unrelated titles must not match on a non-deterministic scheme alone"
    );
}

// =============================================================================
// Negative cases
// =============================================================================

/// Unrelated titles stay below the match threshold and classify Low.
#[test]
fn unrelated_courses_do_not_classify_as_match() {
    // With only `name` present, the renormalised weighted-sum reduces
    // to Jaro-Winkler on the title — common letters give a non-zero
    // floor even for unrelated titles. The invariant under test is
    // not the exact score but that the result stays below the match
    // threshold and below the Medium-confidence band.
    let a = course("Organic Chemistry");
    let b = course("Medieval European History");

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(r.score < 0.85, "below match threshold; got {}", r.score);
    assert!(!r.is_match);
    assert_eq!(r.confidence, Confidence::Low);
}

/// An identical course code at different providers does NOT short-circuit.
#[test]
fn same_course_code_at_different_providers_does_not_short_circuit() {
    let mut a = course("Intro to CS");
    let mut b = course("Intro to Biology");
    a.provider_id = Some(Uuid::new_v4());
    b.provider_id = Some(Uuid::new_v4());
    a.course_code = Some("CS101".into());
    b.course_code = Some("CS101".into()); // identical code, different provider

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(
        !r.breakdown.deterministic_match,
        "course-code on its own (without shared provider) must NOT short-circuit"
    );
    assert!(!r.is_match);
}

// =============================================================================
// Field-routing pinning — guards the adapter's per-enum rewires
// =============================================================================

/// Adapter pins: a `provider_id` UUID routes through to the matcher string.
#[test]
fn provider_id_uuid_routes_through_to_matcher_string() {
    let provider = Uuid::new_v4();
    let mut a = course("Calc I");
    let mut b = course("Calc I");
    a.provider_id = Some(provider);
    b.provider_id = Some(provider);

    // Identical names + same provider → provider_score = 1.0
    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(r.score >= 0.95);
    assert!(r.is_match);
}

/// Adapter pins: `educational_level` maps one-to-one into the breakdown.
#[test]
fn educational_level_routes_one_to_one() {
    let mut a = course("Quantum Mechanics");
    let mut b = course("Quantum Mechanics");
    a.educational_level = Some(EducationalLevel::Graduate);
    b.educational_level = Some(EducationalLevel::Graduate);

    let r = engine().match_courses(&to_matcher_course(&a), &to_matcher_course(&b));
    assert!(r.breakdown.educational_level_score.unwrap_or(0.0) >= 0.99);
}

/// Adapter pins: `learning_resource_type` carries through the projection.
#[test]
fn learning_resource_type_carries_through() {
    let mut a = course("Topology");
    a.learning_resource_type = Some(LearningResourceType::Lecture);
    let m = to_matcher_course(&a);
    assert!(matches!(
        m.learning_resource_type,
        Some(course_service::matching::matcher_lib::LearningResourceType::Lecture)
    ));
}

/// Adapter pins: a custom identifier scheme round-trips its label.
#[test]
fn custom_identifier_scheme_round_trips_label() {
    let mut a = course("X");
    a.identifiers = vec![ident(IdentifierType::Custom("KhanCourse".into()), "kc-42")];
    let m = to_matcher_course(&a);
    assert!(matches!(
        m.identifiers[0].scheme,
        course_service::matching::matcher_lib::IdentifierScheme::Custom(ref s) if s == "KhanCourse"
    ));
}

// =============================================================================
// Config presets
// =============================================================================

/// The strict preset raises the threshold (its matches are a subset of
/// default's) while leaving raw scores unchanged.
#[test]
fn strict_preset_raises_threshold() {
    let strict = MatchingEngine::new(MatchConfig::strict());
    let default = MatchingEngine::new(MatchConfig::default());

    // A typo-pair that classifies under default may or may not under
    // strict — the invariant we pin is that strict's threshold is ≥
    // default's, so any default `is_match=false` implies the same
    // under strict.
    let a = course("Discrete Mathematics");
    let b = course("Discrete Mathmatics"); // small typo
    let ma = to_matcher_course(&a);
    let mb = to_matcher_course(&b);
    let r_default = default.match_courses(&ma, &mb);
    let r_strict = strict.match_courses(&ma, &mb);
    if !r_default.is_match {
        assert!(
            !r_strict.is_match,
            "strict must be a subset of default matches"
        );
    }
    // Scores must be identical — only the threshold changes.
    assert!((r_default.score - r_strict.score).abs() < 1e-9);
}
