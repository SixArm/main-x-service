//! The [`EmergencyContact`] model — a next-of-kin / point-of-contact.
//!
//! Each person may carry zero or more emergency contacts. A contact
//! reuses the shared [`crate::models::ContactPoint`] and
//! [`crate::models::Address`] value objects for its telecom and postal
//! details, and the `is_primary` flag marks the one to reach first.
//!
//! # Examples
//!
//! ```
//! use person_service::models::{EmergencyContact, ContactPoint, ContactPointSystem};
//!
//! let contact = EmergencyContact {
//!     name: "Jane Doe".to_string(),
//!     relationship: "spouse".to_string(),
//!     telecom: vec![ContactPoint {
//!         system: ContactPointSystem::Phone,
//!         value: "+15551234567".to_string(),
//!         use_type: None,
//!     }],
//!     address: None,
//!     is_primary: true,
//! };
//! assert!(contact.is_primary);
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Address, ContactPoint};

/// Emergency contact for a person
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmergencyContact {
    /// Contact's full name
    pub name: String,

    /// Relationship to person (e.g., "spouse", "parent", "sibling", "friend")
    pub relationship: String,

    /// Contact phone numbers, emails, etc.
    pub telecom: Vec<ContactPoint>,

    /// Contact address
    pub address: Option<Address>,

    /// Whether this is the primary emergency contact
    pub is_primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ContactPointSystem;

    /// An `EmergencyContact` constructed inline exposes its fields as set.
    #[test]
    fn test_emergency_contact_creation() {
        let contact = EmergencyContact {
            name: "Jane Doe".into(),
            relationship: "spouse".into(),
            telecom: vec![ContactPoint {
                system: ContactPointSystem::Phone,
                value: "+15551234567".into(),
                use_type: None,
            }],
            address: None,
            is_primary: true,
        };

        assert_eq!(contact.name, "Jane Doe");
        assert_eq!(contact.relationship, "spouse");
        assert!(contact.is_primary);
        assert_eq!(contact.telecom.len(), 1);
        assert!(contact.address.is_none());
    }

    /// An `EmergencyContact` (including a nested address) survives a
    /// JSON serialize → deserialize round-trip.
    #[test]
    fn test_emergency_contact_serialization() {
        let contact = EmergencyContact {
            name: "Bob Smith".into(),
            relationship: "parent".into(),
            telecom: vec![],
            address: Some(Address {
                use_type: None,
                line1: Some("456 Elm St".into()),
                line2: None,
                city: Some("Anytown".into()),
                state: Some("CA".into()),
                postal_code: Some("90210".into()),
                country: Some("US".into()),
            }),
            is_primary: false,
        };

        let json = serde_json::to_string(&contact).expect("Serialization should succeed");
        let deser: EmergencyContact = serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deser.name, "Bob Smith");
        assert_eq!(deser.relationship, "parent");
        assert!(!deser.is_primary);
        assert!(deser.address.is_some());
        assert_eq!(deser.address.as_ref().unwrap().city.as_deref(), Some("Anytown"));
    }
}
