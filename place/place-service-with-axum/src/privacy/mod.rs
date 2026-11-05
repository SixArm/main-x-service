//! Privacy controls: field masking and GDPR data export for [`Place`] records.
//!
//! Two complementary operations:
//!
//! - [`mask_place`] returns a redacted *copy* suitable for low-trust views —
//!   phone/fax numbers keep only their leading digits and geo coordinates are
//!   rounded to ~1 km precision (two decimal places), so a place can be shown
//!   without disclosing an exact contact number or pinpoint location.
//! - [`gdpr_export`] serializes the *full* record to JSON for a data-subject
//!   access request (right of access / portability).
//!
//! Masking never mutates the input; the original record is left intact.
//!
//! # Examples
//!
//! ```
//! use bigdecimal::BigDecimal;
//! use place_service::models::place::Place;
//! use place_service::models::geo::GeoCoordinates;
//! use place_service::privacy::mask_place;
//!
//! let mut place = Place::new("Sensitive Place");
//! place.telephone = Some("+1-555-867-5309".into());
//! place.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));
//!
//! let masked = mask_place(&place);
//! assert!(masked.telephone.unwrap().ends_with("****"));
//! // Exactly two decimals — not `40.78000000000000113…`, which is what
//! // rounding an `f64` used to leave behind.
//! assert_eq!(masked.geo.unwrap().latitude_as_decimal_degrees, "40.78".parse::<BigDecimal>().unwrap());
//! ```

use crate::models::place::Place;
use bigdecimal::RoundingMode;
use serde_json::Value;

/// Mask sensitive fields in a Place for privacy.
///
/// Returns a clone with telephone and fax masked (all but the last four
/// characters) and geo coordinates rounded to two decimal places (~1 km). All
/// other fields are copied verbatim; the input is not modified.
///
/// # Examples
///
/// ```
/// use place_service::models::place::Place;
/// use place_service::privacy::mask_place;
///
/// let mut place = Place::new("Test");
/// place.telephone = Some("+1-555-867-5309".into());
/// let masked = mask_place(&place);
/// assert!(!masked.telephone.unwrap().contains("5309"));
/// // The original is untouched.
/// assert_eq!(place.telephone.as_deref(), Some("+1-555-867-5309"));
/// ```
#[must_use]
pub fn mask_place(place: &Place) -> Place {
    // Work on a clone so the caller's record is never mutated.
    let mut masked = place.clone();

    if let Some(tel) = &masked.telephone {
        masked.telephone = Some(mask_phone(tel));
    }
    if let Some(fax) = &masked.fax_number {
        masked.fax_number = Some(mask_phone(fax));
    }
    if let Some(geo) = &mut masked.geo {
        // Round to two decimals (~1 km) so the exact location is obscured.
        //
        // Exact decimal rounding, so the result is the two-decimal value
        // and nothing more: the old `(x * 100.0).round() / 100.0` on `f64`
        // returned things like `40.78000000000000113686837721616029739`,
        // and its half-way behaviour depended on binary representation
        // rather than on the decimal the caller sent (`40.785` is really
        // `40.78499999…` as an `f64`, so it rounded *down*). Half-up here
        // is the rule a reader expects, applied to the digits actually
        // stored.
        geo.latitude_as_decimal_degrees = geo
            .latitude_as_decimal_degrees
            .with_scale_round(2, RoundingMode::HalfUp);
        geo.longitude_as_decimal_degrees = geo
            .longitude_as_decimal_degrees
            .with_scale_round(2, RoundingMode::HalfUp);
    }

    masked
}

/// Mask a phone/fax number, keeping all but the last four characters and
/// replacing the tail with `"****"`.
///
/// Numbers of four characters or fewer are fully redacted to `"****"` so no
/// digits leak from very short inputs.
fn mask_phone(phone: &str) -> String {
    // Too short to reveal any prefix without exposing most of the number.
    if phone.len() <= 4 {
        return "****".to_string();
    }
    // `saturating_sub` is defensive; the length guard above already ensures
    // a non-empty visible prefix.
    let visible = &phone[..phone.len().saturating_sub(4)];
    format!("{visible}****")
}

/// Export place data for GDPR compliance (all fields, JSON format).
///
/// Serializes the entire record via Serde. Falls back to JSON `null` if
/// serialization somehow fails (it does not for the `Place` type).
///
/// # Examples
///
/// ```
/// use place_service::models::place::Place;
/// use place_service::privacy::gdpr_export;
///
/// let place = Place::new("Export Test");
/// let export = gdpr_export(&place);
/// assert_eq!(export["name"], "Export Test");
/// ```
#[must_use]
pub fn gdpr_export(place: &Place) -> Value {
    serde_json::to_value(place).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geo::GeoCoordinates;
    use bigdecimal::BigDecimal;

    /// Telephone is masked, hiding the trailing digits.
    #[test]
    fn test_mask_telephone() {
        let mut place = Place::new("Test");
        place.telephone = Some("+1-555-867-5309".into());
        let masked = mask_place(&place);
        let tel = masked.telephone.unwrap();
        assert!(tel.ends_with("****"));
        assert!(!tel.contains("5309"));
    }

    /// Fax number is masked like a telephone.
    #[test]
    fn test_mask_fax() {
        let mut place = Place::new("Test");
        place.fax_number = Some("+1-555-123-4567".into());
        let masked = mask_place(&place);
        let fax = masked.fax_number.unwrap();
        assert!(fax.ends_with("****"));
    }

    /// Geo coordinates are rounded to ~1 km precision.
    #[test]
    fn test_mask_geo_coordinates() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(40.782_934_56, -73.965_432_10));
        let masked = mask_place(&place);
        let geo = masked.geo.unwrap();
        // Exact: masking now yields the two-decimal value and nothing
        // more. The float version returned `40.78000000000000113686…`,
        // which only an epsilon comparison could assert against.
        assert_eq!(
            geo.latitude_as_decimal_degrees,
            "40.78".parse::<BigDecimal>().unwrap()
        );
        assert_eq!(
            geo.longitude_as_decimal_degrees,
            "-73.97".parse::<BigDecimal>().unwrap()
        );
    }

    /// Non-sensitive fields like the name are preserved.
    #[test]
    fn test_mask_preserves_name() {
        let place = Place::new("Central Park");
        let masked = mask_place(&place);
        assert_eq!(masked.name, "Central Park");
    }

    /// Masking a record with no sensitive fields leaves them absent.
    #[test]
    fn test_mask_no_sensitive_fields() {
        let place = Place::new("Test");
        let masked = mask_place(&place);
        assert!(masked.telephone.is_none());
        assert!(masked.fax_number.is_none());
    }

    /// A short phone (≤4 chars) is fully redacted to `****`.
    #[test]
    fn test_mask_short_phone() {
        let mut place = Place::new("Test");
        place.telephone = Some("123".into());
        let masked = mask_place(&place);
        assert_eq!(masked.telephone.as_deref(), Some("****"));
    }

    /// GDPR export includes user-supplied fields.
    #[test]
    fn test_gdpr_export() {
        let mut place = Place::new("Export Test");
        place.description = Some("A test place".into());
        let export = gdpr_export(&place);
        assert_eq!(export["name"], "Export Test");
        assert_eq!(export["description"], "A test place");
    }

    /// GDPR export includes system fields (id, timestamps, soft-delete flag).
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
