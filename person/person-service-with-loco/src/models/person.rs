//! The [`Person`] domain model — the central identity record of the service.
//!
//! A [`Person`] aggregates demographics, names, identifiers, contact
//! points, addresses, documents, and links to other records. It is the
//! unit that the matching engine compares, the search engine indexes,
//! and the repository persists (decomposed across several relational
//! tables by [`crate::db::repositories`]).
//!
//! This module also defines the name-related helper types
//! ([`HumanName`], [`NameUse`]) and the person-to-person link types
//! ([`PersonLink`], [`LinkType`]) used by the merge workflow.
//!
//! # Examples
//!
//! ```
//! use person_service::models::{Person, HumanName, Gender};
//!
//! let name = HumanName {
//!     use_type: None,
//!     family: "Smith".to_string(),
//!     given: vec!["John".to_string()],
//!     prefix: vec![],
//!     suffix: vec![],
//! };
//! let person = Person::new(name, Gender::Male);
//! assert_eq!(person.full_name(), "John Smith");
//! assert!(person.active);
//! ```

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{Address, ContactPoint, EmergencyContact, Gender, Identifier, IdentityDocument};

/// Person resource.
///
/// Server-generated fields (`id`, `created_at`, `updated_at`) and all
/// collection-typed fields default to sensible empty values when
/// missing from an incoming JSON body. Callers `POSTing` a new person
/// therefore only need to supply `name` + the demographic fields they
/// know — the service fills the rest. This is what the front-end's
/// `PersonRepository.create()` relies on.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Person {
    /// Unique person identifier — generated server-side on create.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    /// External identity keys (MRN, SSN, passport, tax, national IDs).
    /// Maps to the FHIR `Person.identifier` element; the matcher treats an
    /// exact identifier match as a strong (often short-circuiting) signal.
    #[serde(default)]
    pub identifiers: Vec<Identifier>,

    /// Whether this record is active. Soft delete sets this `false` rather
    /// than removing the row (FHIR `Person.active`). Defaults to `true`.
    #[serde(default = "default_true")]
    pub active: bool,

    /// Primary name (FHIR `Person.name`, the first / preferred entry).
    pub name: HumanName,

    /// Aliases, former names, and other non-primary names
    /// (FHIR `Person.name` entries beyond the first).
    #[serde(default)]
    pub additional_names: Vec<HumanName>,

    /// Contact points — phone, email, fax, etc. (FHIR `Person.telecom`).
    #[serde(default)]
    pub telecom: Vec<ContactPoint>,

    /// Administrative gender (FHIR `Person.gender`).
    pub gender: Gender,

    /// Date of birth (FHIR `Person.birthDate`), if known.
    #[serde(default)]
    pub birth_date: Option<NaiveDate>,

    /// National tax identifier (CPF, SSN, TIN, …). A convenience field
    /// distinct from the typed `identifiers` vector; see
    /// [`Person::effective_tax_id`] for how the two combine.
    #[serde(default)]
    pub tax_id: Option<String>,

    /// Identity documents — passport, birth certificate, national ID, etc.
    /// (stronger proofs of identity than a bare `Identifier`).
    #[serde(default)]
    pub documents: Vec<IdentityDocument>,

    /// Next-of-kin / points of contact to reach in an emergency.
    #[serde(default)]
    pub emergency_contacts: Vec<EmergencyContact>,

    /// Whether the person is recorded as deceased (FHIR
    /// `Person.deceasedBoolean`). Paired with `deceased_datetime`.
    #[serde(default)]
    pub deceased: bool,

    /// Date/time of death, if recorded (FHIR `Person.deceasedDateTime`).
    #[serde(default)]
    pub deceased_datetime: Option<DateTime<Utc>>,

    /// Physical / postal addresses (FHIR `Person.address`).
    #[serde(default)]
    pub addresses: Vec<Address>,

    /// Marital status as a free-text / coded string
    /// (FHIR `Person.maritalStatus`), if known.
    #[serde(default)]
    pub marital_status: Option<String>,

    /// Whether the person was part of a multiple birth
    /// (FHIR `Person.multipleBirthBoolean`), if known.
    #[serde(default)]
    pub multiple_birth: Option<bool>,

    /// References to photo attachments (FHIR `Person.photo`).
    #[serde(default)]
    pub photo: Vec<String>,

    /// The organization responsible for this record
    /// (FHIR `Person.managingOrganization`), by UUID.
    #[serde(default)]
    pub managing_organization: Option<Uuid>,

    /// Typed links to other person records (FHIR `Person.link`), created
    /// by the merge workflow (e.g. `Replaces` from main → duplicate).
    #[serde(default)]
    pub links: Vec<PersonLink>,

    /// Created timestamp — set server-side on create.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,

    /// Updated timestamp — set server-side on create and on update.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

/// serde default for [`Person::active`] — new records are active.
fn default_true() -> bool {
    true
}

/// Human name representation.
///
/// `prefix` and `suffix` are optional in the public contract — callers
/// MAY omit them (the front-end and FHIR Person resource often do).
/// They round-trip as empty arrays when omitted.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HumanName {
    /// Intended use of this name (official, nickname, maiden, …).
    #[serde(default)]
    pub use_type: Option<NameUse>,
    /// Family / last / surname.
    pub family: String,
    /// Given / first / middle names, in order.
    pub given: Vec<String>,
    /// Honorific prefixes (e.g. `Dr.`, `Mr.`).
    #[serde(default)]
    pub prefix: Vec<String>,
    /// Honorific suffixes (e.g. `Jr.`, `III`).
    #[serde(default)]
    pub suffix: Vec<String>,
}

/// Intended use of a [`HumanName`], mirroring the FHIR `name-use`
/// value set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NameUse {
    /// The name normally used.
    Usual,
    /// The official / legal name.
    Official,
    /// A temporary name.
    Temp,
    /// A nickname / alias.
    Nickname,
    /// An anonymous / pseudonymous name.
    Anonymous,
    /// A former name no longer in use.
    Old,
    /// A maiden (pre-marriage) name.
    Maiden,
}

/// A typed link from one person record to another.
///
/// Created by the merge workflow (e.g. a surviving record gains a
/// [`LinkType::Replaces`] link to the record it absorbed).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PersonLink {
    /// The UUID of the person on the other end of the link.
    pub other_person_id: Uuid,
    /// The semantic relationship this link expresses.
    pub link_type: LinkType,
}

/// The semantic relationship expressed by a [`PersonLink`], mirroring
/// the FHIR `link-type` value set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    /// The person resource containing this link is replaced by the linked person
    ReplacedBy,
    /// The person resource containing this link replaces the linked person
    Replaces,
    /// The person resource containing this link refers to the same person as the linked person
    Refer,
    /// The person resource containing this link is semantically referring to the linked person
    Seealso,
}

impl Person {
    /// Create a new active person from a name and gender.
    ///
    /// Generates a fresh UUID, stamps `created_at` / `updated_at` with
    /// the current time, marks the record `active`, and leaves every
    /// collection empty and every optional field `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::models::{Person, HumanName, Gender};
    ///
    /// let name = HumanName {
    ///     use_type: None,
    ///     family: "Doe".to_string(),
    ///     given: vec!["Jane".to_string()],
    ///     prefix: vec![],
    ///     suffix: vec![],
    /// };
    /// let person = Person::new(name, Gender::Female);
    /// assert!(person.active);
    /// assert!(person.identifiers.is_empty());
    /// ```
    #[must_use]
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

    /// Render the primary name as `"Given... Family"`.
    ///
    /// Given names are space-joined in order, then the family name is
    /// appended. Used by search indexing and the FHIR `name.text` field.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::models::{Person, HumanName, Gender};
    ///
    /// let name = HumanName {
    ///     use_type: None,
    ///     family: "Garcia".to_string(),
    ///     given: vec!["Maria".to_string(), "Elena".to_string()],
    ///     prefix: vec![],
    ///     suffix: vec![],
    /// };
    /// let person = Person::new(name, Gender::Female);
    /// assert_eq!(person.full_name(), "Maria Elena Garcia");
    /// ```
    #[must_use]
    pub fn full_name(&self) -> String {
        let given = self.name.given.join(" ");
        format!("{} {}", given, self.name.family)
    }

    /// Return the effective tax identifier for matching.
    ///
    /// Prefers the dedicated [`tax_id`](Person::tax_id) field; if that is
    /// `None`, falls back to the value of the first
    /// [`TAX`](super::IdentifierType::TAX)-typed entry in `identifiers`.
    /// The deterministic matcher uses this so a tax ID supplied either
    /// way short-circuits to a confident match.
    #[must_use]
    pub fn effective_tax_id(&self) -> Option<&str> {
        if let Some(ref tid) = self.tax_id {
            return Some(tid.as_str());
        }
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

    /// `Person::new` yields an active, non-deceased record with empty
    /// collections and `None` optionals.
    #[test]
    fn test_person_new_defaults() {
        let name = HumanName {
            use_type: None,
            family: "Doe".into(),
            given: vec!["Jane".into()],
            prefix: vec![],
            suffix: vec![],
        };
        let person = Person::new(name, Gender::Female);

        assert!(person.active);
        assert!(!person.deceased);
        assert_eq!(person.gender, Gender::Female);
        assert_eq!(person.name.family, "Doe");
        assert_eq!(person.name.given, vec!["Jane".to_string()]);
        assert!(person.identifiers.is_empty());
        assert!(person.addresses.is_empty());
        assert!(person.telecom.is_empty());
        assert!(person.documents.is_empty());
        assert!(person.emergency_contacts.is_empty());
        assert!(person.links.is_empty());
        assert!(person.birth_date.is_none());
        assert!(person.tax_id.is_none());
        assert!(person.marital_status.is_none());
        assert!(person.managing_organization.is_none());
    }

    /// A `Person` survives a JSON serialize → deserialize round-trip
    /// with key demographic fields intact.
    #[test]
    fn test_person_serialization_roundtrip() {
        let name = HumanName {
            use_type: Some(NameUse::Official),
            family: "Smith".into(),
            given: vec!["John".into(), "Michael".into()],
            prefix: vec!["Dr.".into()],
            suffix: vec!["Jr.".into()],
        };
        let mut person = Person::new(name, Gender::Male);
        person.birth_date = Some(NaiveDate::from_ymd_opt(1985, 3, 20).unwrap());
        person.tax_id = Some("123-45-6789".into());

        let json = serde_json::to_string(&person).expect("Serialization should succeed");
        let deserialized: Person =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.name.family, "Smith");
        assert_eq!(deserialized.name.given.len(), 2);
        assert_eq!(deserialized.gender, Gender::Male);
        assert_eq!(deserialized.tax_id.as_deref(), Some("123-45-6789"));
        assert_eq!(deserialized.birth_date, person.birth_date);
    }

    /// `full_name` joins multiple given names before the family name.
    #[test]
    fn test_human_name_display() {
        let name = HumanName {
            use_type: None,
            family: "Garcia".into(),
            given: vec!["Maria".into(), "Elena".into()],
            prefix: vec![],
            suffix: vec![],
        };
        let person = Person::new(name, Gender::Female);
        let full = person.full_name();
        assert_eq!(full, "Maria Elena Garcia");
    }

    /// Every `Gender` variant round-trips through JSON.
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
