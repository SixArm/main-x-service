//! DB-free tests: the service embeds `care-pathway-matcher` directly and
//! persists `CarePathway` as JSON, so both the matcher contract and the
//! storage round-trip can be checked without a database.

use care_pathway_matcher::{CarePathway, IdentifierScheme, MatchingEngine, PathwayIdentifier};

#[test]
fn embeds_the_canonical_matcher() {
    let engine = MatchingEngine::default_config();
    let mut a = CarePathway::new("Acute Stroke Care Pathway");
    let mut b = CarePathway::new("Cerebrovascular accident pathway");
    a.identifiers.push(PathwayIdentifier {
        scheme: IdentifierScheme::GuidelineId,
        value: "NICE-NG128".into(),
    });
    b.identifiers.push(PathwayIdentifier {
        scheme: IdentifierScheme::GuidelineId,
        value: "nice-ng128".into(),
    });
    let r = engine.match_care_pathways(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn care_pathway_json_round_trips_for_storage() {
    let mut p = CarePathway::new("Acute Stroke Care Pathway");
    p.provider_id = Some("trust-1".into());
    p.pathway_code = Some("STROKE-01".into());
    let value = serde_json::to_value(&p).expect("to json");
    let back: CarePathway = serde_json::from_value(value).expect("from json");
    assert_eq!(back.name, p.name);
    assert_eq!(back.provider_id, p.provider_id);
    assert_eq!(back.pathway_code, p.pathway_code);
}
