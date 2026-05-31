use thing_service::matching::scoring::{compute_match, MatchWeights};
use thing_service::models::identifier::{IdentifierType, ThingIdentifier};
use thing_service::models::thing::Thing;
use thing_service::privacy::{gdpr_export, mask_thing};
use thing_service::validation::{normalize_thing, validate_thing};

// -- Validation edge cases --

#[test]
fn test_validate_url_protocols() {
    let mut thing = Thing::new("X");

    thing.url = Some("http://example.com".into());
    assert!(validate_thing(&thing).is_empty());

    thing.url = Some("https://example.com".into());
    assert!(validate_thing(&thing).is_empty());

    thing.url = Some("ftp://example.com".into());
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_isbn_length() {
    let mut thing = Thing::new("X");

    thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")]; // 13
    assert!(validate_thing(&thing).is_empty());

    thing.identifiers = vec![ThingIdentifier::isbn("0141439513")]; // 10
    assert!(validate_thing(&thing).is_empty());

    thing.identifiers = vec![ThingIdentifier::isbn("123")];
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_gtin_length() {
    let mut thing = Thing::new("X");

    thing.identifiers = vec![ThingIdentifier::gtin("12345678")]; // 8
    assert!(validate_thing(&thing).is_empty());

    thing.identifiers = vec![ThingIdentifier::gtin("0012345600012")]; // 13
    assert!(validate_thing(&thing).is_empty());

    thing.identifiers = vec![ThingIdentifier::gtin("12345")];
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_doi_format() {
    let mut thing = Thing::new("X");

    thing.identifiers = vec![ThingIdentifier::doi("10.1038/nature12373")];
    assert!(validate_thing(&thing).is_empty());

    thing.identifiers = vec![ThingIdentifier::doi("nature12373")];
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_uuid_format() {
    let mut thing = Thing::new("X");

    thing.identifiers = vec![ThingIdentifier::uuid("550e8400-e29b-41d4-a716-446655440000")];
    assert!(validate_thing(&thing).is_empty());

    thing.identifiers = vec![ThingIdentifier::uuid("not-a-uuid")];
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_custom_identifier_type_skips_format_check() {
    let mut thing = Thing::new("X");
    thing.identifiers = vec![ThingIdentifier::new(
        IdentifierType::Custom("Internal".into()),
        "anything-goes-42",
    )];
    assert!(validate_thing(&thing).is_empty());
}

// -- Normalization edge cases --

#[test]
fn test_normalize_url_scheme_lowercased() {
    let mut thing = Thing::new("X");
    thing.url = Some("HTTPS://Example.com/Path".into());
    normalize_thing(&mut thing);
    // scheme lowered, rest preserved
    assert_eq!(thing.url.as_deref(), Some("https://Example.com/Path"));
}

#[test]
fn test_normalize_dedupes_same_as() {
    let mut thing = Thing::new("X");
    thing.same_as = vec![
        "https://example.com".into(),
        "https://example.com".into(),
        "https://other.com".into(),
    ];
    normalize_thing(&mut thing);
    assert_eq!(thing.same_as.len(), 2);
}

#[test]
fn test_normalize_dedupes_alternate_names() {
    let mut thing = Thing::new("X");
    thing.alternate_names = vec!["A".into(), "A".into(), "B".into()];
    normalize_thing(&mut thing);
    assert_eq!(thing.alternate_names, vec!["A".to_string(), "B".to_string()]);
}

// -- Privacy edge cases --

#[test]
fn test_mask_preserves_property_id() {
    let mut thing = Thing::new("X");
    thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    let masked = mask_thing(&thing);
    assert_eq!(
        masked.identifiers[0].property_id,
        IdentifierType::Isbn
    );
}

#[test]
fn test_mask_short_identifier_value() {
    let mut thing = Thing::new("X");
    thing.identifiers = vec![ThingIdentifier::sku("AB")];
    let masked = mask_thing(&thing);
    assert_eq!(masked.identifiers[0].value, "****");
}

#[test]
fn test_gdpr_export_preserves_schema_org_fields() {
    let mut thing = Thing::new("GDPR Test");
    thing.alternate_names = vec!["Alias".into()];
    thing.description = Some("Description".into());
    thing.additional_type = Some("https://schema.org/Book".into());
    thing.url = Some("https://example.com".into());
    thing.same_as = vec!["https://other.example.com".into()];

    let export = gdpr_export(&thing);
    assert_eq!(export["name"], "GDPR Test");
    assert_eq!(export["alternate_names"].as_array().unwrap().len(), 1);
    assert_eq!(export["description"], "Description");
    assert_eq!(export["additional_type"], "https://schema.org/Book");
    assert_eq!(export["url"], "https://example.com");
    assert_eq!(export["same_as"].as_array().unwrap().len(), 1);
}

#[test]
fn test_gdpr_export_soft_deleted_thing() {
    let mut thing = Thing::new("Deleted");
    thing.soft_delete();
    let export = gdpr_export(&thing);
    assert_eq!(export["is_deleted"], true);
    assert!(export["deleted_at"].is_string());
}

// -- Combined workflow tests --

#[test]
fn test_validate_normalize_match_workflow() {
    let mut a = Thing::new("  pride and prejudice  ");
    a.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());

    assert!(validate_thing(&a).is_empty());

    normalize_thing(&mut a);
    assert_eq!(a.name, "pride and prejudice");

    let mut b = Thing::new("Pride and Prejudice");
    b.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());

    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.9, "normalized should match well: {}", result.score);
}

#[test]
fn test_validate_normalize_mask_export_workflow() {
    let mut thing = Thing::new("  Sensitive Thing  ");
    thing.owner = Some("Jane Doe".into());
    thing.identifiers = vec![ThingIdentifier::serial_number("SN-1234567890")];

    assert!(validate_thing(&thing).is_empty());

    normalize_thing(&mut thing);
    assert_eq!(thing.name, "Sensitive Thing");

    let masked = mask_thing(&thing);
    assert_eq!(masked.owner.as_deref(), Some("[owner withheld]"));

    let export = gdpr_export(&masked);
    assert_eq!(export["name"], "Sensitive Thing");
    assert_eq!(export["owner"], "[owner withheld]");
}

#[test]
fn test_isbn_deterministic_trumps_everything() {
    // Different names, URLs, descriptions — but shared ISBN forces 1.0.
    let mut a = Thing::new("Pride and Prejudice");
    a.url = Some("https://example.com/a".into());
    a.description = Some("English-language paperback".into());
    a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

    let mut b = Thing::new("Stolz und Vorurteil");
    b.url = Some("https://verlag.example.de/buch".into());
    b.description = Some("Deutschsprachige Taschenbuchausgabe".into());
    b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}
