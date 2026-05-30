use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::address::PostalAddress;
use super::amenity::AmenityFeature;
use super::geo::GeoCoordinates;
use super::identifier::PlaceIdentifier;
use super::opening_hours::OpeningHoursSpecification;
use super::place_type::PlaceType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: Uuid,
    pub name: String,
    pub alternate_name: Option<String>,
    pub description: Option<String>,
    pub place_type: Option<PlaceType>,
    pub address: Option<PostalAddress>,
    pub geo: Option<GeoCoordinates>,
    pub telephone: Option<String>,
    pub fax_number: Option<String>,
    pub url: Option<String>,
    pub global_location_number: Option<String>,
    pub branch_code: Option<String>,
    pub contained_in_place: Option<Uuid>,
    pub keywords: Vec<String>,
    pub identifiers: Vec<PlaceIdentifier>,
    pub amenity_features: Vec<AmenityFeature>,
    pub opening_hours: Vec<OpeningHoursSpecification>,
    pub is_accessible_for_free: Option<bool>,
    pub public_access: Option<bool>,
    pub smoking_allowed: Option<bool>,
    pub maximum_attendee_capacity: Option<u32>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Place {
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

    pub fn soft_delete(&mut self) {
        self.is_deleted = true;
        self.deleted_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_new() {
        let place = Place::new("Central Park");
        assert_eq!(place.name, "Central Park");
        assert!(place.id != uuid::Uuid::nil());
        assert!(!place.is_deleted);
    }

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

    #[test]
    fn test_place_soft_delete() {
        let mut place = Place::new("To Delete");
        assert!(!place.is_deleted);
        place.soft_delete();
        assert!(place.is_deleted);
        assert!(place.deleted_at.is_some());
    }
}
