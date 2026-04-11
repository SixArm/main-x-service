use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::address::PostalAddress;
use super::amenity::AmenityFeature;
use super::geo::GeoCoordinates;
use super::identifier::ThingIdentifier;
use super::opening_hours::OpeningHoursSpecification;
use super::thing_type::ThingType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thing {
    pub id: Uuid,
    pub name: String,
    pub alternate_name: Option<String>,
    pub description: Option<String>,
    pub thing_type: Option<ThingType>,
    pub address: Option<PostalAddress>,
    pub geo: Option<GeoCoordinates>,
    pub telephone: Option<String>,
    pub fax_number: Option<String>,
    pub url: Option<String>,
    pub global_location_number: Option<String>,
    pub branch_code: Option<String>,
    pub contained_in_thing: Option<Uuid>,
    pub keywords: Vec<String>,
    pub identifiers: Vec<ThingIdentifier>,
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

impl Thing {
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            alternate_name: None,
            description: None,
            thing_type: None,
            address: None,
            geo: None,
            telephone: None,
            fax_number: None,
            url: None,
            global_location_number: None,
            branch_code: None,
            contained_in_thing: None,
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
    fn test_thing_new() {
        let thing = Thing::new("Central Park");
        assert_eq!(thing.name, "Central Park");
        assert!(thing.id != uuid::Uuid::nil());
        assert!(!thing.is_deleted);
    }

    #[test]
    fn test_thing_default_fields() {
        let thing = Thing::new("Test");
        assert!(thing.alternate_name.is_none());
        assert!(thing.description.is_none());
        assert!(thing.address.is_none());
        assert!(thing.geo.is_none());
        assert!(thing.thing_type.is_none());
        assert!(thing.telephone.is_none());
        assert!(thing.url.is_none());
        assert!(thing.identifiers.is_empty());
        assert!(thing.amenity_features.is_empty());
        assert!(!thing.is_deleted);
    }

    #[test]
    fn test_thing_with_address() {
        let addr = PostalAddress {
            street_address: Some("14 E 60th St".into()),
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: Some("10022".into()),
        };
        let mut thing = Thing::new("Central Park");
        thing.address = Some(addr);
        assert_eq!(thing.address.as_ref().unwrap().address_locality.as_deref(), Some("New York"));
    }

    #[test]
    fn test_thing_with_geo() {
        let geo = GeoCoordinates {
            latitude: 40.7829,
            longitude: -73.9654,
            elevation: None,
        };
        let mut thing = Thing::new("Central Park");
        thing.geo = Some(geo);
        assert!((thing.geo.as_ref().unwrap().latitude - 40.7829).abs() < f64::EPSILON);
    }

    #[test]
    fn test_thing_serialization_roundtrip() {
        let mut thing = Thing::new("Test Thing");
        thing.description = Some("A test".into());
        let json = serde_json::to_string(&thing).unwrap();
        let deserialized: Thing = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Thing");
        assert_eq!(deserialized.description.as_deref(), Some("A test"));
        assert_eq!(deserialized.id, thing.id);
    }

    #[test]
    fn test_thing_soft_delete() {
        let mut thing = Thing::new("To Delete");
        assert!(!thing.is_deleted);
        thing.soft_delete();
        assert!(thing.is_deleted);
        assert!(thing.deleted_at.is_some());
    }
}
