use main_place_service::models::address::PostalAddress;
use main_place_service::models::amenity::AmenityFeature;
use main_place_service::models::consent::{Consent, ConsentStatus, ConsentType};
use main_place_service::models::geo::GeoCoordinates;
use main_place_service::models::identifier::{IdentifierType, PlaceIdentifier};
use main_place_service::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};
use main_place_service::models::place::Place;
use main_place_service::models::place_type::PlaceType;

use chrono::{Duration, Utc};
use uuid::Uuid;

// -- Place lifecycle tests --

#[test]
fn test_place_full_construction_and_serialization() {
    let mut place = Place::new("Eiffel Tower");
    place.alternate_name = Some("La Tour Eiffel".into());
    place.description = Some("Iconic iron lattice tower in Paris".into());
    place.place_type = Some(PlaceType::CivicStructure);
    place.address = Some(PostalAddress {
        street_address: Some("Champ de Mars, 5 Avenue Anatole France".into()),
        address_locality: Some("Paris".into()),
        address_region: Some("Île-de-France".into()),
        address_country: Some("FR".into()),
        postal_code: Some("75007".into()),
    });
    place.geo = Some(GeoCoordinates::new(48.8584, 2.2945));
    place.telephone = Some("+33-892-70-12-39".into());
    place.url = Some("https://www.toureiffel.paris".into());
    place.global_location_number = Some("1234567890123".into());
    place.branch_code = Some("EIFFEL-01".into());
    place.keywords = vec!["landmark".into(), "tourism".into(), "paris".into()];
    place.identifiers = vec![
        PlaceIdentifier::gln("1234567890123"),
        PlaceIdentifier::new(IdentifierType::OpenStreetMap, "5013364"),
    ];
    place.amenity_features = vec![
        AmenityFeature::new("Elevator"),
        AmenityFeature::with_value("Restaurant", "Le Jules Verne"),
    ];
    place.opening_hours = vec![
        OpeningHoursSpecification::new(DayOfWeek::Monday, "09:30", "23:45"),
        OpeningHoursSpecification::new(DayOfWeek::Saturday, "09:00", "00:45"),
    ];
    place.is_accessible_for_free = Some(false);
    place.public_access = Some(true);
    place.smoking_allowed = Some(false);
    place.maximum_attendee_capacity = Some(10000);

    // Serialize round-trip
    let json = serde_json::to_string(&place).unwrap();
    let deserialized: Place = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "Eiffel Tower");
    assert_eq!(deserialized.alternate_name.as_deref(), Some("La Tour Eiffel"));
    assert_eq!(deserialized.place_type, Some(PlaceType::CivicStructure));
    assert_eq!(deserialized.keywords.len(), 3);
    assert_eq!(deserialized.identifiers.len(), 2);
    assert_eq!(deserialized.amenity_features.len(), 2);
    assert_eq!(deserialized.opening_hours.len(), 2);
    assert_eq!(deserialized.maximum_attendee_capacity, Some(10000));
    assert_eq!(deserialized.id, place.id);
}

#[test]
fn test_place_soft_delete_timestamps() {
    let mut place = Place::new("Temporary Place");
    let created = place.created_at;

    assert!(!place.is_deleted);
    assert!(place.deleted_at.is_none());

    place.soft_delete();

    assert!(place.is_deleted);
    assert!(place.deleted_at.is_some());
    assert!(place.deleted_at.unwrap() >= created);
}

#[test]
fn test_place_ids_are_unique() {
    let a = Place::new("Place A");
    let b = Place::new("Place B");
    assert_ne!(a.id, b.id);
}

#[test]
fn test_place_contained_in_place_hierarchy() {
    let parent = Place::new("New York City");
    let mut child = Place::new("Central Park");
    child.contained_in_place = Some(parent.id);
    assert_eq!(child.contained_in_place, Some(parent.id));
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
    let place = Place::new("Test");
    let ids = vec![
        PlaceIdentifier::gln("1234567890123"),
        PlaceIdentifier::new(IdentifierType::Fips, "36061"),
        PlaceIdentifier::new(IdentifierType::Gnis, "975772"),
        PlaceIdentifier::new(IdentifierType::OpenStreetMap, "175905"),
        PlaceIdentifier::new(IdentifierType::BranchCode, "NYC-001"),
        PlaceIdentifier::new(IdentifierType::Custom("IATA".into()), "JFK"),
    ];

    // Serialize and deserialize
    let json = serde_json::to_string(&ids).unwrap();
    let deserialized: Vec<PlaceIdentifier> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), 6);
    assert_eq!(deserialized[0].identifier_type, IdentifierType::GlobalLocationNumber);
    assert_eq!(deserialized[5].identifier_type, IdentifierType::Custom("IATA".into()));
    let _ = place;
}

// -- Consent integration tests --

#[test]
fn test_consent_lifecycle() {
    let place_id = Uuid::new_v4();

    // Create active consent
    let mut consent = Consent {
        id: Uuid::new_v4(),
        place_id,
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

// -- PlaceType integration tests --

#[test]
fn test_all_place_types_display_and_roundtrip() {
    let types = [
        PlaceType::LocalBusiness,
        PlaceType::CivicStructure,
        PlaceType::AdministrativeArea,
        PlaceType::Landform,
        PlaceType::Park,
        PlaceType::Airport,
        PlaceType::Hospital,
        PlaceType::School,
        PlaceType::Library,
        PlaceType::Museum,
        PlaceType::Restaurant,
        PlaceType::Hotel,
        PlaceType::Other("Stadium".into()),
    ];
    for pt in &types {
        let display = pt.to_string();
        assert!(!display.is_empty());
        let json = serde_json::to_string(pt).unwrap();
        let deserialized: PlaceType = serde_json::from_str(&json).unwrap();
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
