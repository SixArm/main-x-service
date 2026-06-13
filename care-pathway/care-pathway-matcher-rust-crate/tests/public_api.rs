//! Integration tests driving the public re-exported surface only —
//! everything reachable via `use care_pathway_matcher::…`.

use care_pathway_matcher::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, Confidence, IdentifierScheme, MatchConfig,
    MatchingEngine, PathwayIdentifier,
};

fn ident(scheme: IdentifierScheme, value: &str) -> PathwayIdentifier {
    PathwayIdentifier {
        scheme,
        value: value.into(),
    }
}

fn cond(system: CodeSystem, code: &str) -> ConditionCode {
    ConditionCode {
        system,
        code: code.into(),
    }
}

#[test]
fn r0_fires_for_every_deterministic_scheme() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Doi,
        IdentifierScheme::Wikidata,
        IdentifierScheme::GuidelineId,
        IdentifierScheme::Uri,
        IdentifierScheme::Uuid,
    ] {
        let mut a = CarePathway::new("Left");
        let mut b = CarePathway::new("Right");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_care_pathways(&a, &b);
        assert!(
            r.breakdown.deterministic_match,
            "{scheme:?} should short-circuit"
        );
        assert_eq!(r.score, 1.0);
        assert_eq!(r.confidence, Confidence::High);
    }
}

#[test]
fn provider_scoped_and_custom_schemes_do_not_short_circuit() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::PathwayCode,
        IdentifierScheme::LocalId,
        IdentifierScheme::Custom("MapOfMedicine".into()),
    ] {
        let mut a = CarePathway::new("Alpha pathway");
        let mut b = CarePathway::new("Beta pathway");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_care_pathways(&a, &b);
        assert!(
            !r.breakdown.deterministic_match,
            "{scheme:?} must not short-circuit"
        );
    }
}

#[test]
fn same_provider_pathway_code_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = CarePathway::new("Stroke v1");
    let mut b = CarePathway::new("Stroke v2");
    a.provider_id = Some("trust-1".into());
    b.provider_id = Some("trust-1".into());
    a.pathway_code = Some("STROKE-01".into());
    b.pathway_code = Some("stroke 01".into());
    let r = engine.match_care_pathways(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn same_as_url_overlap_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = CarePathway::new("Anything");
    let mut b = CarePathway::new("Anything Else");
    a.same_as = vec!["https://www.nice.org.uk/guidance/ng128".into()];
    b.same_as = vec!["https://www.nice.org.uk/guidance/ng128".into()];
    let r = engine.match_care_pathways(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn condition_and_setting_corroborate_name() {
    let engine = MatchingEngine::default_config();
    let mut a = CarePathway::new("Acute Stroke Care Pathway");
    let mut b = CarePathway::new("Acute Stroke Pathway");
    a.condition_codes = vec![cond(CodeSystem::Icd10, "I63")];
    b.condition_codes = vec![cond(CodeSystem::Icd10, "i63")];
    a.care_setting = Some(CareSetting::Inpatient);
    b.care_setting = Some(CareSetting::Inpatient);
    let r = engine.match_care_pathways(&a, &b);
    assert_eq!(r.breakdown.condition_score, Some(1.0));
    assert_eq!(r.breakdown.care_setting_score, Some(1.0));
    assert!(r.is_match, "got {}", r.score);
}

#[test]
fn renormalisation_ignores_absent_components() {
    let engine = MatchingEngine::default_config();
    let mut a = CarePathway::new("Sepsis Six Pathway");
    let mut b = CarePathway::new("Sepsis Six Pathway");
    a.condition_codes = vec![cond(CodeSystem::Snomed, "91302008")];
    b.condition_codes = vec![cond(CodeSystem::Snomed, "91302008")];
    let r = engine.match_care_pathways(&a, &b);
    assert!(r.score >= 0.99, "got {}", r.score);
    assert!(r.breakdown.name_score.is_some());
    assert_eq!(r.breakdown.condition_score, Some(1.0));
    assert!(r.breakdown.pathway_code_score.is_none());
    assert!(r.breakdown.keywords_score.is_none());
}

#[test]
fn unrelated_pathways_are_non_matches() {
    let engine = MatchingEngine::default_config();
    let a = CarePathway::new("Acute Stroke Care Pathway");
    let b = CarePathway::new("Antenatal Care Pathway");
    let r = engine.match_care_pathways(&a, &b);
    assert!(!r.is_match);
    assert!(!r.breakdown.deterministic_match);
}

#[test]
fn strict_and_lenient_change_is_match_not_score() {
    let mut a = CarePathway::new("Heart Failure Pathway");
    let mut b = CarePathway::new("Heart Failure Care Pathway");
    a.condition_codes = vec![cond(CodeSystem::Icd10, "I50")];
    b.condition_codes = vec![cond(CodeSystem::Icd10, "I50")];
    let default = MatchingEngine::new(MatchConfig::default()).match_care_pathways(&a, &b);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_care_pathways(&a, &b);
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_care_pathways(&a, &b);
    assert!((default.score - strict.score).abs() < 1e-9);
    assert!((default.score - lenient.score).abs() < 1e-9);
    if strict.is_match {
        assert!(default.is_match && lenient.is_match);
    }
}

#[test]
fn one_to_many_surface() {
    let engine = MatchingEngine::default_config();
    let query = CarePathway::new("Acute Stroke Care Pathway");
    let cands = vec![
        CarePathway::new("Antenatal Care Pathway"),
        CarePathway::new("Acute Stroke Care Pathway"),
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
    let a = CarePathway::new("Acute Stroke Care Pathway");
    let b = CarePathway::new("Acute Stroke Pathway");
    let r = engine.match_care_pathways(&a, &b);
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("score"));
    assert!(json.contains("breakdown"));
}
