//! The [`Worker`] aggregate — the central record of the worker-identity index.
//!
//! A [`Worker`] gathers everything the registry knows about one real-world
//! worker: their [`HumanName`] (plus aliases), identifiers, contacts,
//! addresses, identity documents, emergency contacts, and links to other
//! worker records. Most fields are collections or `Option`s because the data
//! is assembled incrementally from many source systems, and few sources
//! supply every attribute.
//!
//! This file also defines the worker-specific helper types: [`HumanName`] and
//! its [`NameUse`], the [`WorkerType`] classification, and the [`WorkerLink`]
//! /[`LinkType`] pair used to express "this record replaces / refers to that
//! one" relationships created during merges.
//!
//! # Examples
//!
//! ```
//! use worker_service::models::{Worker, HumanName, Gender, WorkerType};
//!
//! let name = HumanName {
//!     use_type: None,
//!     family: "Okafor".into(),
//!     given: vec!["Ada".into()],
//!     prefix: vec!["Dr.".into()],
//!     suffix: vec![],
//! };
//! let mut worker = Worker::new(name, Gender::Female);
//! worker.worker_type = Some(WorkerType::Doctor);
//!
//! assert_eq!(worker.full_name(), "Ada Okafor");
//! assert!(worker.active);
//! ```

use jiff::{Timestamp, civil::Date};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{Address, ContactPoint, EmergencyContact, Gender, Identifier, IdentityDocument};

/// A worker identity record — the registry's central aggregate.
///
/// Created via [`Worker::new`], which assigns a fresh v4 [`Uuid`] and sets the
/// creation/update timestamps. The remaining fields start empty/`None` and are
/// populated as data arrives. Soft-delete is expressed through [`active`](Self::active)
/// (set `false` rather than removing the row) so the audit trail stays intact.
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
    pub birth_date: Option<Date>,

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
    pub deceased_datetime: Option<Timestamp>,

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
    pub created_at: Timestamp,

    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// A structured human name, modeled on the FHIR `HumanName` datatype.
///
/// Splitting the name into [`family`](Self::family), [`given`](Self::given),
/// [`prefix`](Self::prefix), and [`suffix`](Self::suffix) lets the matcher
/// compare components independently (family-name similarity is weighted more
/// heavily than given-name similarity — see `crate::matching::algorithms`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HumanName {
    /// How this name is used (official, nickname, maiden, …); `None` if unspecified.
    pub use_type: Option<NameUse>,
    /// Family / last / surname.
    pub family: String,
    /// Ordered given / first / middle names.
    pub given: Vec<String>,
    /// Honorific prefixes such as `Dr.` or `Mr.`.
    pub prefix: Vec<String>,
    /// Generational or qualification suffixes such as `Jr.` or `III`.
    pub suffix: Vec<String>,
}

/// The role a [`HumanName`] plays, mirroring the FHIR `name-use` value set.
/// Serializes in lowercase.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NameUse {
    /// The name normally used.
    Usual,
    /// The name registered on official documents.
    Official,
    /// A temporary name.
    Temp,
    /// A nickname / preferred informal name.
    Nickname,
    /// A name used to preserve anonymity.
    Anonymous,
    /// A name no longer in use.
    Old,
    /// A maiden (pre-marriage) name.
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

/// Renders the snake_case wire token for each variant (e.g. `WorkerType::Doctor`
/// → `"doctor"`), matching its serde representation.
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

/// A typed link from this worker record to another, used to record merges and
/// cross-references (e.g. after a merge the duplicate gets a `ReplacedBy` link
/// and the survivor gets a `Replaces` link).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkerLink {
    /// The [`Uuid`] of the linked worker record.
    pub other_worker_id: Uuid,
    /// The semantics of the link (see [`LinkType`]).
    pub link_type: LinkType,
}

/// The semantics of a [`WorkerLink`], mirroring the FHIR `link-type` value
/// set. Serializes in lowercase.
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
    /// Creates a new active worker with a fresh v4 [`Uuid`] and
    /// `created_at`/`updated_at` set to the current time.
    ///
    /// All collection fields start empty and all optional fields start `None`;
    /// populate them via field assignment after construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::{Worker, HumanName, Gender};
    ///
    /// let name = HumanName {
    ///     use_type: None,
    ///     family: "Lee".into(),
    ///     given: vec!["Min".into()],
    ///     prefix: vec![],
    ///     suffix: vec![],
    /// };
    /// let worker = Worker::new(name, Gender::Unknown);
    /// assert!(worker.active);
    /// assert!(worker.identifiers.is_empty());
    /// ```
    pub fn new(name: HumanName, gender: Gender) -> Self {
        let now = Timestamp::now();
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

    /// Returns the display name as `"<given...> <family>"`.
    ///
    /// The given names are space-joined in order, then the family name is
    /// appended. With no given names the result has a leading space.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::{Worker, HumanName, Gender};
    ///
    /// let name = HumanName {
    ///     use_type: None,
    ///     family: "Garcia".into(),
    ///     given: vec!["Maria".into(), "Elena".into()],
    ///     prefix: vec![],
    ///     suffix: vec![],
    /// };
    /// let worker = Worker::new(name, Gender::Female);
    /// assert_eq!(worker.full_name(), "Maria Elena Garcia");
    /// ```
    pub fn full_name(&self) -> String {
        let given = self.name.given.join(" ");
        format!("{} {}", given, self.name.family)
    }

    /// Returns the worker's effective tax ID for matching purposes.
    ///
    /// Prefers the dedicated [`tax_id`](Self::tax_id) field; if that is `None`,
    /// falls back to the value of the first identifier whose type is
    /// [`IdentifierType::TAX`](super::IdentifierType::TAX). Returns `None` when
    /// neither source carries a tax ID. The deterministic matcher uses this to
    /// short-circuit on an exact tax-ID match.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::{Worker, HumanName, Gender};
    ///
    /// let name = HumanName {
    ///     use_type: None, family: "Roe".into(), given: vec!["Jo".into()],
    ///     prefix: vec![], suffix: vec![],
    /// };
    /// let mut worker = Worker::new(name, Gender::Other);
    /// assert_eq!(worker.effective_tax_id(), None);
    ///
    /// worker.tax_id = Some("123-45-6789".into());
    /// assert_eq!(worker.effective_tax_id(), Some("123-45-6789"));
    /// ```
    pub fn effective_tax_id(&self) -> Option<&str> {
        // The dedicated field wins when present.
        if let Some(ref tid) = self.tax_id {
            return Some(tid.as_str());
        }
        // Otherwise scan the identifier list for a TAX-typed entry.
        self.identifiers
            .iter()
            .find(|id| id.identifier_type == super::IdentifierType::TAX)
            .map(|id| id.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Gender;

    /// `new` sets sensible defaults: active, not deceased, empty collections.
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

    /// A worker survives a JSON serialize/deserialize round-trip unchanged.
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
        worker.birth_date = Some(jiff::civil::date(1985, 3, 20));
        worker.tax_id = Some("123-45-6789".into());

        let json = serde_json::to_string(&worker).expect("Serialization should succeed");
        let deserialized: Worker =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.name.family, "Smith");
        assert_eq!(deserialized.name.given.len(), 2);
        assert_eq!(deserialized.gender, Gender::Male);
        assert_eq!(deserialized.tax_id.as_deref(), Some("123-45-6789"));
        assert_eq!(deserialized.birth_date, worker.birth_date);
    }

    /// `full_name` joins all given names before the family name.
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

    /// Every [`WorkerType`] variant round-trips through JSON.
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
            let deser: WorkerType =
                serde_json::from_str(&json).expect("WorkerType deserialization");
            assert_eq!(deser, wt);
        }
    }

    /// A worker's `worker_type` round-trips and its `Display` is correct.
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

    /// Every [`Gender`] variant round-trips through JSON.
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
