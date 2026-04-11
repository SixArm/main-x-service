use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::thing::Thing;
use main_thing_index::privacy::{gdpr_export, mask_thing};

#[test]
fn test_mask_then_export_workflow() {
    let mut thing = Thing::new("Sensitive Thing");
    thing.telephone = Some("+1-555-867-5309".into());
    thing.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

    let masked = mask_thing(&thing);
    let export = gdpr_export(&masked);
    assert_eq!(export["name"], "Sensitive Thing");

    let tel = export["telephone"].as_str().unwrap();
    assert!(tel.ends_with("****"));
}

#[test]
fn test_gdpr_export_full_data() {
    let mut thing = Thing::new("GDPR Test");
    thing.description = Some("Full data export test".into());
    thing.telephone = Some("+44-20-7123-4567".into());
    thing.url = Some("https://example.co.uk".into());

    let export = gdpr_export(&thing);
    assert!(export.get("id").is_some());
    assert!(export.get("name").is_some());
    assert!(export.get("description").is_some());
    assert!(export.get("created_at").is_some());
    assert!(export.get("updated_at").is_some());
}

#[test]
fn test_mask_does_not_modify_original() {
    let mut thing = Thing::new("Original");
    thing.telephone = Some("+1-555-1234".into());

    let _masked = mask_thing(&thing);
    assert_eq!(thing.telephone.as_deref(), Some("+1-555-1234"));
}

#[test]
fn test_soft_delete_then_export() {
    let mut thing = Thing::new("Deleted Thing");
    thing.soft_delete();

    let export = gdpr_export(&thing);
    assert_eq!(export["is_deleted"], true);
    assert!(export["deleted_at"].as_str().is_some());
}
