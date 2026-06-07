//! Integration tests for the privacy workflow ([`mask_thing`] /
//! [`gdpr_export`]): masking sensitive fields, full GDPR export, and the
//! immutability of the original record.

use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;
use thing_service::privacy::{gdpr_export, mask_thing};

/// Masking then exporting yields withheld owner and a truncated identifier.
#[test]
fn test_mask_then_export_workflow() {
    let mut thing = Thing::new("Private Diary");
    thing.owner = Some("Jane Doe".into());
    thing.identifiers = vec![ThingIdentifier::serial_number("SN-1234567890")];

    let masked = mask_thing(&thing);
    let export = gdpr_export(&masked);
    assert_eq!(export["name"], "Private Diary");
    assert_eq!(export["owner"], "[owner withheld]");

    let id_value = export["identifiers"][0]["value"].as_str().unwrap();
    assert!(id_value.starts_with("****"));
    assert!(id_value.ends_with("7890"));
}

/// A full GDPR export contains all the expected top-level fields.
#[test]
fn test_gdpr_export_full_data() {
    let mut thing = Thing::new("GDPR Test");
    thing.description = Some("Full export test".into());
    thing.url = Some("https://example.co.uk".into());
    thing.same_as = vec!["https://other.example.com".into()];

    let export = gdpr_export(&thing);
    assert!(export.get("id").is_some());
    assert!(export.get("name").is_some());
    assert!(export.get("description").is_some());
    assert!(export.get("url").is_some());
    assert!(export.get("same_as").is_some());
    assert!(export.get("created_at").is_some());
    assert!(export.get("updated_at").is_some());
}

/// Masking returns a copy and leaves the original record untouched.
#[test]
fn test_mask_does_not_modify_original() {
    let mut thing = Thing::new("Original");
    thing.owner = Some("Owner Name".into());

    let _ = mask_thing(&thing);
    assert_eq!(thing.owner.as_deref(), Some("Owner Name"));
}

/// A soft-deleted thing exports with its delete flag and timestamp set.
#[test]
fn test_soft_delete_then_export() {
    let mut thing = Thing::new("Deleted Thing");
    thing.soft_delete();

    let export = gdpr_export(&thing);
    assert_eq!(export["is_deleted"], true);
    assert!(export["deleted_at"].as_str().is_some());
}
