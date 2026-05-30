use main_thing_service::matching::scoring::{compute_match, MatchWeights};
use main_thing_service::models::address::PostalAddress;
use main_thing_service::models::geo::GeoCoordinates;
use main_thing_service::models::identifier::ThingIdentifier;
use main_thing_service::models::thing::Thing;
use main_thing_service::models::thing_type::ThingType;
use main_thing_service::privacy::{gdpr_export, mask_thing};
use main_thing_service::validation::{normalize_thing, validate_thing};

// -- Validation edge cases --

#[test]
fn test_validate_boundary_coordinates() {
    // Exactly at boundaries should be valid
    let mut thing = Thing::new("North Pole");
    thing.geo = Some(GeoCoordinates::new(90.0, 180.0));
    assert!(validate_thing(&thing).is_empty());

    thing.geo = Some(GeoCoordinates::new(-90.0, -180.0));
    assert!(validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_just_past_boundary() {
    let mut thing = Thing::new("Invalid");
    thing.geo = Some(GeoCoordinates::new(90.001, 0.0));
    assert!(!validate_thing(&thing).is_empty());

    thing.geo = Some(GeoCoordinates::new(0.0, 180.001));
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_gln_exactly_13_digits() {
    let mut thing = Thing::new("Test");
    thing.global_location_number = Some("1234567890123".into());
    assert!(validate_thing(&thing).is_empty());

    thing.global_location_number = Some("123456789012".into()); // 12 digits
    assert!(!validate_thing(&thing).is_empty());

    thing.global_location_number = Some("12345678901234".into()); // 14 digits
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_url_protocols() {
    let mut thing = Thing::new("Test");

    thing.url = Some("http://example.com".into());
    assert!(validate_thing(&thing).is_empty());

    thing.url = Some("https://example.com".into());
    assert!(validate_thing(&thing).is_empty());

    thing.url = Some("ftp://example.com".into());
    assert!(!validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_address_minimal() {
    let mut thing = Thing::new("Test");

    // Only postal code - should be valid
    thing.address = Some(PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: None,
        postal_code: Some("10001".into()),
    });
    assert!(validate_thing(&thing).is_empty());

    // Only country - should be valid
    thing.address = Some(PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: Some("US".into()),
        postal_code: None,
    });
    assert!(validate_thing(&thing).is_empty());
}

#[test]
fn test_validate_empty_string_address_fields() {
    let mut thing = Thing::new("Test");
    thing.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some(String::new()),
        address_region: None,
        address_country: Some(String::new()),
        postal_code: Some(String::new()),
    });
    let errors = validate_thing(&thing);
    assert!(!errors.is_empty(), "Empty string fields should fail validation");
}

// -- Normalization edge cases --

#[test]
fn test_normalize_multi_word_locality() {
    let mut thing = Thing::new("Test");
    thing.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("san francisco bay area".into()),
        address_region: Some("ca".into()),
        address_country: Some("us".into()),
        postal_code: None,
    });
    normalize_thing(&mut thing);
    let addr = thing.address.as_ref().unwrap();
    assert_eq!(addr.address_locality.as_deref(), Some("San Francisco Bay Area"));
    assert_eq!(addr.address_region.as_deref(), Some("CA"));
}

#[test]
fn test_normalize_already_normalized() {
    let mut thing = Thing::new("Test Thing");
    thing.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: None,
    });
    normalize_thing(&mut thing);
    let addr = thing.address.as_ref().unwrap();
    assert_eq!(addr.address_locality.as_deref(), Some("New York"));
    assert_eq!(addr.address_region.as_deref(), Some("NY"));
}

#[test]
fn test_normalize_no_address() {
    let mut thing = Thing::new("  Trimmed  ");
    normalize_thing(&mut thing);
    assert_eq!(thing.name, "Trimmed");
    assert!(thing.address.is_none());
}

// -- Privacy edge cases --

#[test]
fn test_mask_thing_with_all_sensitive_fields() {
    let mut thing = Thing::new("Full Privacy Test");
    thing.telephone = Some("+1-555-867-5309".into());
    thing.fax_number = Some("+1-555-123-4567".into());
    thing.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

    let masked = mask_thing(&thing);

    // Phone masked
    assert!(masked.telephone.as_ref().unwrap().ends_with("****"));
    assert!(!masked.telephone.as_ref().unwrap().contains("5309"));

    // Fax masked
    assert!(masked.fax_number.as_ref().unwrap().ends_with("****"));
    assert!(!masked.fax_number.as_ref().unwrap().contains("4567"));

    // Geo rounded
    let geo = masked.geo.unwrap();
    assert!((geo.latitude - 40.78).abs() < 0.01);
    assert!((geo.longitude - (-73.97)).abs() < 0.01);

    // Non-sensitive unchanged
    assert_eq!(masked.name, "Full Privacy Test");
}

#[test]
fn test_mask_thing_empty_phone() {
    let mut thing = Thing::new("Test");
    thing.telephone = Some(String::new());
    let masked = mask_thing(&thing);
    assert_eq!(masked.telephone.as_deref(), Some("****"));
}

#[test]
fn test_gdpr_export_preserves_all_fields() {
    let mut thing = Thing::new("GDPR Test");
    thing.alternate_name = Some("Alt Name".into());
    thing.description = Some("Description".into());
    thing.thing_type = Some(ThingType::Park);
    thing.telephone = Some("+1-555-1234".into());
    thing.url = Some("https://example.com".into());
    thing.keywords = vec!["tag1".into(), "tag2".into()];

    let export = gdpr_export(&thing);
    assert_eq!(export["name"], "GDPR Test");
    assert_eq!(export["alternate_name"], "Alt Name");
    assert_eq!(export["description"], "Description");
    assert_eq!(export["telephone"], "+1-555-1234");
    assert_eq!(export["url"], "https://example.com");
    assert_eq!(export["keywords"].as_array().unwrap().len(), 2);
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
    // Simulate the full pipeline: validate -> normalize -> match
    let mut thing_a = Thing::new("  central park  ");
    thing_a.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("new york".into()),
        address_region: Some("ny".into()),
        address_country: Some("us".into()),
        postal_code: Some("10022".into()),
    });
    thing_a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    // Validate
    assert!(validate_thing(&thing_a).is_empty());

    // Normalize
    normalize_thing(&mut thing_a);
    assert_eq!(thing_a.name, "central park");

    // Match against a reference
    let mut thing_b = Thing::new("Central Park");
    thing_b.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });
    thing_b.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    let result = compute_match(&thing_a, &thing_b, &MatchWeights::default());
    assert!(result.score > 0.95, "Normalized things should match well: {}", result.score);
}

#[test]
fn test_validate_normalize_mask_export_workflow() {
    let mut thing = Thing::new("  Sensitive Thing  ");
    thing.telephone = Some("+1-555-867-5309".into());
    thing.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));
    thing.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("new york".into()),
        address_region: Some("ny".into()),
        address_country: Some("us".into()),
        postal_code: None,
    });

    // Step 1: Validate
    assert!(validate_thing(&thing).is_empty());

    // Step 2: Normalize
    normalize_thing(&mut thing);
    assert_eq!(thing.name, "Sensitive Thing");

    // Step 3: Mask for privacy
    let masked = mask_thing(&thing);
    assert!(masked.telephone.as_ref().unwrap().ends_with("****"));

    // Step 4: GDPR export of masked data
    let export = gdpr_export(&masked);
    assert_eq!(export["name"], "Sensitive Thing");
    assert!(export["telephone"].as_str().unwrap().ends_with("****"));
}

#[test]
fn test_gln_deterministic_trumps_everything() {
    // Even with completely different names, addresses, geo, and types,
    // a GLN match should give a perfect score
    let mut a = Thing::new("Alpha Store");
    a.thing_type = Some(ThingType::LocalBusiness);
    a.geo = Some(GeoCoordinates::new(40.7128, -74.0060));
    a.address = Some(PostalAddress {
        street_address: Some("100 Broadway".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10005".into()),
    });
    a.identifiers = vec![ThingIdentifier::gln("1234567890123")];

    let mut b = Thing::new("Beta Market");
    b.thing_type = Some(ThingType::Restaurant);
    b.geo = Some(GeoCoordinates::new(48.8584, 2.2945));
    b.address = Some(PostalAddress {
        street_address: Some("1 Rue de Rivoli".into()),
        address_locality: Some("Paris".into()),
        address_region: None,
        address_country: Some("FR".into()),
        postal_code: Some("75001".into()),
    });
    b.identifiers = vec![ThingIdentifier::gln("1234567890123")];

    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}
