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
//! digit counts) with two exceptions: the GLN is fully verified, including
//! its GS1 mod-10 check digit (see [`gln_is_valid`]), and opening-hours times
//! are checked against a real 24-hour `HH:MM` clock (see [`time_is_valid`]).
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

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::place::Place;

/// A single validation failure: which field failed and why.
///
/// [`validate_place`] returns a `Vec<ValidationError>`; an empty vec means the
/// record is valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ValidationError {
    /// The offending field path (e.g. `"name"`, `"geo.latitude_as_decimal_degrees"`).
    pub field: String,
    /// A human-readable explanation suitable for an API error response.
    pub message: String,
}

/// SEC-M1 — maximum length, in Unicode scalar values (`.chars().count()`),
/// of any single scalar text field. Bounds the per-field cost of the
/// matcher's character-level string comparisons so one huge string cannot be
/// a CPU/memory `DoS`, amplified across the `check-duplicates`/`deduplicate`
/// scans.
const MAX_TEXT_LEN: usize = 1024;
/// SEC-M1 — maximum number of entries in any array field. Bounds the O(n·m)
/// Jaccard / overlap work the matcher does over arrays.
const MAX_ARRAY_LEN: usize = 256;
/// SEC-M1 — maximum length of any single string entry inside an array.
const MAX_ITEM_LEN: usize = 512;

/// Maximum number of decimal places accepted on a geo coordinate.
///
/// Coordinates are exact decimals ([`BigDecimal`]), not `f64`, so the digit
/// count is no longer capped by the ~17 significant digits a binary float
/// could hold — a caller could otherwise post a latitude with thousands of
/// fraction digits and have every one stored. Ten places is roughly 10 µm
/// at the equator: far past any real positioning system, and well inside
/// what an `f64` used to carry, so nothing a client could previously send
/// is newly rejected.
const MAX_COORDINATE_SCALE: i64 = 10;

/// Push an error when a geo coordinate carries more than
/// [`MAX_COORDINATE_SCALE`] decimal places.
fn check_coordinate_scale(errors: &mut Vec<ValidationError>, field: &str, value: &BigDecimal) {
    if value.fractional_digit_count() > MAX_COORDINATE_SCALE {
        errors.push(ValidationError {
            field: field.into(),
            message: format!("Must not exceed {MAX_COORDINATE_SCALE} decimal places"),
        });
    }
}

/// SEC-M1: push an error when a scalar text `field` exceeds [`MAX_TEXT_LEN`].
fn cap_text(errors: &mut Vec<ValidationError>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("Must not exceed {MAX_TEXT_LEN} characters"),
        });
    }
}

/// [`cap_text`] for an optional field; a no-op when the value is absent.
fn cap_opt_text(errors: &mut Vec<ValidationError>, field: &str, value: Option<&String>) {
    if let Some(v) = value {
        cap_text(errors, field, v);
    }
}

/// SEC-M1: push an error when array `field` holds more than [`MAX_ARRAY_LEN`]
/// entries.
fn cap_array(errors: &mut Vec<ValidationError>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("Must not exceed {MAX_ARRAY_LEN} entries"),
        });
    }
}

/// SEC-M1: push an error when the `index`-th entry of array `field` exceeds
/// [`MAX_ITEM_LEN`]. The field path is indexed so the caller can point at the
/// offending entry.
fn cap_item(errors: &mut Vec<ValidationError>, field: &str, index: usize, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        errors.push(ValidationError {
            field: format!("{field}[{index}]"),
            message: format!("Must not exceed {MAX_ITEM_LEN} characters"),
        });
    }
}

/// SEC-M1: cap the cardinality of a string array and the length of each entry
/// — the common shape for the `Place` list fields.
fn cap_string_array(errors: &mut Vec<ValidationError>, field: &str, values: &[String]) {
    cap_array(errors, field, values.len());
    for (i, v) in values.iter().enumerate() {
        cap_item(errors, field, i, v);
    }
}

/// SEC-M1: cap the nested [`PostalAddress`](crate::models::address::PostalAddress)
/// text fields, dotted under `address.*`.
fn place_address_caps(errors: &mut Vec<ValidationError>, place: &Place) {
    if let Some(addr) = &place.address {
        cap_opt_text(
            errors,
            "address.street_address",
            addr.street_address.as_ref(),
        );
        cap_opt_text(
            errors,
            "address.address_locality",
            addr.address_locality.as_ref(),
        );
        cap_opt_text(
            errors,
            "address.address_region",
            addr.address_region.as_ref(),
        );
        cap_opt_text(
            errors,
            "address.address_country",
            addr.address_country.as_ref(),
        );
        cap_opt_text(errors, "address.postal_code", addr.postal_code.as_ref());
    }
}

/// SEC-M1: cap the `identifiers` array cardinality plus each identifier's
/// `value` string.
fn place_identifier_caps(errors: &mut Vec<ValidationError>, place: &Place) {
    cap_array(errors, "identifiers", place.identifiers.len());
    for (i, id) in place.identifiers.iter().enumerate() {
        cap_item(errors, "identifiers", i, &id.value);
    }
}

/// SEC-M1: cap the `amenity_features` array cardinality plus each feature's
/// `name` and optional `value`.
fn place_amenity_caps(errors: &mut Vec<ValidationError>, place: &Place) {
    cap_array(errors, "amenity_features", place.amenity_features.len());
    for (i, feature) in place.amenity_features.iter().enumerate() {
        cap_item(errors, "amenity_features.name", i, &feature.name);
        if let Some(v) = &feature.value {
            cap_item(errors, "amenity_features.value", i, v);
        }
    }
}

/// SEC-M1: apply the input-size caps to a [`Place`]'s scalar text fields,
/// string arrays, and struct-array cardinality. Split out of
/// [`validate_place`] to keep that function within the line budget.
///
/// Skipped (already bounded by a stricter existing rule, left untouched):
/// `global_location_number` (exactly 13 digits), and each
/// `opening_hours` entry's `opens`/`closes` (exactly 5 chars via
/// [`time_is_valid`]) — so only `opening_hours` cardinality is capped here.
/// Capacity carries no text and is not capped. Geo lat/lon/elevation are
/// exact decimals rather than text, and are bounded on *scale* by
/// [`check_coordinate_scale`] instead — an `f64` capped their digit count
/// implicitly, a `BigDecimal` does not. The `place_type` `Other(String)` enum payload is not a plain text
/// field and is left uncapped.
fn place_size_caps(errors: &mut Vec<ValidationError>, place: &Place) {
    cap_text(errors, "name", &place.name);
    cap_opt_text(errors, "alternate_name", place.alternate_name.as_ref());
    cap_opt_text(errors, "description", place.description.as_ref());
    cap_opt_text(errors, "telephone", place.telephone.as_ref());
    cap_opt_text(errors, "fax_number", place.fax_number.as_ref());
    cap_opt_text(errors, "url", place.url.as_ref());
    cap_opt_text(errors, "branch_code", place.branch_code.as_ref());

    cap_string_array(errors, "keywords", &place.keywords);

    place_address_caps(errors, place);
    place_identifier_caps(errors, place);
    place_amenity_caps(errors, place);

    cap_array(errors, "opening_hours", place.opening_hours.len());
}
/// Range- and scale-check a set of coordinates.
///
/// Extracted from [`validate_place`] so that function stays readable:
/// the coordinate field names carry their units now, which is worth the
/// length at every call site but pushes one long function over the
/// line limit.
fn validate_geo(errors: &mut Vec<ValidationError>, geo: &crate::models::geo::GeoCoordinates) {
    if !(BigDecimal::from(-90)..=BigDecimal::from(90)).contains(&geo.latitude_as_decimal_degrees) {
        errors.push(ValidationError {
            field: "geo.latitude_as_decimal_degrees".into(),
            message: format!(
                "Latitude must be between -90 and 90, got {}",
                geo.latitude_as_decimal_degrees
            ),
        });
    }
    if !(BigDecimal::from(-180)..=BigDecimal::from(180)).contains(&geo.longitude_as_decimal_degrees)
    {
        errors.push(ValidationError {
            field: "geo.longitude_as_decimal_degrees".into(),
            message: format!(
                "Longitude must be between -180 and 180, got {}",
                geo.longitude_as_decimal_degrees
            ),
        });
    }
    check_coordinate_scale(
        errors,
        "geo.latitude_as_decimal_degrees",
        &geo.latitude_as_decimal_degrees,
    );
    check_coordinate_scale(
        errors,
        "geo.longitude_as_decimal_degrees",
        &geo.longitude_as_decimal_degrees,
    );
    if let Some(elevation) = geo.elevation_as_decimal_metres.as_ref() {
        check_coordinate_scale(errors, "geo.elevation_as_decimal_metres", elevation);
    }
}

/// Validate a place, returning all validation errors.
///
/// Checks (each independent, all reported): non-empty name; latitude in
/// `[-90, 90]` and longitude in `[-180, 180]` when geo is present; a 13-digit
/// numeric GLN with a valid GS1 check digit; `http(s)://` URL scheme;
/// `+`-prefixed telephone; valid 24-hour `HH:MM` opening/closing times; and
/// that any present address carries at least a locality, postal code, or
/// country.
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
/// assert!(errors.iter().any(|e| e.field == "geo.latitude_as_decimal_degrees"));
/// ```
#[must_use]
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
        validate_geo(&mut errors, geo);
    }

    // GLN must be exactly 13 ASCII digits with a valid GS1 mod-10 check digit.
    if let Some(gln) = &place.global_location_number
        && !gln_is_valid(gln)
    {
        errors.push(ValidationError {
            field: "global_location_number".into(),
            message: "GLN must be exactly 13 digits with a valid GS1 check digit".into(),
        });
    }

    // URL must carry an http(s) scheme; we do not parse the full URL.
    if let Some(url) = &place.url
        && !url.starts_with("http://")
        && !url.starts_with("https://")
    {
        errors.push(ValidationError {
            field: "url".into(),
            message: "URL must start with http:// or https://".into(),
        });
    }

    // Telephone, when non-empty, must use the international `+` prefix.
    if let Some(tel) = &place.telephone
        && !tel.is_empty()
        && !tel.starts_with('+')
    {
        errors.push(ValidationError {
            field: "telephone".into(),
            message: "Telephone must start with + for international format".into(),
        });
    }

    // Opening hours: each window's `opens`/`closes` must be a valid 24-hour
    // `HH:MM` clock time. The field paths are indexed so the caller can point
    // at the offending entry.
    for (i, spec) in place.opening_hours.iter().enumerate() {
        if !time_is_valid(&spec.opens) {
            errors.push(ValidationError {
                field: format!("opening_hours[{i}].opens"),
                message: format!(
                    "Opening time must be a valid 24-hour HH:MM clock time, got {:?}",
                    spec.opens
                ),
            });
        }
        if !time_is_valid(&spec.closes) {
            errors.push(ValidationError {
                field: format!("opening_hours[{i}].closes"),
                message: format!(
                    "Closing time must be a valid 24-hour HH:MM clock time, got {:?}",
                    spec.closes
                ),
            });
        }
    }

    // A present address needs at least one locating field; a lone street line
    // is not enough to place it.
    if let Some(addr) = &place.address {
        let has_locality = addr
            .address_locality
            .as_ref()
            .is_some_and(|s| !s.is_empty());
        let has_postal = addr.postal_code.as_ref().is_some_and(|s| !s.is_empty());
        let has_country = addr.address_country.as_ref().is_some_and(|s| !s.is_empty());
        if !has_locality && !has_postal && !has_country {
            errors.push(ValidationError {
                field: "address".into(),
                message: "Address must have at least locality, postal code, or country".into(),
            });
        }
    }

    // A place cannot contain itself. This is the direct (0-hop) case of
    // the hierarchy-cycle rejection `spec/16-open-questions.md` OQ-2
    // already documents as "validation rejects on insert" — pure and
    // checkable with no DB access. The multi-hop case (A contains B,
    // B contains A) needs a DB round-trip and is a repository-level
    // check on create/update instead (T-16).
    if place.contained_in_place == Some(place.id) {
        errors.push(ValidationError {
            field: "contained_in_place".into(),
            message: "A place cannot contain itself".into(),
        });
    }

    // SEC-M1 input-size caps (additive; independent of the checks above).
    place_size_caps(&mut errors, place);

    errors
}

/// Validate a Global Location Number (GLN): exactly 13 ASCII digits whose
/// last digit is the correct GS1 mod-10 check digit.
///
/// The GS1 check-digit algorithm weights the 12 data digits right-to-left by
/// alternating 3, 1, 3, 1, …; the check digit is the value that brings the
/// weighted sum up to the next multiple of 10.
///
/// # Examples
///
/// ```
/// use place_service::validation::gln_is_valid;
///
/// assert!(gln_is_valid("0614141999996")); // valid GS1 check digit
/// assert!(!gln_is_valid("0614141999990")); // wrong check digit
/// assert!(!gln_is_valid("12345")); // wrong length
/// assert!(!gln_is_valid("061414199999A")); // non-digit
/// ```
#[must_use]
pub fn gln_is_valid(gln: &str) -> bool {
    if gln.len() != 13 {
        return false;
    }
    let mut digits = [0u32; 13];
    for (slot, ch) in digits.iter_mut().zip(gln.chars()) {
        match ch.to_digit(10) {
            Some(d) => *slot = d,
            None => return false,
        }
    }
    // GS1 mod-10: weight the 12 data digits right-to-left by 3, 1, 3, 1, …
    // (the rightmost data digit gets 3). Here `digits[..12]` excludes the
    // check digit, `.rev()` walks right-to-left, and even `i` (0, 2, …) maps
    // to the ×3 positions, odd `i` to the ×1 positions.
    let sum: u32 = digits[..12]
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| if i % 2 == 0 { d * 3 } else { d })
        .sum();
    // The check digit is whatever rounds the weighted sum up to the next
    // multiple of 10. The outer `% 10` maps the "sum already a multiple of
    // 10" case (10 − 0 = 10) back to a 0 check digit.
    let check = (10 - (sum % 10)) % 10;
    check == digits[12]
}

/// Validate a 24-hour `HH:MM` clock time: exactly five ASCII characters with a
/// `:` separator, hours in `00..=23`, and minutes in `00..=59`.
///
/// Used by [`validate_place`] to reject malformed
/// [`OpeningHoursSpecification`](crate::models::opening_hours::OpeningHoursSpecification)
/// times such as `"25:99"`, `"9am"`, or `""`. Schema.org stores opening hours
/// as plain strings, so without this check any text would be accepted.
///
/// # Examples
///
/// ```
/// use place_service::validation::time_is_valid;
///
/// assert!(time_is_valid("09:00"));
/// assert!(time_is_valid("23:59"));
/// assert!(time_is_valid("00:00"));
/// assert!(!time_is_valid("24:00")); // hour out of range
/// assert!(!time_is_valid("12:60")); // minute out of range
/// assert!(!time_is_valid("9:00")); // not zero-padded
/// assert!(!time_is_valid("0900")); // missing separator
/// assert!(!time_is_valid("")); // empty
/// ```
#[must_use]
pub fn time_is_valid(time: &str) -> bool {
    // Require the canonical `HH:MM` shape: 2 ASCII digits, colon, 2 ASCII
    // digits. Checking the digits explicitly avoids `parse` quirks such as
    // accepting a leading `+` (e.g. "+9").
    let bytes = time.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
        || !bytes[4].is_ascii_digit()
    {
        return false;
    }
    let hours = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
    let minutes = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
    hours < 24 && minutes < 60
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
    use crate::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};

    /// A minimal place (name only) passes validation.
    #[test]
    fn test_valid_place() {
        let place = Place::new("Central Park");
        let errors = validate_place(&place);
        assert!(errors.is_empty(), "Errors: {errors:?}");
    }

    /// A place cannot list itself as its own container (T-16 direct
    /// case; the multi-hop case is a repository-level check, since it
    /// needs a DB round-trip — see `db::SeaOrmPlaceRepository::
    /// ancestor_chain_contains`).
    #[test]
    fn test_self_referencing_contained_in_place_is_rejected() {
        let mut place = Place::new("Self-contained");
        place.contained_in_place = Some(place.id);
        let errors = validate_place(&place);
        assert_eq!(errors.len(), 1, "Errors: {errors:?}");
        assert_eq!(errors[0].field, "contained_in_place");
    }

    /// Containing a *different* place is unaffected by the self-reference
    /// check.
    #[test]
    fn test_contained_in_a_different_place_is_not_rejected() {
        let mut place = Place::new("Nested");
        place.contained_in_place = Some(uuid::Uuid::new_v4());
        let errors = validate_place(&place);
        assert!(
            errors.iter().all(|e| e.field != "contained_in_place"),
            "Errors: {errors:?}"
        );
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
        assert!(
            errors
                .iter()
                .any(|e| e.field == "geo.latitude_as_decimal_degrees")
        );
    }

    /// Longitude below -180 is flagged.
    #[test]
    fn test_invalid_longitude() {
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates::new(0.0, -181.0));
        let errors = validate_place(&place);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "geo.longitude_as_decimal_degrees")
        );
    }

    /// A coordinate a hair outside the range is flagged.
    ///
    /// Exact decimals make this testable: as an `f64`, `90.0000000001`
    /// and values near it are subject to representation error, so a
    /// "just outside" case could round back into range.
    #[test]
    fn coordinates_just_outside_the_range_are_flagged() {
        for (lat, lon, field) in [
            ("90.0000000001", "0", "geo.latitude_as_decimal_degrees"),
            ("-90.0000000001", "0", "geo.latitude_as_decimal_degrees"),
            ("0", "180.0000000001", "geo.longitude_as_decimal_degrees"),
            ("0", "-180.0000000001", "geo.longitude_as_decimal_degrees"),
        ] {
            let mut place = Place::new("Test");
            place.geo = Some(GeoCoordinates {
                latitude_as_decimal_degrees: lat.parse().unwrap(),
                longitude_as_decimal_degrees: lon.parse().unwrap(),
                elevation_as_decimal_metres: None,
            });
            let errors = validate_place(&place);
            assert!(
                errors.iter().any(|e| e.field == field),
                "{lat},{lon} should fail on {field}, got {errors:?}"
            );
        }
    }

    /// A coordinate carrying more than [`MAX_COORDINATE_SCALE`] decimal
    /// places is rejected.
    ///
    /// An `f64` bounded the digit count implicitly; an exact decimal does
    /// not, so without this a caller could post a latitude with thousands
    /// of fraction digits and have every one stored.
    #[test]
    fn coordinate_scale_is_capped() {
        let places = usize::try_from(MAX_COORDINATE_SCALE).unwrap();
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates {
            latitude_as_decimal_degrees: format!("40.{}", "1".repeat(places + 1)).parse().unwrap(),
            longitude_as_decimal_degrees: "0".parse().unwrap(),
            elevation_as_decimal_metres: Some(
                format!("10.{}", "1".repeat(places + 1)).parse().unwrap(),
            ),
        });
        let errors = validate_place(&place);
        for field in [
            "geo.latitude_as_decimal_degrees",
            "geo.elevation_as_decimal_metres",
        ] {
            assert!(
                errors
                    .iter()
                    .any(|e| e.field == field && e.message.contains("decimal places")),
                "{field} should exceed the scale cap, got {errors:?}"
            );
        }

        // Exactly at the cap passes; only strictly-over is rejected,
        // matching how the text and array caps behave.
        let mut place = Place::new("Test");
        place.geo = Some(GeoCoordinates {
            latitude_as_decimal_degrees: format!("40.{}", "1".repeat(places)).parse().unwrap(),
            longitude_as_decimal_degrees: "0".parse().unwrap(),
            elevation_as_decimal_metres: None,
        });
        let errors = validate_place(&place);
        assert!(
            !errors.iter().any(|e| e.message.contains("decimal places")),
            "a coordinate at the cap should pass, got {errors:?}"
        );
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

    /// A well-formed 13-digit GLN with a valid GS1 check digit passes.
    #[test]
    fn test_valid_gln() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("0614141999996".into());
        let errors = validate_place(&place);
        assert!(errors.is_empty(), "Errors: {errors:?}");
    }

    /// A 13-digit GLN with a wrong GS1 check digit is rejected. Same first 12
    /// digits as the valid case above, last digit corrupted 6 → 0.
    #[test]
    fn test_invalid_gln_check_digit() {
        let mut place = Place::new("Test");
        place.global_location_number = Some("0614141999990".into());
        let errors = validate_place(&place);
        assert!(errors.iter().any(|e| e.field == "global_location_number"));
    }

    /// `gln_is_valid` accepts real GS1 GLNs and rejects length / digit /
    /// check-digit failures.
    #[test]
    fn test_gln_is_valid_helper() {
        // Valid: correct length, all digits, correct GS1 check digit.
        assert!(gln_is_valid("0614141999996"));
        assert!(gln_is_valid("4006381333931"));
        // Wrong check digit (last digit corrupted).
        assert!(!gln_is_valid("0614141999990"));
        assert!(!gln_is_valid("4006381333930"));
        // Wrong length.
        assert!(!gln_is_valid("061414199999"));
        assert!(!gln_is_valid("06141419999966"));
        // Non-digit character.
        assert!(!gln_is_valid("061414199999A"));
    }

    /// Well-formed opening hours pass validation.
    #[test]
    fn test_valid_opening_hours() {
        let mut place = Place::new("Test");
        place.opening_hours = vec![
            OpeningHoursSpecification::new(DayOfWeek::Monday, "09:00", "17:00"),
            OpeningHoursSpecification::new(DayOfWeek::Saturday, "00:00", "23:59"),
        ];
        let errors = validate_place(&place);
        assert!(errors.is_empty(), "Errors: {errors:?}");
    }

    /// An out-of-range opening time is rejected with an indexed field path.
    #[test]
    fn test_invalid_opening_time() {
        let mut place = Place::new("Test");
        place.opening_hours = vec![OpeningHoursSpecification::new(
            DayOfWeek::Monday,
            "25:99",
            "17:00",
        )];
        let errors = validate_place(&place);
        assert!(
            errors.iter().any(|e| e.field == "opening_hours[0].opens"),
            "Errors: {errors:?}"
        );
    }

    /// A malformed closing time is rejected; the index points at the entry.
    #[test]
    fn test_invalid_closing_time() {
        let mut place = Place::new("Test");
        place.opening_hours = vec![
            OpeningHoursSpecification::new(DayOfWeek::Monday, "09:00", "17:00"),
            OpeningHoursSpecification::new(DayOfWeek::Tuesday, "09:00", "5pm"),
        ];
        let errors = validate_place(&place);
        assert!(
            errors.iter().any(|e| e.field == "opening_hours[1].closes"),
            "Errors: {errors:?}"
        );
    }

    /// `time_is_valid` accepts canonical 24-hour times and rejects the common
    /// malformed shapes.
    #[test]
    fn test_time_is_valid_helper() {
        // Valid boundaries.
        assert!(time_is_valid("00:00"));
        assert!(time_is_valid("23:59"));
        assert!(time_is_valid("09:05"));
        // Hour / minute out of range.
        assert!(!time_is_valid("24:00"));
        assert!(!time_is_valid("12:60"));
        assert!(!time_is_valid("99:99"));
        // Wrong shape.
        assert!(!time_is_valid("9:00")); // not zero-padded
        assert!(!time_is_valid("0900")); // missing separator
        assert!(!time_is_valid("09:00:00")); // seconds not allowed
        assert!(!time_is_valid("9am")); // not numeric
        assert!(!time_is_valid("")); // empty
        assert!(!time_is_valid("+9:00")); // sign rejected
        assert!(!time_is_valid(" 9:00")); // leading space rejected
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

    /// SEC-M1: an oversized scalar text field (here `description`) is capped.
    #[test]
    fn test_size_cap_oversized_text() {
        let mut place = Place::new("Test");
        place.description = Some("x".repeat(MAX_TEXT_LEN + 1));
        let errors = validate_place(&place);
        assert!(
            errors.iter().any(|e| e.field == "description"),
            "Errors: {errors:?}"
        );
    }

    /// SEC-M1: an over-long array (here `keywords`) is flagged on cardinality.
    #[test]
    fn test_size_cap_array_cardinality() {
        let mut place = Place::new("Test");
        place.keywords = vec!["k".to_string(); MAX_ARRAY_LEN + 1];
        let errors = validate_place(&place);
        assert!(
            errors.iter().any(|e| e.field == "keywords"),
            "Errors: {errors:?}"
        );
    }

    /// SEC-M1: an oversized array entry is flagged with an indexed field path.
    #[test]
    fn test_size_cap_oversized_array_entry() {
        let mut place = Place::new("Test");
        place.keywords = vec!["ok".to_string(), "y".repeat(MAX_ITEM_LEN + 1)];
        let errors = validate_place(&place);
        assert!(
            errors.iter().any(|e| e.field == "keywords[1]"),
            "Errors: {errors:?}"
        );
    }

    /// SEC-M1: a large-but-within-caps record produces no cap errors.
    #[test]
    fn test_size_cap_within_bounds_ok() {
        let mut place = Place::new(&"x".repeat(MAX_TEXT_LEN));
        place.description = Some("y".repeat(MAX_TEXT_LEN));
        place.branch_code = Some("z".repeat(MAX_TEXT_LEN));
        place.keywords = vec!["k".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN];
        place.address = Some(PostalAddress {
            street_address: Some("s".repeat(MAX_TEXT_LEN)),
            address_locality: Some("Town".into()),
            address_region: None,
            address_country: None,
            postal_code: None,
        });
        let errors = validate_place(&place);
        assert!(errors.is_empty(), "Errors: {errors:?}");
    }
}
