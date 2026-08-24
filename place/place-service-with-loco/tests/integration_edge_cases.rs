#![warn(clippy::pedantic)]

//! Integration tests for boundary conditions and cross-module workflows.
//!
//! These pin validation boundaries (coordinate limits, GLN length, URL
//! schemes, minimal/empty addresses), normalization edge cases, privacy
//! masking of all sensitive fields, and full end-to-end pipelines that chain
//! validate → normalize → match / mask → export.

use bigdecimal::BigDecimal;
use place_service::matching::scoring::{MatchWeights, compute_match};
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::identifier::PlaceIdentifier;
use place_service::models::place::Place;
use place_service::models::place_type::PlaceType;
use place_service::privacy::{gdpr_export, mask_place};
use place_service::validation::{normalize_place, validate_place};

// -- Validation edge cases --

/// Coordinates exactly at the WGS-84 limits are valid.
#[test]
fn test_validate_boundary_coordinates() {
    // Exactly at boundaries should be valid
    let mut place = Place::new("North Pole");
    place.geo = Some(GeoCoordinates::new(90.0, 180.0));
    assert!(validate_place(&place).is_empty());

    place.geo = Some(GeoCoordinates::new(-90.0, -180.0));
    assert!(validate_place(&place).is_empty());
}

/// Coordinates just past the limits are rejected.
#[test]
fn test_validate_just_past_boundary() {
    let mut place = Place::new("Invalid");
    place.geo = Some(GeoCoordinates::new(90.001, 0.0));
    assert!(!validate_place(&place).is_empty());

    place.geo = Some(GeoCoordinates::new(0.0, 180.001));
    assert!(!validate_place(&place).is_empty());
}

/// A 13-digit GLN with a valid GS1 check digit passes; 12 or 14 digits, or a
/// wrong check digit, fail.
#[test]
fn test_validate_gln_exactly_13_digits() {
    let mut place = Place::new("Test");
    place.global_location_number = Some("0614141999996".into()); // valid check digit
    assert!(validate_place(&place).is_empty());

    place.global_location_number = Some("061414199999".into()); // 12 digits
    assert!(!validate_place(&place).is_empty());

    place.global_location_number = Some("06141419999966".into()); // 14 digits
    assert!(!validate_place(&place).is_empty());

    place.global_location_number = Some("0614141999990".into()); // wrong check digit
    assert!(!validate_place(&place).is_empty());
}

/// `http`/`https` URLs pass; other schemes (e.g. `ftp`) fail.
#[test]
fn test_validate_url_protocols() {
    let mut place = Place::new("Test");

    place.url = Some("http://example.com".into());
    assert!(validate_place(&place).is_empty());

    place.url = Some("https://example.com".into());
    assert!(validate_place(&place).is_empty());

    place.url = Some("ftp://example.com".into());
    assert!(!validate_place(&place).is_empty());
}

/// A single locating field (postal code or country alone) satisfies the rule.
#[test]
fn test_validate_address_minimal() {
    let mut place = Place::new("Test");

    // Only postal code - should be valid
    place.address = Some(PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: None,
        postal_code: Some("10001".into()),
    });
    assert!(validate_place(&place).is_empty());

    // Only country - should be valid
    place.address = Some(PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: Some("US".into()),
        postal_code: None,
    });
    assert!(validate_place(&place).is_empty());
}

/// Empty-string locating fields count as absent and fail validation.
#[test]
fn test_validate_empty_string_address_fields() {
    let mut place = Place::new("Test");
    place.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some(String::new()),
        address_region: None,
        address_country: Some(String::new()),
        postal_code: Some(String::new()),
    });
    let errors = validate_place(&place);
    assert!(
        !errors.is_empty(),
        "Empty string fields should fail validation"
    );
}

// -- Normalization edge cases --

/// Multi-word localities are title-cased word by word.
#[test]
fn test_normalize_multi_word_locality() {
    let mut place = Place::new("Test");
    place.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("san francisco bay area".into()),
        address_region: Some("ca".into()),
        address_country: Some("us".into()),
        postal_code: None,
    });
    normalize_place(&mut place);
    let addr = place.address.as_ref().unwrap();
    assert_eq!(
        addr.address_locality.as_deref(),
        Some("San Francisco Bay Area")
    );
    assert_eq!(addr.address_region.as_deref(), Some("CA"));
}

/// Normalization is idempotent on already-normalized input.
#[test]
fn test_normalize_already_normalized() {
    let mut place = Place::new("Test Place");
    place.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: None,
    });
    normalize_place(&mut place);
    let addr = place.address.as_ref().unwrap();
    assert_eq!(addr.address_locality.as_deref(), Some("New York"));
    assert_eq!(addr.address_region.as_deref(), Some("NY"));
}

/// Normalizing a place without an address still trims the name.
#[test]
fn test_normalize_no_address() {
    let mut place = Place::new("  Trimmed  ");
    normalize_place(&mut place);
    assert_eq!(place.name, "Trimmed");
    assert!(place.address.is_none());
}

// -- Privacy edge cases --

/// Masking redacts phone, fax, and geo while keeping non-sensitive fields.
#[test]
fn test_mask_place_with_all_sensitive_fields() {
    let mut place = Place::new("Full Privacy Test");
    place.telephone = Some("+1-555-867-5309".into());
    place.fax_number = Some("+1-555-123-4567".into());
    place.geo = Some(GeoCoordinates::new(40.782_934_56, -73.965_432_10));

    let masked = mask_place(&place);

    // Phone masked
    assert!(masked.telephone.as_ref().unwrap().ends_with("****"));
    assert!(!masked.telephone.as_ref().unwrap().contains("5309"));

    // Fax masked
    assert!(masked.fax_number.as_ref().unwrap().ends_with("****"));
    assert!(!masked.fax_number.as_ref().unwrap().contains("4567"));

    // Geo rounded
    let geo = masked.geo.unwrap();
    assert_eq!(geo.latitude, "40.78".parse::<BigDecimal>().unwrap());
    assert_eq!(geo.longitude, "-73.97".parse::<BigDecimal>().unwrap());

    // Non-sensitive unchanged
    assert_eq!(masked.name, "Full Privacy Test");
}

/// An empty phone string masks to `****`.
#[test]
fn test_mask_place_empty_phone() {
    let mut place = Place::new("Test");
    place.telephone = Some(String::new());
    let masked = mask_place(&place);
    assert_eq!(masked.telephone.as_deref(), Some("****"));
}

/// GDPR export preserves every populated field, including arrays.
#[test]
fn test_gdpr_export_preserves_all_fields() {
    let mut place = Place::new("GDPR Test");
    place.alternate_name = Some("Alt Name".into());
    place.description = Some("Description".into());
    place.place_type = Some(PlaceType::Park);
    place.telephone = Some("+1-555-1234".into());
    place.url = Some("https://example.com".into());
    place.keywords = vec!["tag1".into(), "tag2".into()];

    let export = gdpr_export(&place);
    assert_eq!(export["name"], "GDPR Test");
    assert_eq!(export["alternate_name"], "Alt Name");
    assert_eq!(export["description"], "Description");
    assert_eq!(export["telephone"], "+1-555-1234");
    assert_eq!(export["url"], "https://example.com");
    assert_eq!(export["keywords"].as_array().unwrap().len(), 2);
}

/// A soft-deleted record exports its deletion flag and timestamp.
#[test]
fn test_gdpr_export_soft_deleted_place() {
    let mut place = Place::new("Deleted");
    place.soft_delete();

    let export = gdpr_export(&place);
    assert_eq!(export["is_deleted"], true);
    assert!(export["deleted_at"].is_string());
}

// -- Combined workflow tests --

/// Full validate → normalize → match pipeline yields a strong match.
#[test]
fn test_validate_normalize_match_workflow() {
    // Simulate the full pipeline: validate -> normalize -> match
    let mut place_a = Place::new("  central park  ");
    place_a.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("new york".into()),
        address_region: Some("ny".into()),
        address_country: Some("us".into()),
        postal_code: Some("10022".into()),
    });
    place_a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    // Validate
    assert!(validate_place(&place_a).is_empty());

    // Normalize
    normalize_place(&mut place_a);
    assert_eq!(place_a.name, "central park");

    // Match against a reference
    let mut place_b = Place::new("Central Park");
    place_b.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });
    place_b.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    let result = compute_match(&place_a, &place_b, &MatchWeights::default());
    assert!(
        result.score > 0.95,
        "Normalized places should match well: {}",
        result.score
    );
}

/// Full validate → normalize → mask → export pipeline preserves redaction.
#[test]
fn test_validate_normalize_mask_export_workflow() {
    let mut place = Place::new("  Sensitive Place  ");
    place.telephone = Some("+1-555-867-5309".into());
    place.geo = Some(GeoCoordinates::new(40.782_934_56, -73.965_432_10));
    place.address = Some(PostalAddress {
        street_address: None,
        address_locality: Some("new york".into()),
        address_region: Some("ny".into()),
        address_country: Some("us".into()),
        postal_code: None,
    });

    // Step 1: Validate
    assert!(validate_place(&place).is_empty());

    // Step 2: Normalize
    normalize_place(&mut place);
    assert_eq!(place.name, "Sensitive Place");

    // Step 3: Mask for privacy
    let masked = mask_place(&place);
    assert!(masked.telephone.as_ref().unwrap().ends_with("****"));

    // Step 4: GDPR export of masked data
    let export = gdpr_export(&masked);
    assert_eq!(export["name"], "Sensitive Place");
    assert!(export["telephone"].as_str().unwrap().ends_with("****"));
}

/// A shared GLN pins the score to 1.0 despite every other field differing.
#[test]
fn test_gln_deterministic_trumps_everything() {
    // Even with completely different names, addresses, geo, and types,
    // a GLN match should give a perfect score
    let mut a = Place::new("Alpha Store");
    a.place_type = Some(PlaceType::LocalBusiness);
    a.geo = Some(GeoCoordinates::new(40.7128, -74.0060));
    a.address = Some(PostalAddress {
        street_address: Some("100 Broadway".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10005".into()),
    });
    a.identifiers = vec![PlaceIdentifier::gln("1234567890123")];

    let mut b = Place::new("Beta Market");
    b.place_type = Some(PlaceType::Restaurant);
    b.geo = Some(GeoCoordinates::new(48.8584, 2.2945));
    b.address = Some(PostalAddress {
        street_address: Some("1 Rue de Rivoli".into()),
        address_locality: Some("Paris".into()),
        address_region: None,
        address_country: Some("FR".into()),
        postal_code: Some("75001".into()),
    });
    b.identifiers = vec![PlaceIdentifier::gln("1234567890123")];

    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}
