//! DB-free tests: the service embeds `case-matcher` directly and
//! persists `Case` as JSON, so both the matcher contract and the storage
//! round-trip can be checked without a database.

use case_matcher::{Case, CaseIdentifier, IdentifierScheme, MatchingEngine};

#[test]
fn embeds_the_canonical_matcher() {
    let engine = MatchingEngine::default_config();
    let mut a = Case::new("Housing benefit appeal — J. Smith");
    let mut b = Case::new("HB appeal — John Smith");
    a.identifiers.push(CaseIdentifier {
        scheme: IdentifierScheme::Docket,
        value: "CV-2024-001234".into(),
    });
    b.identifiers.push(CaseIdentifier {
        scheme: IdentifierScheme::Docket,
        value: "CV-2024-001234".into(),
    });
    let r = engine.match_cases(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn case_json_round_trips_for_storage() {
    let mut c = Case::new("Housing benefit appeal");
    c.agency_id = Some("dwp".into());
    c.case_number = Some("HB-2024-0007".into());
    let value = serde_json::to_value(&c).expect("to json");
    let back: Case = serde_json::from_value(value).expect("from json");
    assert_eq!(back.title, c.title);
    assert_eq!(back.agency_id, c.agency_id);
    assert_eq!(back.case_number, c.case_number);
}
