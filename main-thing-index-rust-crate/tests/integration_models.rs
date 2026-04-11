use main_thing_index::models::address::PostalAddress;
use main_thing_index::models::amenity::AmenityFeature;
use main_thing_index::models::consent::{Consent, ConsentStatus, ConsentType};
use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::identifier::{IdentifierType, ThingIdentifier};
use main_thing_index::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};
use main_thing_index::models::thing::Thing;
use main_thing_index::models::thing_type::ThingType;

use chrono::{Duration, Utc};
use uuid::Uuid;

// -- Thing lifecycle tests --

#[test]
fn test_thing_full_construction_and_serialization() {
    let mut thing = Thing::new("Eiffel Tower");
    thing.alternate_name = Some("La Tour Eiffel".into());
    thing.description = Some("Iconic iron lattice tower in Paris".into());
    thing.thing_type = Some(ThingType::CivicStructure);
    thing.address = Some(PostalAddress {
        street_address: Some("Champ de Mars, 5 Avenue Anatole France".into()),
        address_locality: Some("Paris".into()),
        address_region: Some("Île-de-France".into()),
        address_country: Some("FR".into()),
        postal_code: Some("75007".into()),
    });
    thing.geo = Some(GeoCoordinates::new(48.8584, 2.2945));
    thing.telephone = Some("+33-892-70-12-39".into());
    thing.url = Some("https://www.toureiffel.paris".into());
    thing.global_location_number = Some("1234567890123".into());
    thing.branch_code = Some("EIFFEL-01".into());
    thing.keywords = vec!["landmark".into(), "tourism".into(), "paris".into()];
    thing.identifiers = vec![
        ThingIdentifier::gln("1234567890123"),
        ThingIdentifier::new(IdentifierType::OpenStreetMap, "5013364"),
    ];
    thing.amenity_features = vec![
        AmenityFeature::new("Elevator"),
        AmenityFeature::with_value("Restaurant", "Le Jules Verne"),
    ];
    thing.opening_hours = vec![
        OpeningHoursSpecification::new(DayOfWeek::Monday, "09:30", "23:45"),
        OpeningHoursSpecification::new(DayOfWeek::Saturday, "09:00", "00:45"),
    ];
    thing.is_accessible_for_free = Some(false);
    thing.public_access = Some(true);
    thing.smoking_allowed = Some(false);
    thing.maximum_attendee_capacity = Some(10000);

    // Serialize round-trip
    let json = serde_json::to_string(&thing).unwrap();
    let deserialized: Thing = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "Eiffel Tower");
    assert_eq!(deserialized.alternate_name.as_deref(), Some("La Tour Eiffel"));
    assert_eq!(deserialized.thing_type, Some(ThingType::CivicStructure));
    assert_eq!(deserialized.keywords.len(), 3);
    assert_eq!(deserialized.identifiers.len(), 2);
    assert_eq!(deserialized.amenity_features.len(), 2);
    assert_eq!(deserialized.opening_hours.len(), 2);
    assert_eq!(deserialized.maximum_attendee_capacity, Some(10000));
    assert_eq!(deserialized.id, thing.id);
}

#[test]
fn test_thing_soft_delete_timestamps() {
    let mut thing = Thing::new("Temporary Thing");
    let created = thing.created_at;

    assert!(!thing.is_deleted);
    assert!(thing.deleted_at.is_none());

    thing.soft_delete();

    assert!(thing.is_deleted);
    assert!(thing.deleted_at.is_some());
    assert!(thing.deleted_at.unwrap() >= created);
}

#[test]
fn test_thing_ids_are_unique() {
    let a = Thing::new("Thing A");
    let b = Thing::new("Thing B");
    assert_ne!(a.id, b.id);
}

#[test]
fn test_thing_contained_in_thing_hierarchy() {
    let parent = Thing::new("New York City");
    let mut child = Thing::new("Central Park");
    child.contained_in_thing = Some(parent.id);
    assert_eq!(child.contained_in_thing, Some(parent.id));
}

// -- GeoCoordinates integration tests --

#[test]
fn test_geo_distance_symmetry() {
    let nyc = GeoCoordinates::new(40.7128, -74.0060);
    let london = GeoCoordinates::new(51.5074, -0.1278);
    let d1 = nyc.distance_to(&london);
    let d2 = london.distance_to(&nyc);
    assert!((d1 - d2).abs() < 0.01, "Distance should be symmetric: {d1} vs {d2}");
}

#[test]
fn test_geo_distance_triangle_inequality() {
    let nyc = GeoCoordinates::new(40.7128, -74.0060);
    let chicago = GeoCoordinates::new(41.8781, -87.6298);
    let lax = GeoCoordinates::new(33.9425, -118.4081);

    let d_nc = nyc.distance_to(&chicago);
    let d_cl = chicago.distance_to(&lax);
    let d_nl = nyc.distance_to(&lax);

    assert!(d_nl <= d_nc + d_cl + 1.0, "Triangle inequality violated");
}

// -- Identifier integration tests --

#[test]
fn test_multiple_identifier_types() {
    let thing = Thing::new("Test");
    let ids = vec![
        ThingIdentifier::gln("1234567890123"),
        ThingIdentifier::new(IdentifierType::Fips, "36061"),
        ThingIdentifier::new(IdentifierType::Gnis, "975772"),
        ThingIdentifier::new(IdentifierType::OpenStreetMap, "175905"),
        ThingIdentifier::new(IdentifierType::BranchCode, "NYC-001"),
        ThingIdentifier::new(IdentifierType::Custom("IATA".into()), "JFK"),
    ];

    // Serialize and deserialize
    let json = serde_json::to_string(&ids).unwrap();
    let deserialized: Vec<ThingIdentifier> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), 6);
    assert_eq!(deserialized[0].identifier_type, IdentifierType::GlobalLocationNumber);
    assert_eq!(deserialized[5].identifier_type, IdentifierType::Custom("IATA".into()));
    let _ = thing;
}

// -- Consent integration tests --

#[test]
fn test_consent_lifecycle() {
    let thing_id = Uuid::new_v4();

    // Create active consent
    let mut consent = Consent {
        id: Uuid::new_v4(),
        thing_id,
        consent_type: ConsentType::DataProcessing,
        status: ConsentStatus::Active,
        granted_at: Utc::now(),
        expires_at: Some(Utc::now() + Duration::days(365)),
    };
    assert!(consent.is_active());

    // Revoke
    consent.status = ConsentStatus::Revoked;
    assert!(!consent.is_active());
}

#[test]
fn test_consent_all_types_serialization() {
    let types = [
        ConsentType::DataProcessing,
        ConsentType::DataSharing,
        ConsentType::Marketing,
        ConsentType::Research,
    ];
    for ct in &types {
        let json = serde_json::to_string(ct).unwrap();
        let deserialized: ConsentType = serde_json::from_str(&json).unwrap();
        assert_eq!(&deserialized, ct);
    }
}

// -- ThingType integration tests --

#[test]
fn test_all_thing_types_display_and_roundtrip() {
    let types = [
        ThingType::LocalBusiness,
        ThingType::CivicStructure,
        ThingType::AdministrativeArea,
        ThingType::Landform,
        ThingType::Park,
        ThingType::Airport,
        ThingType::Hospital,
        ThingType::School,
        ThingType::Library,
        ThingType::Museum,
        ThingType::Restaurant,
        ThingType::Hotel,
        ThingType::Other("Stadium".into()),
    ];
    for pt in &types {
        let display = pt.to_string();
        assert!(!display.is_empty());
        let json = serde_json::to_string(pt).unwrap();
        let deserialized: ThingType = serde_json::from_str(&json).unwrap();
        assert_eq!(&deserialized, pt);
    }
}

// -- OpeningHours integration tests --

#[test]
fn test_opening_hours_full_week() {
    let days = [
        DayOfWeek::Monday,
        DayOfWeek::Tuesday,
        DayOfWeek::Wednesday,
        DayOfWeek::Thursday,
        DayOfWeek::Friday,
        DayOfWeek::Saturday,
        DayOfWeek::Sunday,
    ];
    let hours: Vec<OpeningHoursSpecification> = days
        .into_iter()
        .map(|d| OpeningHoursSpecification::new(d, "09:00", "17:00"))
        .collect();
    assert_eq!(hours.len(), 7);

    let json = serde_json::to_string(&hours).unwrap();
    let deserialized: Vec<OpeningHoursSpecification> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), 7);
}

// -- PostalAddress integration tests --

#[test]
fn test_address_default() {
    let addr = PostalAddress::default();
    assert!(addr.street_address.is_none());
    assert!(addr.address_locality.is_none());
    assert!(addr.address_region.is_none());
    assert!(addr.address_country.is_none());
    assert!(addr.postal_code.is_none());
}

#[test]
fn test_address_equality() {
    let a = PostalAddress {
        street_address: Some("123 Main St".into()),
        address_locality: Some("Springfield".into()),
        address_region: Some("IL".into()),
        address_country: Some("US".into()),
        postal_code: Some("62701".into()),
    };
    let b = a.clone();
    assert_eq!(a, b);
}
