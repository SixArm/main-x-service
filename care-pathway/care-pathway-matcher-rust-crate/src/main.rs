//! Demo binary — a runnable walkthrough of the `care-pathway-matcher`
//! public API. Not part of the SemVer surface.

// SEC-I3: this demo has no reason to reach for `unsafe`; forbid it so the
// crate's binary target matches the `#![forbid(unsafe_code)]` on its lib.
#![forbid(unsafe_code)]

// On MUSL static builds, swap the default allocator for mimalloc, which
// is markedly faster than the musl libc allocator. Gated to `musl` so
// glibc/macOS builds keep the system allocator. Demo-only; the library
// itself sets no global allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use care_pathway_matcher::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, MatchConfig,
    MatchingEngine, PathwayIdentifier,
};

/// Demo entry point — runs four representative scenarios and prints the
/// score / confidence / breakdown for each, exercising both the
/// probabilistic and deterministic paths of the public API.
fn main() {
    // One engine with default weights/threshold drives every scenario.
    let engine = MatchingEngine::new(MatchConfig::default());

    println!("== care-pathway-matcher demo ==\n");

    // 1. Probabilistic path: near-identical names plus a shared ICD-10
    //    condition code and care setting should score high without any
    //    deterministic identifier.
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

    // 2. Deterministic path (R-0): a shared guideline id pins the score
    //    to 1.0 even though the names share almost nothing. Note the
    //    case difference ("NICE-NG128" vs "nice-ng128") — `fold` makes
    //    the values compare equal.
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

    // 3. Deterministic path (R-1): same provider + same normalised
    //    pathway code short-circuits. "STROKE-01" and "stroke 01"
    //    normalise to "STROKE01" (alphanumerics only, uppercased), so
    //    they match despite the hyphen/space and case difference.
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

    // 4. Negative control: two unrelated pathways should fall well below
    //    the threshold and report `is_match == false`.
    let a = CarePathway::new("Acute Stroke Care Pathway");
    let b = CarePathway::new("Diabetic Foot Ulcer Management");
    let r = engine.match_care_pathways(&a, &b);
    println!(
        "unrelated      : score={:.3}  is_match={}",
        r.score, r.is_match
    );
}
