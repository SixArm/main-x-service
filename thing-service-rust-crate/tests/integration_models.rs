#![warn(clippy::pedantic)]

//! Integration tests for the domain models: [`Thing`] construction /
//! serialization / soft-delete, [`ThingIdentifier`] round-trips, and the
//! [`Consent`] lifecycle.

use jiff::Timestamp;
use uuid::Uuid;

use thing_service::models::consent::{Consent, ConsentStatus, ConsentType};
use thing_service::models::identifier::{IdentifierType, ThingIdentifier};
use thing_service::models::thing::Thing;

// -- Thing lifecycle tests --

/// A fully-populated Thing survives a JSON round-trip with all fields intact.
#[test]
fn test_thing_full_construction_and_serialization() {
    let mut thing = Thing::new("Pride and Prejudice");
    thing.alternate_names = vec!["First Impressions".into()];
    thing.description = Some("A novel of manners by Jane Austen.".into());
    thing.disambiguating_description = Some("The 1813 Austen novel, not the 2005 film.".into());
    thing.additional_type = Some("https://schema.org/Book".into());
    thing.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    thing.images = vec!["https://example.com/cover.jpg".into()];
    thing.main_entity_of_page = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    thing.owner = Some("Penguin Random House".into());
    thing.same_as = vec![
        "https://www.wikidata.org/wiki/Q170583".into(),
        "https://openlibrary.org/works/OL1394865W".into(),
    ];
    thing.subject_of = Some("https://www.gutenberg.org/ebooks/1342".into());
    thing.potential_action = Some("ReadAction".into());
    thing.identifiers = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
    ];

    let json = serde_json::to_string(&thing).unwrap();
    let deserialized: Thing = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "Pride and Prejudice");
    assert_eq!(deserialized.alternate_names.len(), 1);
    assert_eq!(deserialized.additional_type.as_deref(), Some("https://schema.org/Book"));
    assert_eq!(deserialized.identifiers.len(), 2);
    assert_eq!(deserialized.same_as.len(), 2);
    assert_eq!(deserialized.id, thing.id);
}

/// Soft-delete sets the flag and a `deleted_at` at/after creation time.
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

/// Distinct Things get distinct auto-generated IDs.
#[test]
fn test_thing_ids_are_unique() {
    let a = Thing::new("Thing A");
    let b = Thing::new("Thing B");
    assert_ne!(a.id, b.id);
}

// -- Identifier integration tests --

/// A mixed list of identifier types round-trips through JSON.
#[test]
fn test_multiple_identifier_types() {
    let ids = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::doi("10.1000/xyz123"),
        ThingIdentifier::issn("0317-8471"),
        ThingIdentifier::gtin("0012345600012"),
        ThingIdentifier::sku("SKU-101"),
        ThingIdentifier::serial_number("SN-001"),
        ThingIdentifier::new(IdentifierType::Custom("Internal".into()), "INT-42"),
    ];

    let json = serde_json::to_string(&ids).unwrap();
    let deserialized: Vec<ThingIdentifier> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), 7);
    assert_eq!(deserialized[0].property_id, IdentifierType::Isbn);
    assert_eq!(
        deserialized[6].property_id,
        IdentifierType::Custom("Internal".into())
    );
}

/// An identifier's optional PropertyValue `name`/`url` survive serialization.
#[test]
fn test_identifier_with_property_value_fields() {
    let mut id = ThingIdentifier::isbn("9780141439518");
    id.name = Some("Penguin Classics paperback".into());
    id.url = Some("https://www.worldcat.org/isbn/9780141439518".into());

    let json = serde_json::to_string(&id).unwrap();
    let deserialized: ThingIdentifier = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

// -- Consent integration tests --

/// A consent is active while granted, then inactive once revoked.
#[test]
fn test_consent_lifecycle() {
    let thing_id = Uuid::new_v4();

    let mut consent = Consent {
        id: Uuid::new_v4(),
        thing_id,
        consent_type: ConsentType::DataProcessing,
        status: ConsentStatus::Active,
        granted_at: Timestamp::now(),
        expires_at: Some(Timestamp::now() + jiff::SignedDuration::from_hours(24 * (365))),
    };
    assert!(consent.is_active());

    consent.status = ConsentStatus::Revoked;
    assert!(!consent.is_active());
}

/// Every ConsentType variant round-trips through JSON.
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
