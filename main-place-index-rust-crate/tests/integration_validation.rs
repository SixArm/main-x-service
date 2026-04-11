use main_place_index::models::address::PostalAddress;
use main_place_index::models::geo::GeoCoordinates;
use main_place_index::models::place::Place;
use main_place_index::validation::{validate_place, normalize_place};

#[test]
fn test_validate_then_normalize_workflow() {
    let mut place = Place::new("  test place  ");
    place.address = Some(PostalAddress {
        street_address: Some("123 main st".into()),
        address_locality: Some("new york".into()),
        address_region: Some("ny".into()),
        address_country: Some("us".into()),
        postal_code: Some("10001".into()),
    });
    place.geo = Some(GeoCoordinates::new(40.7128, -74.0060));

    let errors = validate_place(&place);
    assert!(errors.is_empty(), "Validation errors: {errors:?}");

    normalize_place(&mut place);
    assert_eq!(place.name, "test place");
    let addr = place.address.as_ref().unwrap();
    assert_eq!(addr.address_locality.as_deref(), Some("New York"));
    assert_eq!(addr.address_region.as_deref(), Some("NY"));
    assert_eq!(addr.address_country.as_deref(), Some("US"));
}

#[test]
fn test_invalid_place_does_not_normalize() {
    let mut place = Place::new("");
    place.geo = Some(GeoCoordinates::new(999.0, 999.0));

    let errors = validate_place(&place);
    assert!(errors.len() >= 2, "Expected multiple errors: {errors:?}");

    normalize_place(&mut place);
}

#[test]
fn test_full_place_lifecycle_validation() {
    let mut place = Place::new("Test Place");
    place.url = Some("https://example.com".into());
    place.telephone = Some("+1-555-0100".into());
    place.global_location_number = Some("1234567890123".into());
    place.address = Some(PostalAddress {
        street_address: Some("100 broadway".into()),
        address_locality: Some("san francisco".into()),
        address_region: Some("ca".into()),
        address_country: Some("us".into()),
        postal_code: Some("94111".into()),
    });
    place.geo = Some(GeoCoordinates::new(37.7749, -122.4194));

    assert!(validate_place(&place).is_empty());
    normalize_place(&mut place);
    assert!(validate_place(&place).is_empty());
    assert_eq!(
        place.address.as_ref().unwrap().address_locality.as_deref(),
        Some("San Francisco")
    );
}
