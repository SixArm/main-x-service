//! Integration tests driving the public re-exported surface only —
//! everything reachable via `use organization_matcher::…`. A rename of
//! any re-export breaks it.

use organization_matcher::{
    Confidence, IdentifierScheme, MatchConfig, MatchingEngine, OrgIdentifier, Organization,
    PostalAddress, RelationKind, RelationshipRef,
};

/// Test helper: build an `OrgIdentifier` from a scheme + string value.
fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
    OrgIdentifier {
        scheme,
        value: value.into(),
    }
}

// ─── Deterministic short-circuits ────────────────────────────────────

/// Pins that EVERY scheme flagged deterministic short-circuits to a
/// score-1.0 High match when its value is shared, regardless of name.
#[test]
fn r0_fires_for_every_deterministic_scheme() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Lei,
        IdentifierScheme::Duns,
        IdentifierScheme::Iso6523,
        IdentifierScheme::Gln,
        IdentifierScheme::Wikidata,
        IdentifierScheme::Ror,
        IdentifierScheme::Isni,
        IdentifierScheme::Vat,
    ] {
        let mut a = Organization::new("Left");
        let mut b = Organization::new("Right");
        a.identifiers
            .push(ident(scheme.clone(), "shared-value-123"));
        b.identifiers
            .push(ident(scheme.clone(), "shared-value-123"));
        let r = engine.match_organizations(&a, &b);
        assert!(
            r.breakdown.deterministic_match,
            "{scheme:?} should short-circuit"
        );
        assert_eq!(r.score, 1.0);
        assert_eq!(r.confidence, Confidence::High);
    }
}

/// Pins the negative side: classification codes, `Custom`, and an
/// unscoped `TaxId` must NOT short-circuit even when the value matches.
#[test]
fn classification_and_custom_schemes_do_not_short_circuit() {
    let engine = MatchingEngine::default_config();
    for scheme in [
        IdentifierScheme::Naics,
        IdentifierScheme::IsicV4,
        IdentifierScheme::Sic,
        IdentifierScheme::TaxId, // scoped, needs jurisdiction
        IdentifierScheme::Custom("Crunchbase".into()),
    ] {
        let mut a = Organization::new("Alpha Holdings");
        let mut b = Organization::new("Beta Ventures");
        a.identifiers.push(ident(scheme.clone(), "shared-123"));
        b.identifiers.push(ident(scheme.clone(), "shared-123"));
        let r = engine.match_organizations(&a, &b);
        assert!(
            !r.breakdown.deterministic_match,
            "{scheme:?} must not short-circuit"
        );
    }
}

/// Pins R-1 on the public surface: same jurisdiction + same tax id → 1.0.
#[test]
fn tax_id_short_circuits_within_jurisdiction() {
    let engine = MatchingEngine::default_config();
    let mut a = Organization::new("Initech");
    let mut b = Organization::new("Initech Limited");
    a.jurisdiction = Some("GB".into());
    b.jurisdiction = Some("GB".into());
    a.identifiers.push(ident(IdentifierScheme::TaxId, "GB123"));
    b.identifiers.push(ident(IdentifierScheme::TaxId, "GB123"));
    let r = engine.match_organizations(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

/// Pins R-2 on the public surface: a shared `same_as` (ROR) URL → 1.0.
#[test]
fn same_as_url_overlap_short_circuits() {
    let engine = MatchingEngine::default_config();
    let mut a = Organization::new("Anything");
    let mut b = Organization::new("Anything Else");
    a.same_as = vec!["https://ror.org/02nr0ka47".into()];
    b.same_as = vec!["https://ror.org/02nr0ka47".into()];
    let r = engine.match_organizations(&a, &b);
    assert_eq!(r.score, 1.0);
    assert!(r.breakdown.deterministic_match);
}

// ─── Probabilistic path ──────────────────────────────────────────────

/// Pins the end-to-end legal-suffix case: differing suffixes still
/// reach the High band and `is_match`.
#[test]
fn legal_suffix_variants_are_high_confidence() {
    let engine = MatchingEngine::default_config();
    let a = Organization::new("Acme, Inc.");
    let b = Organization::new("ACME Corporation");
    let r = engine.match_organizations(&a, &b);
    assert!(r.score >= 0.95, "got {}", r.score);
    assert_eq!(r.confidence, Confidence::High);
    assert!(r.is_match);
}

/// Pins renormalisation end-to-end: with only name/url/jurisdiction
/// present and all agreeing, the score is ~1.0 and the breakdown shows
/// the absent components as `None`.
#[test]
fn renormalisation_over_present_components() {
    // name + url + jurisdiction present; all agree → ~1.0, not diluted
    // by the absent address/founding/keywords weights.
    let engine = MatchingEngine::default_config();
    let mut a = Organization::new("Wonka Industries");
    let mut b = Organization::new("Wonka Industries");
    a.url = Some("https://www.wonka.com".into());
    b.url = Some("http://wonka.com/about".into());
    a.jurisdiction = Some("US".into());
    b.jurisdiction = Some("US".into());
    let r = engine.match_organizations(&a, &b);
    assert!(r.score >= 0.99, "got {}", r.score);
    assert!(r.breakdown.name_score.is_some());
    assert_eq!(r.breakdown.url_score, Some(1.0));
    assert_eq!(r.breakdown.jurisdiction_score, Some(1.0));
    assert!(r.breakdown.address_score.is_none());
    assert!(r.breakdown.keywords_score.is_none());
}

/// Pins that unrelated orgs are Low-band non-matches with no rule fired.
#[test]
fn unrelated_orgs_are_low_confidence_non_matches() {
    let engine = MatchingEngine::default_config();
    let a = Organization::new("Acme Corporation");
    let b = Organization::new("Stark Industries");
    let r = engine.match_organizations(&a, &b);
    assert!(!r.is_match);
    assert_eq!(r.confidence, Confidence::Low);
    assert!(!r.breakdown.deterministic_match);
}

/// Pins that corroborating address + domain evidence pushes a near-name
/// match over the threshold into `is_match`.
#[test]
fn address_and_domain_corroborate_a_name_match() {
    let engine = MatchingEngine::default_config();
    let mut a = Organization::new("Wonka Industries");
    let mut b = Organization::new("Wonka Industries GmbH");
    let addr = |city: &str| PostalAddress {
        locality: Some(city.into()),
        country: Some("DE".into()),
        ..Default::default()
    };
    a.address = Some(addr("Munich"));
    b.address = Some(addr("munich"));
    a.url = Some("https://wonka.com".into());
    b.url = Some("https://www.wonka.com".into());
    let r = engine.match_organizations(&a, &b);
    assert!(r.is_match, "got {}", r.score);
}

// ─── Threshold presets + one-to-many ─────────────────────────────────

/// Pins that the presets only move `is_match` (via the threshold), never
/// the raw `score`, and that strict ⊆ default ⊆ lenient for matching.
#[test]
fn strict_and_lenient_change_is_match_not_score() {
    let a = Organization::new("Acme Corporation");
    let b = Organization::new("Acme Corporated Holdings");
    let default = MatchingEngine::new(MatchConfig::default()).match_organizations(&a, &b);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_organizations(&a, &b);
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_organizations(&a, &b);
    assert!((default.score - strict.score).abs() < 1e-9);
    assert!((default.score - lenient.score).abs() < 1e-9);
    if strict.is_match {
        assert!(default.is_match && lenient.is_match);
    }
}

/// Pins the one-to-many surface: `match_one_to_many` preserves order,
/// `rank` sorts best-first, and both handle empty candidate slices.
#[test]
fn one_to_many_surface() {
    let engine = MatchingEngine::default_config();
    let query = Organization::new("Acme Corporation");
    let cands = vec![
        Organization::new("Stark Industries"),
        Organization::new("Acme Corporation"),
    ];
    let out = engine.match_one_to_many(&query, &cands);
    assert_eq!(out.len(), 2);
    assert!(out[1].score > out[0].score);

    let ranked = engine.rank(&query, &cands);
    assert_eq!(ranked[0].0, 1);

    assert!(engine.match_one_to_many(&query, &[]).is_empty());
    assert!(engine.find_matches(&query, &[]).is_empty());
}

/// Pins that `MatchResult` (and its nested breakdown) serialises to
/// JSON via serde — the wire contract for service callers.
#[test]
fn match_result_serialises_to_json() {
    let engine = MatchingEngine::default_config();
    let a = Organization::new("Acme Corporation");
    let b = Organization::new("Acme Corp");
    let r = engine.match_organizations(&a, &b);
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("score"));
    assert!(json.contains("breakdown"));
}

// ─── Relationships + tags (§14a / §14b) ───────────────────────────────

/// Pins the relationships + tags surface end-to-end: both components
/// are `None` when absent (renormalised out, not diluting the score),
/// and populating identical relationship/tag sets on both sides yields
/// perfect `Some(1.0)` per-component scores in the breakdown.
#[test]
fn relationships_and_tags_score_end_to_end() {
    let engine = MatchingEngine::default_config();

    // Absent on both sides: `None`, not a penalising `0.0`.
    let a = Organization::new("Acme Corporation");
    let b = Organization::new("Acme Corporation");
    let r = engine.match_organizations(&a, &b);
    assert_eq!(r.breakdown.relationships_score, None);
    assert_eq!(r.breakdown.tags_score, None);
    assert!(r.score >= 0.99, "got {}", r.score);

    // Populated identically on both sides: perfect per-component scores.
    let mut a = Organization::new("Acme Corporation");
    let mut b = Organization::new("Acme Corporation");
    a.relationships =
        vec![RelationshipRef::new(RelationKind::SubOrganizationOf, "org-parent").unwrap()];
    b.relationships = a.relationships.clone();
    a.tags = vec!["vendor".to_string()];
    b.tags = vec!["Vendor".to_string()];
    let r = engine.match_organizations(&a, &b);
    assert_eq!(r.breakdown.relationships_score, Some(1.0));
    assert_eq!(r.breakdown.tags_score, Some(1.0));
}
