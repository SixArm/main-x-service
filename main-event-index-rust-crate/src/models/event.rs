//! Event model definition

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use super::{Address, ContactPoint, Gender, Identifier, IdentityDocument, EmergencyContact};

/// Event resource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Event {
    /// Unique event identifier
    pub id: Uuid,

    /// Event identifiers (MRN, SSN, etc.)
    pub identifiers: Vec<Identifier>,

    /// Active status
    pub active: bool,

    /// Event name
    pub name: HumanName,

    /// Additional names
    pub additional_names: Vec<HumanName>,

    /// Telecom contacts
    pub telecom: Vec<ContactPoint>,

    /// Gender
    pub gender: Gender,

    /// Birth date
    pub birth_date: Option<NaiveDate>,

    /// Tax ID number (CPF, SSN, TIN, etc.)
    #[serde(default)]
    pub tax_id: Option<String>,

    /// Identity documents (passport, birth certificate, etc.)
    #[serde(default)]
    pub documents: Vec<IdentityDocument>,

    /// Emergency contacts
    #[serde(default)]
    pub emergency_contacts: Vec<EmergencyContact>,

    /// Deceased indicator
    pub deceased: bool,

    /// Deceased date/time
    pub deceased_datetime: Option<DateTime<Utc>>,

    /// Addresses
    pub addresses: Vec<Address>,

    /// Marital status
    pub marital_status: Option<String>,

    /// Multiple birth indicator
    pub multiple_birth: Option<bool>,

    /// Photo attachments
    pub photo: Vec<String>,

    /// Managing organization
    pub managing_organization: Option<Uuid>,

    /// Links to other event records
    pub links: Vec<EventLink>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Human name representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HumanName {
    pub use_type: Option<NameUse>,
    pub family: String,
    pub given: Vec<String>,
    pub prefix: Vec<String>,
    pub suffix: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NameUse {
    Usual,
    Official,
    Temp,
    Nickname,
    Anonymous,
    Old,
    Maiden,
}

/// Event link to another event record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventLink {
    pub other_event_id: Uuid,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// The event resource containing this link is replaced by the linked event
    ReplacedBy,
    /// The event resource containing this link replaces the linked event
    Replaces,
    /// The event resource containing this link refers to the same event as the linked event
    Refer,
    /// The event resource containing this link is semantically referring to the linked event
    Seealso,
}

impl Event {
    /// Create a new event
    pub fn new(name: HumanName, gender: Gender) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            identifiers: Vec::new(),
            active: true,
            name,
            additional_names: Vec::new(),
            telecom: Vec::new(),
            gender,
            birth_date: None,
            tax_id: None,
            documents: Vec::new(),
            emergency_contacts: Vec::new(),
            deceased: false,
            deceased_datetime: None,
            addresses: Vec::new(),
            marital_status: None,
            multiple_birth: None,
            photo: Vec::new(),
            managing_organization: None,
            links: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get full name as a string
    pub fn full_name(&self) -> String {
        let given = self.name.given.join(" ");
        format!("{} {}", given, self.name.family)
    }

    /// Get tax ID, falling back to TAX-type identifier if tax_id field is empty
    pub fn effective_tax_id(&self) -> Option<&str> {
        if let Some(ref tid) = self.tax_id {
            return Some(tid.as_str());
        }
        self.identifiers.iter()
            .find(|id| id.identifier_type == super::IdentifierType::TAX)
            .map(|id| id.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Gender;

    #[test]
    fn test_event_new_defaults() {
        let name = HumanName {
            use_type: None,
            family: "Doe".into(),
            given: vec!["Jane".into()],
            prefix: vec![],
            suffix: vec![],
        };
        let event = Event::new(name, Gender::Female);

        assert!(event.active);
        assert!(!event.deceased);
        assert_eq!(event.gender, Gender::Female);
        assert_eq!(event.name.family, "Doe");
        assert_eq!(event.name.given, vec!["Jane".to_string()]);
        assert!(event.identifiers.is_empty());
        assert!(event.addresses.is_empty());
        assert!(event.telecom.is_empty());
        assert!(event.documents.is_empty());
        assert!(event.emergency_contacts.is_empty());
        assert!(event.links.is_empty());
        assert!(event.birth_date.is_none());
        assert!(event.tax_id.is_none());
        assert!(event.marital_status.is_none());
        assert!(event.managing_organization.is_none());
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let name = HumanName {
            use_type: Some(NameUse::Official),
            family: "Smith".into(),
            given: vec!["John".into(), "Michael".into()],
            prefix: vec!["Dr.".into()],
            suffix: vec!["Jr.".into()],
        };
        let mut event = Event::new(name, Gender::Male);
        event.birth_date = Some(chrono::NaiveDate::from_ymd_opt(1985, 3, 20).unwrap());
        event.tax_id = Some("123-45-6789".into());

        let json = serde_json::to_string(&event).expect("Serialization should succeed");
        let deserialized: Event = serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.name.family, "Smith");
        assert_eq!(deserialized.name.given.len(), 2);
        assert_eq!(deserialized.gender, Gender::Male);
        assert_eq!(deserialized.tax_id.as_deref(), Some("123-45-6789"));
        assert_eq!(deserialized.birth_date, event.birth_date);
    }

    #[test]
    fn test_human_name_display() {
        let name = HumanName {
            use_type: None,
            family: "Garcia".into(),
            given: vec!["Maria".into(), "Elena".into()],
            prefix: vec![],
            suffix: vec![],
        };
        let event = Event::new(name, Gender::Female);
        let full = event.full_name();
        assert_eq!(full, "Maria Elena Garcia");
    }

    #[test]
    fn test_gender_variants() {
        // Test all gender variants serialize/deserialize correctly
        let genders = vec![Gender::Male, Gender::Female, Gender::Other, Gender::Unknown];
        for g in genders {
            let json = serde_json::to_string(&g).expect("Gender serialization");
            let deser: Gender = serde_json::from_str(&json).expect("Gender deserialization");
            assert_eq!(deser, g);
        }
    }
}
