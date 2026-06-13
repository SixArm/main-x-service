//! Integration tests driving the public re-exported surface only —
//! everything reachable via `use case_matcher::…`.

use case_matcher::{
    Case, CaseIdentifier, CaseStatus, CaseType, Confidence, IdentifierScheme, MatchConfig,
    MatchingEngine,
};

fn ident(scheme: IdentifierScheme, value: &str) -> CaseIdentifier {
    CaseIdentifier {
        scheme,
        value: value.into(),
    }
}

#[test]
fn r0_fires_for_every_deterministic_scheme() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Docket,
        IdentifierScheme::ExternalCaseId,
        IdentifierScheme::Uri,
        IdentifierScheme::Uuid,
    ] {
        let mut a = Case::new("Left");
        let mut b = Case::new("Right");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_cases(&a, &b);
        assert!(
            r.breakdown.deterministic_match,
            "{scheme:?} should short-circuit"
        );
        assert_eq!(r.score, 1.0);
        assert_eq!(r.confidence, Confidence::High);
    }
}

#[test]
fn agency_scoped_and_custom_schemes_do_not_short_circuit() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::AgencyCaseNumber,
        IdentifierScheme::LocalId,
        IdentifierScheme::Custom("LegacySystem".into()),
    ] {
        let mut a = Case::new("Alpha case");
        let mut b = Case::new("Beta case");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_cases(&a, &b);
        assert!(
            !r.breakdown.deterministic_match,
            "{scheme:?} must not short-circuit"
        );
    }
}

#[test]
fn same_agency_case_number_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Benefit claim v1");
    let mut b = Case::new("Benefit claim v2");
    a.agency_id = Some("agency-1".into());
    b.agency_id = Some("agency-1".into());
    a.case_number = Some("CV-2024-001234".into());
    b.case_number = Some("cv 2024 001234".into());
    let r = engine.match_cases(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn case_number_does_not_cross_match_across_agencies() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Alpha");
    let mut b = Case::new("Beta");
    a.agency_id = Some("agency-1".into());
    b.agency_id = Some("agency-2".into());
    a.case_number = Some("CV-2024-001234".into());
    b.case_number = Some("CV-2024-001234".into());
    let r = engine.match_cases(&a, &b);
    assert!(!r.breakdown.deterministic_match);
    assert!(r.breakdown.case_number_score.is_none());
}

#[test]
fn same_as_url_overlap_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Anything");
    let mut b = Case::new("Anything Else");
    a.same_as = vec!["https://courts.example.gov/case/CV-2024-001234".into()];
    b.same_as = vec!["https://courts.example.gov/case/CV-2024-001234".into()];
    let r = engine.match_cases(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn subjects_and_type_corroborate_title() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Housing benefit appeal — J. Smith");
    let mut b = Case::new("Housing benefit appeal — John Smith");
    a.subjects = vec!["person:pid-42".into()];
    b.subjects = vec!["person:pid-42".into()];
    a.case_type = Some(CaseType::Housing);
    b.case_type = Some(CaseType::Housing);
    let r = engine.match_cases(&a, &b);
    assert_eq!(r.breakdown.subjects_score, Some(1.0));
    assert_eq!(r.breakdown.case_type_score, Some(1.0));
    assert!(r.is_match, "got {}", r.score);
}

#[test]
fn renormalisation_ignores_absent_components() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Child protection referral — Doe family");
    let mut b = Case::new("Child protection referral — Doe family");
    a.subjects = vec!["person:99".into()];
    b.subjects = vec!["person:99".into()];
    let r = engine.match_cases(&a, &b);
    assert!(r.score >= 0.99, "got {}", r.score);
    assert!(r.breakdown.title_score.is_some());
    assert_eq!(r.breakdown.subjects_score, Some(1.0));
    assert!(r.breakdown.case_number_score.is_none());
    assert!(r.breakdown.keywords_score.is_none());
    assert!(r.breakdown.status_score.is_none());
}

#[test]
fn unrelated_cases_are_non_matches() {
    let engine = MatchingEngine::default_config();
    let a = Case::new("Housing benefit appeal — J. Smith");
    let b = Case::new("Import licence application — Acme Ltd");
    let r = engine.match_cases(&a, &b);
    assert!(!r.is_match);
    assert!(!r.breakdown.deterministic_match);
}

#[test]
fn status_distinguishes_otherwise_similar_cases() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Benefit reassessment");
    let mut b = Case::new("Benefit reassessment");
    a.status = Some(CaseStatus::Open);
    b.status = Some(CaseStatus::Closed);
    let r = engine.match_cases(&a, &b);
    assert_eq!(r.breakdown.status_score, Some(0.0));
}

#[test]
fn strict_and_lenient_change_is_match_not_score() {
    let mut a = Case::new("Disability benefit appeal");
    let mut b = Case::new("Disability benefit review appeal");
    a.subjects = vec!["person:7".into()];
    b.subjects = vec!["person:7".into()];
    let default = MatchingEngine::new(MatchConfig::default()).match_cases(&a, &b);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_cases(&a, &b);
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_cases(&a, &b);
    assert!((default.score - strict.score).abs() < 1e-9);
    assert!((default.score - lenient.score).abs() < 1e-9);
    if strict.is_match {
        assert!(default.is_match && lenient.is_match);
    }
}

#[test]
fn one_to_many_surface() {
    let engine = MatchingEngine::default_config();
    let query = Case::new("Housing benefit appeal — J. Smith");
    let cands = vec![
        Case::new("Import licence application — Acme Ltd"),
        Case::new("Housing benefit appeal — J. Smith"),
    ];
    let out = engine.match_one_to_many(&query, &cands);
    assert_eq!(out.len(), 2);
    assert!(out[1].score > out[0].score);
    let ranked = engine.rank(&query, &cands);
    assert_eq!(ranked[0].0, 1);
    assert!(engine.match_one_to_many(&query, &[]).is_empty());
    assert!(engine.find_matches(&query, &[]).is_empty());
}

#[test]
fn match_result_serialises_to_json() {
    let engine = MatchingEngine::default_config();
    let a = Case::new("Housing benefit appeal — J. Smith");
    let b = Case::new("Housing benefit appeal — John Smith");
    let r = engine.match_cases(&a, &b);
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("score"));
    assert!(json.contains("breakdown"));
}
