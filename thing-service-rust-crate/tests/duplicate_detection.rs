//! End-to-end duplicate-detection integration tests for the
//! thing-service ↔ thing-matcher bridge.
//!
//! Exercises the schema.org/Thing identity model: typed identifier routing
//! (DOI/ISBN/ISSN/GTIN/UUID → opaque `property_id` strings), `same_as`
//! cross-references, and the matcher's deterministic short-circuit on a
//! shared globally-unique identifier.

use thing_service::matching::adapter::to_matcher_thing;
use thing_service::matching::matcher_lib::{Confidence, MatchingEngine};
use thing_service::models::{
    identifier::{IdentifierType, ThingIdentifier},
    thing::Thing,
};

fn engine() -> MatchingEngine {
    MatchingEngine::default_config()
}

fn pride_and_prejudice() -> Thing {
    let mut t = Thing::new("Pride and Prejudice");
    t.alternate_names = vec!["First Impressions".into()];
    t.description = Some("A novel of manners by Jane Austen.".into());
    t.additional_type = Some("https://schema.org/Book".into());
    t.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    t.same_as = vec![
        "https://www.wikidata.org/wiki/Q170583".into(),
        "https://openlibrary.org/works/OL1394865W".into(),
    ];
    t.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    t
}

// =============================================================================
// Identical / near-duplicate cases
// =============================================================================

#[test]
fn identical_clones_score_near_one_high_confidence() {
    let a = pride_and_prejudice();
    let b = a.clone();

    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    assert!(
        result.score >= 0.95,
        "identical clones should score ≥ 0.95, got {}",
        result.score
    );
    assert_eq!(result.confidence, Confidence::High);
    assert!(result.is_match);
}

#[test]
fn typo_in_name_still_matches_with_supporting_identifier() {
    let a = pride_and_prejudice();
    let mut b = pride_and_prejudice();
    b.name = "Prde and Prejudice".into(); // missing 'i'

    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    // Identifier match (ISBN) is the deterministic anchor; name typo
    // doesn't disqualify.
    assert!(
        result.score >= 0.95,
        "typo with shared ISBN should still score ≥ 0.95, got {}",
        result.score
    );
}

// =============================================================================
// Deterministic short-circuit — globally-unique identifiers
// =============================================================================

#[test]
fn shared_isbn_short_circuits_to_one() {
    // ISBN is one of the matcher's deterministic identifier schemes
    // (`is_deterministic` returns true for Doi / Isbn / Issn / Gtin / Mpn /
    // SerialNumber / Uuid). A shared ISBN must hit `deterministic_match`.
    let mut a = Thing::new("Pride and Prejudice");
    a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    let mut b = Thing::new("Stolz und Vorurteil"); // German translation
    b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

    let ma = to_matcher_thing(&a);
    let mb = to_matcher_thing(&b);

    assert!(
        engine().deterministic_match(&ma, &mb),
        "shared ISBN must trigger deterministic_match"
    );
    let result = engine().match_things(&ma, &mb);
    assert_eq!(
        result.breakdown.identifiers_score,
        Some(1.0),
        "shared ISBN pair → identifiers_score = 1.0"
    );
}

#[test]
fn shared_doi_short_circuits_to_one() {
    let mut a = Thing::new("Some Paper");
    a.identifiers = vec![ThingIdentifier::doi("10.1038/nature12373")];
    let mut b = Thing::new("Same Paper, Different Filename");
    b.identifiers = vec![ThingIdentifier::doi("10.1038/nature12373")];

    let ma = to_matcher_thing(&a);
    let mb = to_matcher_thing(&b);
    assert!(engine().deterministic_match(&ma, &mb));
}

#[test]
fn shared_uuid_short_circuits_to_one() {
    let mut a = Thing::new("Resource");
    a.identifiers = vec![ThingIdentifier::uuid(
        "550e8400-e29b-41d4-a716-446655440000",
    )];
    let mut b = Thing::new("Resource (rename)");
    b.identifiers = vec![ThingIdentifier::uuid(
        "550e8400-e29b-41d4-a716-446655440000",
    )];

    assert!(engine().deterministic_match(&to_matcher_thing(&a), &to_matcher_thing(&b)));
}

#[test]
fn different_isbns_do_not_short_circuit_even_with_same_name() {
    let mut a = Thing::new("Pride and Prejudice");
    a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    let mut b = Thing::new("Pride and Prejudice"); // different edition
    b.identifiers = vec![ThingIdentifier::isbn("9781503290563")];

    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    assert_eq!(
        result.breakdown.identifiers_score,
        Some(0.0),
        "different ISBNs (both populated) should score 0 on identifiers axis"
    );
}

// =============================================================================
// Non-deterministic identifiers (SKU / URI / Custom)
// =============================================================================

#[test]
fn non_deterministic_identifier_filter_is_service_side_concern() {
    // SKUs are vendor-scoped and not globally unique. The *matcher* treats
    // any shared `(property_id, value)` pair as a deterministic anchor —
    // that's an intentionally permissive contract. The "globally unique"
    // distinction (DOI/ISBN/UUID/etc. vs SKU/URI/Custom) lives on the
    // *service* side via `ThingIdentifier::is_deterministic()`. Callers
    // doing real dedup should filter the identifier list through
    // `is_deterministic` before invoking `deterministic_match`.
    let sku = ThingIdentifier::sku("WIDGET-42");
    let isbn = ThingIdentifier::isbn("9780141439518");

    assert!(!sku.is_deterministic(), "service-side: SKU is not deterministic");
    assert!(isbn.is_deterministic(), "service-side: ISBN is deterministic");

    // And confirm the matcher's permissive contract — useful for tests
    // wanting to assert *any* shared identifier counts:
    let mut a = Thing::new("Widget");
    a.identifiers = vec![sku.clone()];
    let mut b = Thing::new("Different Item");
    b.identifiers = vec![sku];
    assert!(
        engine().deterministic_match(&to_matcher_thing(&a), &to_matcher_thing(&b)),
        "matcher contract: any shared (property_id, value) → deterministic_match=true; \
         the service must pre-filter when stricter semantics are needed"
    );
}

#[test]
fn custom_identifier_property_id_passes_through_verbatim() {
    let mut a = Thing::new("Item");
    a.identifiers = vec![ThingIdentifier::new(
        IdentifierType::Custom("OpenLibrary".into()),
        "OL1394865W",
    )];
    let m = to_matcher_thing(&a);
    assert_eq!(
        m.identifiers[0].property_id, "OpenLibrary",
        "Custom(s) must pass the carried label through verbatim"
    );
}

// =============================================================================
// Same-as / URL cross-references
// =============================================================================

#[test]
fn shared_same_as_url_drives_evidence() {
    let mut a = Thing::new("Linux kernel");
    a.same_as = vec!["https://www.wikidata.org/wiki/Q14579".into()];
    let mut b = Thing::new("Linux Kernel"); // case-insensitive
    b.same_as = vec!["https://www.wikidata.org/wiki/Q14579".into()];

    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    assert!(
        result.breakdown.same_as_score.unwrap_or(0.0) > 0.0,
        "shared same_as URL must contribute > 0 to same_as_score"
    );
    assert!(result.is_match);
}

#[test]
fn shared_canonical_url_contributes_positive_signal() {
    let mut a = Thing::new("The Rust Programming Language");
    a.url = Some("https://doc.rust-lang.org/book/".into());
    let mut b = Thing::new("The Rust Programming Language");
    b.url = Some("https://doc.rust-lang.org/book/".into());

    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    assert_eq!(result.breakdown.url_score, Some(1.0));
}

// =============================================================================
// Negative cases
// =============================================================================

#[test]
fn unrelated_things_score_low_and_do_not_match() {
    let a = pride_and_prejudice();
    let mut b = Thing::new("Linux Kernel");
    b.identifiers = vec![ThingIdentifier::uuid(
        "11111111-2222-3333-4444-555555555555",
    )];

    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    assert!(
        result.score < 0.50,
        "unrelated things should score < 0.50, got {}",
        result.score
    );
    assert!(!result.is_match);
}

// =============================================================================
// Field-routing pinning
// =============================================================================

#[test]
fn additional_type_singular_routes_to_additional_types_vec() {
    let mut a = Thing::new("Pride and Prejudice");
    a.additional_type = Some("https://schema.org/Book".into());
    let m = to_matcher_thing(&a);
    assert_eq!(m.additional_types.len(), 1);
    assert_eq!(m.additional_types[0], "https://schema.org/Book");
}

#[test]
fn first_image_url_becomes_matcher_image() {
    let mut a = Thing::new("Item");
    a.images = vec![
        "https://example.com/a.jpg".into(),
        "https://example.com/b.jpg".into(),
    ];
    let m = to_matcher_thing(&a);
    assert_eq!(m.image.as_deref(), Some("https://example.com/a.jpg"));
}

#[test]
fn isbn_property_id_lowercases_to_canonical_token() {
    let mut a = Thing::new("Book");
    a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    let m = to_matcher_thing(&a);
    assert_eq!(m.identifiers[0].property_id, "isbn");
    assert_eq!(m.identifiers[0].value, "9780141439518");
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn sparse_records_do_not_panic() {
    let a = Thing::new("");
    let b = Thing::new("X");
    let result = engine().match_things(&to_matcher_thing(&a), &to_matcher_thing(&b));
    assert!(result.score >= 0.0 && result.score <= 1.0);
}
