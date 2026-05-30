use crate::models::place::Place;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Validate a place, returning all validation errors.
pub fn validate_place(place: &Place) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if place.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "Name is required and must not be empty".into(),
        });
    }

    if let Some(geo) = &place.geo {
        if geo.latitude < -90.0 || geo.latitude > 90.0 {
            errors.push(ValidationError {
                field: "geo.latitude".into(),
                message: format!("Latitude must be between -90 and 90, got {}", geo.latitude),
            });
        }
        if geo.longitude < -180.0 || geo.longitude > 180.0 {
            errors.push(ValidationError {
                field: "geo.longitude".into(),
                message: format!("Longitude must be between -180 and 180, got {}", geo.longitude),
            });
        }
    }

    if let Some(gln) = &place.global_location_number
        && (gln.len() != 13 || !gln.chars().all(|c| c.is_ascii_digit()))
    {
        errors.push(ValidationError {
            field: "global_location_number".into(),
            message: "GLN must be exactly 13 digits".into(),
        });
    }

    if let Some(url) = &place.url
        && !url.starts_with("http://") && !url.starts_with("https://")
    {
        errors.push(ValidationError {
            field: "url".into(),
            message: "URL must start with http:// or https://".into(),
        });
    }

    if let Some(tel) = &place.telephone
        && !tel.is_empty() && !tel.starts_with('+')
    {
        errors.push(ValidationError {
            field: "telephone".into(),
            message: "Telephone must start with + for international format".into(),
        });
    }

    if let Some(addr) = &place.address {
        let has_locality = addr.address_locality.as_ref().is_some_and(|s| !s.is_empty());
        let has_postal = addr.postal_code.as_ref().is_some_and(|s| !s.is_empty());
        let has_country = addr.address_country.as_ref().is_some_and(|s| !s.is_empty());
        if !has_locality && !has_postal && !has_country {
            errors.push(ValidationError {
                field: "address".into(),
                message: "Address must have at least locality, postal code, or country".into(),
            });
        }
    }

    errors
}

/// Normalize a place's address (title-case locality, uppercase region/country).
pub fn normalize_place(place: &mut Place) {
    place.name = place.name.trim().to_string();

    if let Some(addr) = &mut place.address {
        if let Some(locality) = &mut addr.address_locality {
            *locality = title_case(locality.trim());
        }
        if let Some(region) = &mut addr.address_region {
            *region = region.trim().to_uppercase();
        }
        if let Some(country) = &mut addr.address_country {
            *country = country.trim().to_uppercase();
        }
    }
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::address::PostalAddress;
    use crate::models::geo::GeoCoordinates;

    #[test]
    fn test_valid_place() {
        let place = Place::new("Central Park");
        let errors = validate_place(&place);
        assert!(errors.is_empty(), "Errors: {errors:?}");
    }

    #[test]
    fn test_empty_name() {
        let place = Place::new("");
        let errors = validate_place(&place);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[test]
    fn test_whitespace_name() {
        let place = Place::new("   ");
        let errors = validate_place(&place);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[test]
    fn test_invalid_latitude() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(91.0, 0.0));
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "geo.latitude"));
    }

    #[test]
    fn test_invalid_longitude() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(0.0, -181.0));
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "geo.longitude"));
    }

    #[test]
    fn test_valid_coordinates() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(90.0, 180.0));
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_gln_too_short() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("123".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "global_location_number"));
    }

    #[test]
    fn test_invalid_gln_non_digit() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("123456789012A".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "global_location_number"));
    }

    #[test]
    fn test_valid_gln() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("1234567890123".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_url() {
        let mut place = Place::new("Test");
        place.url = Some("not-a-url".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "url"));
    }

    #[test]
    fn test_valid_url() {
        let mut place = Place::new("Test");
        place.url = Some("https://example.com".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_telephone() {
        let mut place = Place::new("Test");
        place.telephone = Some("555-1234".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "telephone"));
    }

    #[test]
    fn test_valid_telephone() {
        let mut place = Place::new("Test");
        place.telephone = Some("+1-555-1234".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_address_missing_required_fields() {
        let mut place = Place::new("Test");
        place.address = Some(PostalAddress {
            street_address: Some("123 Main".into()),
            address_locality: None,
            address_region: None,
            address_country: None,
            postal_code: None,
        });
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "address"));
    }

    #[test]
    fn test_address_with_locality() {
        let mut place = Place::new("Test");
        place.address = Some(PostalAddress {
            street_address: None,
            address_locality: Some("Town".into()),
            address_region: None,
            address_country: None,
            postal_code: None,
        });
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_multiple_validation_errors() {
        let mut place = Place::new("");
        place.geo = Some(GeoCoordinates::new(100.0, 200.0));
        place.url = Some("bad-url".into());
        let errors = validate_place(&place);
        assert!(errors.len() >= 3, "Expected 3+ errors, got: {errors:?}");
    }

    #[test]
    fn test_normalize_place_name() {
        let mut place = Place::new("  Central Park  ");
        normalize_place(&mut place);
        assert_eq!(place.name, "Central Park");
    }

    #[test]
    fn test_normalize_address() {
        let mut place = Place::new("Test");
        place.address = Some(PostalAddress {
            street_address: None,
            address_locality: Some("new york".into()),
            address_region: Some("ny".into()),
            address_country: Some("us".into()),
            postal_code: None,
        });
        normalize_place(&mut place);
        let addr = place.address.as_ref().unwrap();
        assert_eq!(addr.address_locality.as_deref(), Some("New York"));
        assert_eq!(addr.address_region.as_deref(), Some("NY"));
        assert_eq!(addr.address_country.as_deref(), Some("US"));
    }

    #[test]
    fn test_title_case() {
        assert_eq!(title_case("hello world"), "Hello World");
        assert_eq!(title_case("SAN FRANCISCO"), "San Francisco");
        assert_eq!(title_case(""), "");
    }
}
