//! The central [`Place`] aggregate — a physical place identity, modeled on
//! [schema.org/Place](https://schema.org/Place).
//!
//! `Place` is the record every other layer revolves around: matching scores
//! two of them, validation checks one, privacy masks one, and the API tier
//! persists/serves them. It carries registry bookkeeping (a stable
//! [`id`](Place::id), audit timestamps, and a [soft-delete](Place::soft_delete)
//! flag) alongside the schema.org descriptive fields.
//!
//! # Examples
//!
//! ```
//! use place_service::models::place::Place;
//!
//! let mut p = Place::new("Central Park");
//! assert!(!p.is_deleted);
//! p.soft_delete();
//! assert!(p.is_deleted);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::address::PostalAddress;
use super::amenity::AmenityFeature;
use super::geo::GeoCoordinates;
use super::identifier::PlaceIdentifier;
use super::opening_hours::OpeningHoursSpecification;
use super::place_type::PlaceType;

/// A physical place identity in the registry.
///
/// Most descriptive fields are `Option`/`Vec` because real-world source data
/// is sparse: a place might be known only by name, or carry a full address,
/// coordinates, contact details, and a list of external identifiers. The
/// only always-present descriptive field is [`name`](Self::name).
///
/// Records are never hard-deleted; [`soft_delete`](Self::soft_delete) flips
/// [`is_deleted`](Self::is_deleted) and stamps [`deleted_at`](Self::deleted_at)
/// so audit history and GDPR exports stay complete.
///
/// # Examples
///
/// ```
/// use place_service::models::place::Place;
/// use place_service::models::place_type::PlaceType;
///
/// let mut p = Place::new("Eiffel Tower");
/// p.place_type = Some(PlaceType::CivicStructure);
/// p.keywords = vec!["landmark".into(), "tourism".into()];
/// assert_eq!(p.name, "Eiffel Tower");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    /// Stable, system-generated UUID v4 primary key. Assigned once at
    /// construction and preserved across updates, merges, and serialization.
    pub id: Uuid,
    /// Primary name. The only required descriptive field; everything else
    /// may be absent. Validation rejects a blank/whitespace name.
    pub name: String,
    /// An alias or alternate spelling (schema.org `alternateName`).
    pub alternate_name: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// Classification (park, hospital, airport, …). See [`PlaceType`].
    pub place_type: Option<PlaceType>,
    /// Structured postal address. See [`PostalAddress`].
    pub address: Option<PostalAddress>,
    /// WGS 84 latitude/longitude (+ optional elevation). See [`GeoCoordinates`].
    pub geo: Option<GeoCoordinates>,
    /// Telephone number; validation expects E.164-style `+`-prefixed format.
    pub telephone: Option<String>,
    /// Fax number.
    pub fax_number: Option<String>,
    /// Website URL; validation requires an `http://` or `https://` scheme.
    pub url: Option<String>,
    /// GS1 Global Location Number — 13 digits. A shared GLN is a
    /// deterministic duplicate signal that short-circuits matching to 1.0.
    pub global_location_number: Option<String>,
    /// Operator-defined branch / site code.
    pub branch_code: Option<String>,
    /// Parent place in a `containedInPlace` hierarchy (e.g. a park inside a
    /// city). Holds the parent's [`id`](Self::id), not the parent itself.
    pub contained_in_place: Option<Uuid>,
    /// Free-form tags for search and categorization.
    pub keywords: Vec<String>,
    /// External identifiers (FIPS, GNIS, OSM, custom, …). See [`PlaceIdentifier`].
    pub identifiers: Vec<PlaceIdentifier>,
    /// Amenity features (WiFi, parking, …). See [`AmenityFeature`].
    pub amenity_features: Vec<AmenityFeature>,
    /// Per-day opening hours. See [`OpeningHoursSpecification`].
    pub opening_hours: Vec<OpeningHoursSpecification>,
    /// Whether the place can be accessed free of charge.
    pub is_accessible_for_free: Option<bool>,
    /// Whether the place is open to the general public.
    pub public_access: Option<bool>,
    /// Whether smoking is permitted.
    pub smoking_allowed: Option<bool>,
    /// Maximum number of people the place can hold.
    pub maximum_attendee_capacity: Option<u32>,
    /// Soft-delete flag. `true` means logically removed but retained for
    /// audit and compliance. See [`soft_delete`](Self::soft_delete).
    pub is_deleted: bool,
    /// When the record was soft-deleted, if it has been. Paired with
    /// [`is_deleted`](Self::is_deleted).
    pub deleted_at: Option<DateTime<Utc>>,
    /// Creation timestamp, set once at construction.
    pub created_at: DateTime<Utc>,
    /// Last-modification timestamp. Initialized equal to
    /// [`created_at`](Self::created_at).
    pub updated_at: DateTime<Utc>,
}

impl Place {
    /// Constructs a new place with the given name and sensible defaults: a
    /// fresh UUID, all optional fields empty, not deleted, and
    /// `created_at == updated_at == now`.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::place::Place;
    ///
    /// let p = Place::new("Central Park");
    /// assert_eq!(p.name, "Central Park");
    /// assert!(p.geo.is_none());
    /// assert_eq!(p.created_at, p.updated_at);
    /// ```
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            alternate_name: None,
            description: None,
            place_type: None,
            address: None,
            geo: None,
            telephone: None,
            fax_number: None,
            url: None,
            global_location_number: None,
            branch_code: None,
            contained_in_place: None,
            keywords: Vec::new(),
            identifiers: Vec::new(),
            amenity_features: Vec::new(),
            opening_hours: Vec::new(),
            is_accessible_for_free: None,
            public_access: None,
            smoking_allowed: None,
            maximum_attendee_capacity: None,
            is_deleted: false,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Logically removes the place: sets [`is_deleted`](Self::is_deleted) and
    /// stamps [`deleted_at`](Self::deleted_at) with the current time. The
    /// record is retained so audit trails and GDPR exports remain complete.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::place::Place;
    ///
    /// let mut p = Place::new("Closed Site");
    /// p.soft_delete();
    /// assert!(p.is_deleted);
    /// assert!(p.deleted_at.is_some());
    /// ```
    pub fn soft_delete(&mut self) {
        self.is_deleted = true;
        self.deleted_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new` sets the name, a non-nil id, and a not-deleted record.
    #[test]
    fn test_place_new() {
        let place = Place::new("Central Park");
        assert_eq!(place.name, "Central Park");
        assert!(place.id != uuid::Uuid::nil());
        assert!(!place.is_deleted);
    }

    /// A freshly-constructed place leaves every optional field unset.
    #[test]
    fn test_place_default_fields() {
        let place = Place::new("Test");
        assert!(place.alternate_name.is_none());
        assert!(place.description.is_none());
        assert!(place.address.is_none());
        assert!(place.geo.is_none());
        assert!(place.place_type.is_none());
        assert!(place.telephone.is_none());
        assert!(place.url.is_none());
        assert!(place.identifiers.is_empty());
        assert!(place.amenity_features.is_empty());
        assert!(!place.is_deleted);
    }

    /// An address can be attached and its fields read back.
    #[test]
    fn test_place_with_address() {
        let addr = PostalAddress {
            street_address: Some("14 E 60th St".into()),
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: Some("10022".into()),
        };
        let mut place = Place::new("Central Park");
        place.address = Some(addr);
        assert_eq!(place.address.as_ref().unwrap().address_locality.as_deref(), Some("New York"));
    }

    /// Coordinates can be attached and read back at full precision.
    #[test]
    fn test_place_with_geo() {
        let geo = GeoCoordinates {
            latitude: 40.7829,
            longitude: -73.9654,
            elevation: None,
        };
        let mut place = Place::new("Central Park");
        place.geo = Some(geo);
        assert!((place.geo.as_ref().unwrap().latitude - 40.7829).abs() < f64::EPSILON);
    }

    /// A place survives a JSON serialization round-trip, id included.
    #[test]
    fn test_place_serialization_roundtrip() {
        let mut place = Place::new("Test Place");
        place.description = Some("A test".into());
        let json = serde_json::to_string(&place).unwrap();
        let deserialized: Place = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Place");
        assert_eq!(deserialized.description.as_deref(), Some("A test"));
        assert_eq!(deserialized.id, place.id);
    }

    /// `soft_delete` flips the flag and records the deletion timestamp.
    #[test]
    fn test_place_soft_delete() {
        let mut place = Place::new("To Delete");
        assert!(!place.is_deleted);
        place.soft_delete();
        assert!(place.is_deleted);
        assert!(place.deleted_at.is_some());
    }
}
