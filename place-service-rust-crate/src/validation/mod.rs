//! Data-quality validation and normalization for [`Place`] records.
//!
//! Two entry points cover the boundary checks the API layer applies before a
//! place is persisted:
//!
//! - [`validate_place`] collects *all* rule violations (it does not
//!   short-circuit on the first), so the caller can return a complete `422`
//!   error body in one round-trip.
//! - [`normalize_place`] canonicalizes free-text fields in place (trimming,
//!   title-casing locality, upper-casing region/country) so equal-but-differently-typed
//!   inputs compare and store consistently.
//!
//! The rules are intentionally lightweight (range checks, prefix checks,
//! digit counts) rather than authoritative validations — e.g. GLN is only
//! length/digit-checked here, not check-digit-verified.
//!
//! # Examples
//!
//! ```
//! use place_service::models::place::Place;
//! use place_service::validation::{validate_place, normalize_place};
//!
//! let mut place = Place::new("  central park  ");
//! normalize_place(&mut place);
//! assert_eq!(place.name, "central park"); // trimmed (name is not title-cased)
//! assert!(validate_place(&place).is_empty());
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::place::Place;

/// A single validation failure: which field failed and why.
///
/// [`validate_place`] returns a `Vec<ValidationError>`; an empty vec means the
/// record is valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ValidationError {
    /// The offending field path (e.g. `"name"`, `"geo.latitude"`).
    pub field: String,
    /// A human-readable explanation suitable for an API error response.
    pub message: String,
}

/// Validate a place, returning all validation errors.
///
/// Checks (each independent, all reported): non-empty name; latitude in
/// `[-90, 90]` and longitude in `[-180, 180]` when geo is present; 13-digit
/// numeric GLN; `http(s)://` URL scheme; `+`-prefixed telephone; and that any
/// present address carries at least a locality, postal code, or country.
///
/// # Examples
///
/// ```
/// use place_service::models::place::Place;
/// use place_service::models::geo::GeoCoordinates;
/// use place_service::validation::validate_place;
///
/// let mut place = Place::new("Test");
/// place.geo = Some(GeoCoordinates::new(91.0, 0.0)); // out of range
/// let errors = validate_place(&place);
/// assert!(errors.iter().any(|e| e.field == "geo.latitude"));
/// ```
pub fn validate_place(place: &Place) -> Vec<ValidationError> {
    // Accumulate every violation rather than returning on the first, so the
    // API can surface a complete error body in one response.
    let mut errors = Vec::new();

    // Name is the only strictly required field.
    if place.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "Name is required and must not be empty".into(),
        });
    }

    // Geo bounds: WGS-84 latitude/longitude ranges.
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

    // GLN must be exactly 13 ASCII digits (check digit not verified here).
    if let Some(gln) = &place.global_location_number
        && (gln.len() != 13 || !gln.chars().all(|c| c.is_ascii_digit()))
    {
        errors.push(ValidationError {
            field: "global_location_number".into(),
            message: "GLN must be exactly 13 digits".into(),
        });
    }

    // URL must carry an http(s) scheme; we do not parse the full URL.
    if let Some(url) = &place.url
        && !url.starts_with("http://") && !url.starts_with("https://")
    {
        errors.push(ValidationError {
            field: "url".into(),
            message: "URL must start with http:// or https://".into(),
        });
    }

    // Telephone, when non-empty, must use the international `+` prefix.
    if let Some(tel) = &place.telephone
        && !tel.is_empty() && !tel.starts_with('+')
    {
        errors.push(ValidationError {
            field: "telephone".into(),
            message: "Telephone must start with + for international format".into(),
        });
    }

    // A present address needs at least one locating field; a lone street line
    // is not enough to place it.
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
///
/// Mutates in place: trims the name, title-cases the locality, and
/// upper-cases the region and country (each also trimmed). Idempotent —
/// running it twice yields the same result.
///
/// # Examples
///
/// ```
/// use place_service::models::place::Place;
/// use place_service::models::address::PostalAddress;
/// use place_service::validation::normalize_place;
///
/// let mut place = Place::new("Test");
/// place.address = Some(PostalAddress {
///     street_address: None,
///     address_locality: Some("new york".into()),
///     address_region: Some("ny".into()),
///     address_country: Some("us".into()),
///     postal_code: None,
/// });
/// normalize_place(&mut place);
/// let addr = place.address.unwrap();
/// assert_eq!(addr.address_locality.as_deref(), Some("New York"));
/// assert_eq!(addr.address_region.as_deref(), Some("NY"));
/// ```
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

/// Title-case a whitespace-separated string: upper-case the first letter of
/// each word and lower-case the rest, collapsing runs of whitespace to single
/// spaces. Used to canonicalize locality names like `"new york"` → `"New York"`.
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            // Split off the first char to upper-case it; lower-case the tail.
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

    /// A minimal place (name only) passes validation.
    #[test]
    fn test_valid_place() {
        let place = Place::new("Central Park");
        let errors = validate_place(&place);
        assert!(errors.is_empty(), "Errors: {errors:?}");
    }

    /// An empty name is rejected.
    #[test]
    fn test_empty_name() {
        let place = Place::new("");
        let errors = validate_place(&place);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    /// A whitespace-only name is treated as empty and rejected.
    #[test]
    fn test_whitespace_name() {
        let place = Place::new("   ");
        let errors = validate_place(&place);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    /// Latitude above 90 is flagged.
    #[test]
    fn test_invalid_latitude() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(91.0, 0.0));
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "geo.latitude"));
    }

    /// Longitude below -180 is flagged.
    #[test]
    fn test_invalid_longitude() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(0.0, -181.0));
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "geo.longitude"));
    }

    /// Boundary coordinates (90, 180) are valid.
    #[test]
    fn test_valid_coordinates() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(90.0, 180.0));
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    /// A GLN shorter than 13 digits is rejected.
    #[test]
    fn test_invalid_gln_too_short() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("123".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "global_location_number"));
    }

    /// A 13-char GLN containing a non-digit is rejected.
    #[test]
    fn test_invalid_gln_non_digit() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("123456789012A".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "global_location_number"));
    }

    /// A well-formed 13-digit GLN passes.
    #[test]
    fn test_valid_gln() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("1234567890123".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    /// A URL without an http(s) scheme is rejected.
    #[test]
    fn test_invalid_url() {
        let mut place = Place::new("Test");
        place.url = Some("not-a-url".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "url"));
    }

    /// An `https://` URL passes.
    #[test]
    fn test_valid_url() {
        let mut place = Place::new("Test");
        place.url = Some("https://example.com".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    /// A telephone without the international `+` prefix is rejected.
    #[test]
    fn test_invalid_telephone() {
        let mut place = Place::new("Test");
        place.telephone = Some("555-1234".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "telephone"));
    }

    /// A `+`-prefixed telephone passes.
    #[test]
    fn test_valid_telephone() {
        let mut place = Place::new("Test");
        place.telephone = Some("+1-555-1234".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty());
    }

    /// An address with only a street line (no locating field) is rejected.
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

    /// An address with a locality satisfies the locating-field rule.
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

    /// Multiple simultaneous violations are all reported.
    #[test]
    fn test_multiple_validation_errors() {
        let mut place = Place::new("");
        place.geo = Some(GeoCoordinates::new(100.0, 200.0));
        place.url = Some("bad-url".into());
        let errors = validate_place(&place);
        assert!(errors.len() >= 3, "Expected 3+ errors, got: {errors:?}");
    }

    /// Normalization trims surrounding whitespace from the name.
    #[test]
    fn test_normalize_place_name() {
        let mut place = Place::new("  Central Park  ");
        normalize_place(&mut place);
        assert_eq!(place.name, "Central Park");
    }

    /// Normalization title-cases locality and upper-cases region/country.
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

    /// `title_case` handles multi-word, all-caps, and empty inputs.
    #[test]
    fn test_title_case() {
        assert_eq!(title_case("hello world"), "Hello World");
        assert_eq!(title_case("SAN FRANCISCO"), "San Francisco");
        assert_eq!(title_case(""), "");
    }
}
