#![warn(clippy::pedantic)]

//! Integration tests for the privacy pipeline.
//!
//! These cover [`mask_place`] and [`gdpr_export`] end to end: masking before
//! export, full data export, the non-mutating contract, and exporting a
//! soft-deleted record.

use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;
use place_service::privacy::{gdpr_export, mask_place};

/// Masking then exporting yields redacted contact fields in the JSON.
#[test]
fn test_mask_then_export_workflow() {
    let mut place = Place::new("Sensitive Place");
    place.telephone = Some("+1-555-867-5309".into());
    place.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

    let masked = mask_place(&place);
    let export = gdpr_export(&masked);
    assert_eq!(export["name"], "Sensitive Place");

    let tel = export["telephone"].as_str().unwrap();
    assert!(tel.ends_with("****"));
}

/// An unmasked GDPR export contains all the record's fields.
#[test]
fn test_gdpr_export_full_data() {
    let mut place = Place::new("GDPR Test");
    place.description = Some("Full data export test".into());
    place.telephone = Some("+44-20-7123-4567".into());
    place.url = Some("https://example.co.uk".into());

    let export = gdpr_export(&place);
    assert!(export.get("id").is_some());
    assert!(export.get("name").is_some());
    assert!(export.get("description").is_some());
    assert!(export.get("created_at").is_some());
    assert!(export.get("updated_at").is_some());
}

/// Masking returns a copy and leaves the original record unchanged.
#[test]
fn test_mask_does_not_modify_original() {
    let mut place = Place::new("Original");
    place.telephone = Some("+1-555-1234".into());

    let _masked = mask_place(&place);
    assert_eq!(place.telephone.as_deref(), Some("+1-555-1234"));
}

/// A soft-deleted record still exports, with the deletion flag and timestamp.
#[test]
fn test_soft_delete_then_export() {
    let mut place = Place::new("Deleted Place");
    place.soft_delete();

    let export = gdpr_export(&place);
    assert_eq!(export["is_deleted"], true);
    assert!(export["deleted_at"].as_str().is_some());
}
