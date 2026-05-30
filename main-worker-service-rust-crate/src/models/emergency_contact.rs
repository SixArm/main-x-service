//! Emergency contact model

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Address, ContactPoint};

/// Emergency contact for a worker
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmergencyContact {
    /// Contact's full name
    pub name: String,

    /// Relationship to worker (e.g., "spouse", "parent", "sibling", "friend")
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
