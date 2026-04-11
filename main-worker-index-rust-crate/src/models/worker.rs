//! Worker model definition

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use super::{Address, ContactPoint, Gender, Identifier, IdentityDocument, EmergencyContact};

/// Worker resource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Worker {
    /// Unique worker identifier
    pub id: Uuid,

    /// Worker identifiers (MRN, SSN, etc.)
    pub identifiers: Vec<Identifier>,

    /// Active status
    pub active: bool,

    /// Worker name
    pub name: HumanName,

    /// Additional names
    pub additional_names: Vec<HumanName>,

    /// Telecom contacts
    pub telecom: Vec<ContactPoint>,

    /// Gender
    pub gender: Gender,

    /// Worker type (doctor, nurse, carer, staff, employee, manager, supervisor, consultant, other)
    #[serde(default)]
    pub worker_type: Option<WorkerType>,

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

    /// Links to other worker records
    pub links: Vec<WorkerLink>,

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

/// Worker type classification
///
/// Categorizes workers by their role within the healthcare system.
/// Workers are medical providers (doctors, nurses, carers, staff, etc.),
/// not medical consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkerType {
    /// Licensed physician / doctor
    Doctor,
    /// Registered nurse
    Nurse,
    /// Care worker / carer
    Carer,
    /// General staff member
    Staff,
    /// Employed worker
    Employee,
    /// Manager
    Manager,
    /// Supervisor
    Supervisor,
    /// External consultant
    Consultant,
    /// Other worker type
    Other,
}

impl std::fmt::Display for WorkerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerType::Doctor => write!(f, "doctor"),
            WorkerType::Nurse => write!(f, "nurse"),
            WorkerType::Carer => write!(f, "carer"),
            WorkerType::Staff => write!(f, "staff"),
            WorkerType::Employee => write!(f, "employee"),
            WorkerType::Manager => write!(f, "manager"),
            WorkerType::Supervisor => write!(f, "supervisor"),
            WorkerType::Consultant => write!(f, "consultant"),
            WorkerType::Other => write!(f, "other"),
        }
    }
}

/// Worker link to another worker record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkerLink {
    pub other_worker_id: Uuid,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// The worker resource containing this link is replaced by the linked worker
    ReplacedBy,
    /// The worker resource containing this link replaces the linked worker
    Replaces,
    /// The worker resource containing this link refers to the same worker as the linked worker
    Refer,
    /// The worker resource containing this link is semantically referring to the linked worker
    Seealso,
}

impl Worker {
    /// Create a new worker
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
            worker_type: None,
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
    fn test_worker_new_defaults() {
        let name = HumanName {
            use_type: None,
            family: "Doe".into(),
            given: vec!["Jane".into()],
            prefix: vec![],
            suffix: vec![],
        };
        let worker = Worker::new(name, Gender::Female);

        assert!(worker.active);
        assert!(!worker.deceased);
        assert_eq!(worker.gender, Gender::Female);
        assert_eq!(worker.name.family, "Doe");
        assert_eq!(worker.name.given, vec!["Jane".to_string()]);
        assert!(worker.identifiers.is_empty());
        assert!(worker.addresses.is_empty());
        assert!(worker.telecom.is_empty());
        assert!(worker.documents.is_empty());
        assert!(worker.emergency_contacts.is_empty());
        assert!(worker.links.is_empty());
        assert!(worker.worker_type.is_none());
        assert!(worker.birth_date.is_none());
        assert!(worker.tax_id.is_none());
        assert!(worker.marital_status.is_none());
        assert!(worker.managing_organization.is_none());
    }

    #[test]
    fn test_worker_serialization_roundtrip() {
        let name = HumanName {
            use_type: Some(NameUse::Official),
            family: "Smith".into(),
            given: vec!["John".into(), "Michael".into()],
            prefix: vec!["Dr.".into()],
            suffix: vec!["Jr.".into()],
        };
        let mut worker = Worker::new(name, Gender::Male);
        worker.birth_date = Some(chrono::NaiveDate::from_ymd_opt(1985, 3, 20).unwrap());
        worker.tax_id = Some("123-45-6789".into());

        let json = serde_json::to_string(&worker).expect("Serialization should succeed");
        let deserialized: Worker = serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.name.family, "Smith");
        assert_eq!(deserialized.name.given.len(), 2);
        assert_eq!(deserialized.gender, Gender::Male);
        assert_eq!(deserialized.tax_id.as_deref(), Some("123-45-6789"));
        assert_eq!(deserialized.birth_date, worker.birth_date);
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
        let worker = Worker::new(name, Gender::Female);
        let full = worker.full_name();
        assert_eq!(full, "Maria Elena Garcia");
    }

    #[test]
    fn test_worker_type_variants() {
        let types = vec![
            WorkerType::Doctor,
            WorkerType::Nurse,
            WorkerType::Carer,
            WorkerType::Staff,
            WorkerType::Employee,
            WorkerType::Manager,
            WorkerType::Supervisor,
            WorkerType::Consultant,
            WorkerType::Other,
        ];
        for wt in types {
            let json = serde_json::to_string(&wt).expect("WorkerType serialization");
            let deser: WorkerType = serde_json::from_str(&json).expect("WorkerType deserialization");
            assert_eq!(deser, wt);
        }
    }

    #[test]
    fn test_worker_with_worker_type() {
        let name = HumanName {
            use_type: None,
            family: "Chen".into(),
            given: vec!["Wei".into()],
            prefix: vec!["Dr.".into()],
            suffix: vec![],
        };
        let mut worker = Worker::new(name, Gender::Male);
        worker.worker_type = Some(WorkerType::Doctor);

        assert_eq!(worker.worker_type, Some(WorkerType::Doctor));
        assert_eq!(worker.worker_type.as_ref().unwrap().to_string(), "doctor");

        let json = serde_json::to_string(&worker).expect("Serialization");
        let deser: Worker = serde_json::from_str(&json).expect("Deserialization");
        assert_eq!(deser.worker_type, Some(WorkerType::Doctor));
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
