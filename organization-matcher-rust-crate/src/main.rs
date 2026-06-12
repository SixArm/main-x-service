//! Demo binary — a runnable walkthrough of the `organization-matcher`
//! public API. Not part of the SemVer surface.

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use organization_matcher::{
    IdentifierScheme, MatchConfig, MatchingEngine, OrgIdentifier, Organization, PostalAddress,
};

fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
    OrgIdentifier {
        scheme,
        value: value.into(),
    }
}

fn main() {
    let engine = MatchingEngine::new(MatchConfig::default());

    println!("== organization-matcher demo ==\n");

    // 1. Legal-suffix-aware name match.
    let a = Organization::new("Acme, Inc.");
    let b = Organization::new("ACME Corporation");
    let r = engine.match_organizations(&a, &b);
    println!("name variants  : score={:.3}  {:?}", r.score, r.confidence);

    // 2. Deterministic LEI short-circuit despite different names.
    let mut a = Organization::new("Globex");
    let mut b = Organization::new("Globex International Holdings");
    a.identifiers
        .push(ident(IdentifierScheme::Lei, "5493001KJTIIGC8Y1R12"));
    b.identifiers
        .push(ident(IdentifierScheme::Lei, "5493001KJTIIGC8Y1R12"));
    let r = engine.match_organizations(&a, &b);
    println!(
        "LEI match      : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 3. Same-jurisdiction tax id.
    let mut a = Organization::new("Initech");
    let mut b = Organization::new("Initech LLC");
    a.jurisdiction = Some("US".into());
    b.jurisdiction = Some("US".into());
    a.identifiers
        .push(ident(IdentifierScheme::TaxId, "12-3456789"));
    b.identifiers
        .push(ident(IdentifierScheme::TaxId, "12-3456789"));
    let r = engine.match_organizations(&a, &b);
    println!(
        "tax id (US)    : score={:.3}  deterministic={}",
        r.score, r.breakdown.deterministic_match
    );

    // 4. Probabilistic: name + address + domain + founding year.
    let mut a = Organization::new("Wonka Industries");
    a.url = Some("https://www.wonka.com".into());
    a.address = Some(PostalAddress {
        locality: Some("Munich".into()),
        country: Some("DE".into()),
        ..Default::default()
    });
    a.founding_date = Some("1971".into());
    let mut b = Organization::new("Wonka Industries GmbH");
    b.url = Some("http://wonka.com/about".into());
    b.address = Some(PostalAddress {
        locality: Some("munich".into()),
        country: Some("DE".into()),
        ..Default::default()
    });
    b.founding_date = Some("1971-06-30".into());
    let r = engine.match_organizations(&a, &b);
    println!(
        "probabilistic  : score={:.3}  {:?}  is_match={}",
        r.score, r.confidence, r.is_match
    );
    println!("  breakdown    : {:?}", r.breakdown);

    // 5. Unrelated.
    let a = Organization::new("Acme Corporation");
    let b = Organization::new("Stark Industries");
    let r = engine.match_organizations(&a, &b);
    println!(
        "unrelated      : score={:.3}  is_match={}",
        r.score, r.is_match
    );
}
