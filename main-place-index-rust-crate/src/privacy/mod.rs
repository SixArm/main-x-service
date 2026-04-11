use crate::models::place::Place;
use serde_json::Value;

/// Mask sensitive fields in a Place for privacy.
pub fn mask_place(place: &Place) -> Place {
    let mut masked = place.clone();

    if let Some(tel) = &masked.telephone {
        masked.telephone = Some(mask_phone(tel));
    }
    if let Some(fax) = &masked.fax_number {
        masked.fax_number = Some(mask_phone(fax));
    }
    if let Some(geo) = &mut masked.geo {
        geo.latitude = (geo.latitude * 100.0).round() / 100.0;
        geo.longitude = (geo.longitude * 100.0).round() / 100.0;
    }

    masked
}

fn mask_phone(phone: &str) -> String {
    if phone.len() <= 4 {
        return "****".to_string();
    }
    let visible = &phone[..phone.len().saturating_sub(4)];
    format!("{visible}****")
}

/// Export place data for GDPR compliance (all fields, JSON format).
pub fn gdpr_export(place: &Place) -> Value {
    serde_json::to_value(place).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geo::GeoCoordinates;

    #[test]
    fn test_mask_telephone() {
        let mut place = Place::new("Test");
        place.telephone = Some("+1-555-867-5309".into());
        let masked = mask_place(&place);
        let tel = masked.telephone.unwrap();
        assert!(tel.ends_with("****"));
        assert!(!tel.contains("5309"));
    }

    #[test]
    fn test_mask_fax() {
        let mut place = Place::new("Test");
        place.fax_number = Some("+1-555-123-4567".into());
        let masked = mask_place(&place);
        let fax = masked.fax_number.unwrap();
        assert!(fax.ends_with("****"));
    }

    #[test]
    fn test_mask_geo_coordinates() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));
        let masked = mask_place(&place);
        let geo = masked.geo.unwrap();
        assert!((geo.latitude - 40.78).abs() < 0.01);
        assert!((geo.longitude - (-73.97)).abs() < 0.01);
    }

    #[test]
    fn test_mask_preserves_name() {
        let place = Place::new("Central Park");
        let masked = mask_place(&place);
        assert_eq!(masked.name, "Central Park");
    }

    #[test]
    fn test_mask_no_sensitive_fields() {
        let place = Place::new("Test");
        let masked = mask_place(&place);
        assert!(masked.telephone.is_none());
        assert!(masked.fax_number.is_none());
    }

    #[test]
    fn test_mask_short_phone() {
        let mut place = Place::new("Test");
        place.telephone = Some("123".into());
        let masked = mask_place(&place);
        assert_eq!(masked.telephone.as_deref(), Some("****"));
    }

    #[test]
    fn test_gdpr_export() {
        let mut place = Place::new("Export Test");
        place.description = Some("A test place".into());
        let export = gdpr_export(&place);
        assert_eq!(export["name"], "Export Test");
        assert_eq!(export["description"], "A test place");
    }

    #[test]
    fn test_gdpr_export_has_all_fields() {
        let place = Place::new("Full Export");
        let export = gdpr_export(&place);
        assert!(export.get("id").is_some());
        assert!(export.get("name").is_some());
        assert!(export.get("created_at").is_some());
        assert!(export.get("is_deleted").is_some());
    }
}
