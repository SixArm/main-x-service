use main_thing_service::models::address::PostalAddress;
use main_thing_service::models::geo::GeoCoordinates;
use main_thing_service::models::thing::Thing;
use main_thing_service::validation::{validate_thing, normalize_thing};

#[test]
fn test_validate_then_normalize_workflow() {
    let mut thing = Thing::new("  test thing  ");
    thing.address = Some(PostalAddress {
        street_address: Some("123 main st".into()),
        address_locality: Some("new york".into()),
        address_region: Some("ny".into()),
        address_country: Some("us".into()),
        postal_code: Some("10001".into()),
    });
    thing.geo = Some(GeoCoordinates::new(40.7128, -74.0060));

    let errors = validate_thing(&thing);
    assert!(errors.is_empty(), "Validation errors: {errors:?}");

    normalize_thing(&mut thing);
    assert_eq!(thing.name, "test thing");
    let addr = thing.address.as_ref().unwrap();
    assert_eq!(addr.address_locality.as_deref(), Some("New York"));
    assert_eq!(addr.address_region.as_deref(), Some("NY"));
    assert_eq!(addr.address_country.as_deref(), Some("US"));
}

#[test]
fn test_invalid_thing_does_not_normalize() {
    let mut thing = Thing::new("");
    thing.geo = Some(GeoCoordinates::new(999.0, 999.0));

    let errors = validate_thing(&thing);
    assert!(errors.len() >= 2, "Expected multiple errors: {errors:?}");

    normalize_thing(&mut thing);
}

#[test]
fn test_full_thing_lifecycle_validation() {
    let mut thing = Thing::new("Test Thing");
    thing.url = Some("https://example.com".into());
    thing.telephone = Some("+1-555-0100".into());
    thing.global_location_number = Some("1234567890123".into());
    thing.address = Some(PostalAddress {
        street_address: Some("100 broadway".into()),
        address_locality: Some("san francisco".into()),
        address_region: Some("ca".into()),
        address_country: Some("us".into()),
        postal_code: Some("94111".into()),
    });
    thing.geo = Some(GeoCoordinates::new(37.7749, -122.4194));

    assert!(validate_thing(&thing).is_empty());
    normalize_thing(&mut thing);
    assert!(validate_thing(&thing).is_empty());
    assert_eq!(
        thing.address.as_ref().unwrap().address_locality.as_deref(),
        Some("San Francisco")
    );
}
