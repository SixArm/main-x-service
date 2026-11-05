//! DB-free tests: the service embeds `organization-matcher` directly and
//! persists `Organization` as JSON, so both the matcher contract and the
//! storage round-trip can be checked without a database.

use organization_matcher::{IdentifierScheme, MatchingEngine, OrgIdentifier, Organization};

/// The service uses the canonical `organization-matcher`: two orgs sharing
/// an LEI deterministically match (score 1.0, deterministic short-circuit).
#[test]
fn embeds_the_canonical_matcher() {
    let engine = MatchingEngine::default_config();
    let mut a = Organization::new("Acme Corporation");
    let mut b = Organization::new("Globex");
    a.identifiers.push(OrgIdentifier {
        scheme: IdentifierScheme::Lei,
        value: "5493001KJTIIGC8Y1R12".into(),
    });
    b.identifiers.push(OrgIdentifier {
        scheme: IdentifierScheme::Lei,
        value: "5493001KJTIIGC8Y1R12".into(),
    });
    let r = engine.match_organizations(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

#[test]
fn organization_json_round_trips_for_storage() {
    // The service stores the Organization verbatim as a JSONB `data`
    // column; this is the round-trip the model relies on.
    let mut org = Organization::new("Acme, Inc.");
    org.legal_name = Some("Acme Incorporated".into());
    org.jurisdiction = Some("US".into());
    org.url = Some("https://acme.com".into());
    let value = serde_json::to_value(&org).expect("to json");
    let back: Organization = serde_json::from_value(value).expect("from json");
    assert_eq!(back.name, org.name);
    assert_eq!(back.legal_name, org.legal_name);
    assert_eq!(back.jurisdiction, org.jurisdiction);
    assert_eq!(back.url, org.url);
}
