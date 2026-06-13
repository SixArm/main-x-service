//! Demo binary — a runnable walkthrough of the `care-pathway-matcher`
//! public API. Not part of the SemVer surface.

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use care_pathway_matcher::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, MatchConfig,
    MatchingEngine, PathwayIdentifier,
};

fn main() {
    let engine = MatchingEngine::new(MatchConfig::default());

    println!("== care-pathway-matcher demo ==\n");

    // 1. Fuzzy name + shared condition code.
    let mut a = CarePathway::new("Acute Stroke Care Pathway");
    a.condition_codes = vec![ConditionCode {
        system: CodeSystem::Icd10,
        code: "I63".into(),
    }];
    a.care_setting = Some(CareSetting::Inpatient);
    let mut b = CarePathway::new("Acute Stroke Pathway");
    b.condition_codes = vec![ConditionCode {
        system: CodeSystem::Icd10,
        code: "I63".into(),
    }];
    b.care_setting = Some(CareSetting::Inpatient);
    let r = engine.match_care_pathways(&a, &b);
    println!(
        "name+condition : score={:.3}  {:?}  is_match={}",
        r.score, r.confidence, r.is_match
    );

    // 2. Deterministic guideline-id short-circuit despite different names.
    let mut a = CarePathway::new("Stroke");
    let mut b = CarePathway::new("Cerebrovascular accident management");
    a.identifiers.push(PathwayIdentifier {
        scheme: IdentifierScheme::GuidelineId,
        value: "NICE-NG128".into(),
    });
    b.identifiers.push(PathwayIdentifier {
        scheme: IdentifierScheme::GuidelineId,
        value: "nice-ng128".into(),
    });
    let r = engine.match_care_pathways(&a, &b);
    println!(
        "guideline id   : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 3. Same-provider pathway code.
    let mut a = CarePathway::new("Stroke v1");
    let mut b = CarePathway::new("Stroke v2");
    a.provider_id = Some("trust-1".into());
    b.provider_id = Some("trust-1".into());
    a.pathway_code = Some("STROKE-01".into());
    b.pathway_code = Some("stroke 01".into());
    let r = engine.match_care_pathways(&a, &b);
    println!(
        "pathway code   : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 4. Unrelated.
    let a = CarePathway::new("Acute Stroke Care Pathway");
    let b = CarePathway::new("Diabetic Foot Ulcer Management");
    let r = engine.match_care_pathways(&a, &b);
    println!(
        "unrelated      : score={:.3}  is_match={}",
        r.score, r.is_match
    );
}
