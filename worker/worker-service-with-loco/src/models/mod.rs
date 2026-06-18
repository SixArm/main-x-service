//! Domain models for the worker-identity index.
//!
//! This module is the single home for the service's value objects and
//! aggregate records. The central aggregate is
//! [`Worker`](crate::models::worker::Worker) (in the
//! [`worker`](crate::models::worker) submodule); the remaining submodules
//! carry the worker's sub-objects ([`identifier`](crate::models::identifier),
//! [`document`](crate::models::document),
//! [`emergency_contact`](crate::models::emergency_contact)), related
//! aggregates ([`organization`](crate::models::organization),
//! [`consent`](crate::models::consent)), and the records that drive
//! deduplication workflows ([`merge`](crate::models::merge),
//! [`review_queue`](crate::models::review_queue)). The NHS ODS
//! ([`ods`](crate::models::ods), [`geography`](crate::models::geography),
//! [`codesystem`](crate::models::codesystem)) submodules add UK
//! Organisation-Data-Service alignment for
//! [`Organization`](crate::models::organization::Organization).
//!
//! A handful of small, widely shared value types live directly in this file —
//! [`Gender`](crate::models::Gender), [`Address`](crate::models::Address), and
//! [`ContactPoint`](crate::models::ContactPoint) (with their `*Use` /
//! `*System` enums) — because they are referenced by several submodules and
//! have no natural home of their own.
//!
//! The frequently used types are re-exported here so callers can write, for
//! example, `use worker_service::models::Worker;` rather than the longer
//! `worker_service::models::worker::Worker`.
//!
//! # Examples
//!
//! ```
//! use worker_service::models::{Address, AddressUse, ContactPoint, ContactPointSystem, Gender};
//!
//! let gender = Gender::Female;
//! assert_eq!(serde_json::to_string(&gender).unwrap(), "\"female\"");
//!
//! let phone = ContactPoint {
//!     system: ContactPointSystem::Phone,
//!     value: "+1-202-555-0143".into(),
//!     use_type: None,
//! };
//! assert_eq!(phone.value, "+1-202-555-0143");
//!
//! let address = Address {
//!     use_type: Some(AddressUse::Home),
//!     line1: Some("1 Main St".into()),
//!     line2: None,
//!     city: Some("Springfield".into()),
//!     state: Some("IL".into()),
//!     postal_code: Some("62701".into()),
//!     country: Some("US".into()),
//! };
//! assert_eq!(address.city.as_deref(), Some("Springfield"));
//! ```

use serde::{Deserialize, Serialize};

// Submodules: one file per cluster of related model types.

/// NHS ODS CodeSystem lookup types (role / relationship / record-class names).
pub mod codesystem;
/// Consent records (type, status, validity dates) for privacy gating.
pub mod consent;
/// Identity documents (passport, national ID, driver's license, …).
pub mod document;
/// Emergency-contact records (name, relationship, telecom, address).
pub mod emergency_contact;
/// ONS/NHS geography reference types (postcode-to-area mappings).
pub mod geography;
/// External identifiers (MRN, SSN, DL, NPI, PPN, TAX, …) and their types.
pub mod identifier;
/// Merge request / response / record types for the dedup merge workflow.
pub mod merge;
/// NHS ODS organisation-data types (roles, relationships, successions, periods).
pub mod ods;
/// Organization aggregate (with NHS ODS alignment).
pub mod organization;
/// Deduplication review-queue items and batch-dedup request/response types.
pub mod review_queue;
/// The central [`Worker`](worker::Worker) aggregate and its name/link types.
pub mod worker;

pub use codesystem::{
    GeographyNameReference, OdsRecordClassReference, OdsRecordUseTypeReference,
    OdsRelationshipReference, OdsRoleReference, PractitionerRoleReference,
};
pub use consent::{Consent, ConsentStatus, ConsentType};
pub use document::{DocumentType, IdentityDocument};
pub use emergency_contact::EmergencyContact;
pub use geography::PostcodeGeography;
pub use identifier::{Identifier, IdentifierType, IdentifierUse};
pub use merge::{MergeRecord, MergeRequest, MergeResponse, MergeStatus};
pub use ods::{
    DatePeriod, OdsStatus, OrganizationRelationship, OrganizationRole, OrganizationSuccession,
    PeriodType, RecordClass, RecordUseType, SuccessionType,
};
pub use organization::Organization;
pub use review_queue::{
    BatchDeduplicationRequest, BatchDeduplicationResponse, ReviewQueueItem, ReviewStatus,
};
pub use worker::{HumanName, LinkType, NameUse, Worker, WorkerLink, WorkerType};

/// Administrative gender, modeled on the HL7 FHIR
/// [`AdministrativeGender`](https://www.hl7.org/fhir/valueset-administrative-gender.html)
/// value set.
///
/// Serializes in lowercase (`"male"`, `"female"`, `"other"`, `"unknown"`) to
/// match the FHIR wire format. [`Unknown`](Self::Unknown) is the safe default
/// when the source system did not record a gender; the matcher treats it as a
/// "no information" value rather than a mismatch (see
/// `crate::matching::algorithms`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    /// Male.
    Male,
    /// Female.
    Female,
    /// A gender other than male or female.
    Other,
    /// Gender is not known / not recorded.
    Unknown,
}

/// A postal address, modeled loosely on the FHIR `Address` datatype.
///
/// Every field is optional because source systems vary widely in how much of
/// an address they capture. The validation layer (`crate::validation`)
/// enforces that at least one of [`city`](Self::city),
/// [`postal_code`](Self::postal_code), or [`country`](Self::country) is
/// present, and the matcher weights the structured fields when comparing two
/// addresses.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Address {
    /// How the address is used (home, work, …); `None` if unspecified.
    pub use_type: Option<AddressUse>,
    /// First street line (house number + street).
    pub line1: Option<String>,
    /// Second street line (apartment, suite, …).
    pub line2: Option<String>,
    /// City / locality.
    pub city: Option<String>,
    /// State / province / region.
    pub state: Option<String>,
    /// Postal / ZIP code.
    pub postal_code: Option<String>,
    /// Country, ideally an ISO 3166 code.
    pub country: Option<String>,
}

/// The purpose an [`Address`] serves for a worker. Serializes in lowercase.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AddressUse {
    /// Residential address.
    Home,
    /// Workplace address.
    Work,
    /// Temporary address.
    Temp,
    /// Address no longer in use.
    Old,
    /// Address used for billing.
    Billing,
}

/// A single channel for reaching a worker (phone, email, fax, …), modeled on
/// the FHIR `ContactPoint` datatype.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContactPoint {
    /// The communication system this value belongs to.
    pub system: ContactPointSystem,
    /// The contact value itself (the phone number, email address, …).
    pub value: String,
    /// How this contact is used (home, work, mobile, …); `None` if unspecified.
    pub use_type: Option<ContactPointUse>,
}

/// The communication system a [`ContactPoint`] value uses. Serializes in
/// lowercase to match the FHIR wire format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointSystem {
    /// Telephone number.
    Phone,
    /// Fax number.
    Fax,
    /// Email address.
    Email,
    /// Pager number.
    Pager,
    /// Web URL.
    Url,
    /// SMS-capable number.
    Sms,
    /// Some other contact system.
    Other,
}

/// How a [`ContactPoint`] is used. Serializes in lowercase.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointUse {
    /// Home contact.
    Home,
    /// Work contact.
    Work,
    /// Temporary contact.
    Temp,
    /// Contact no longer in use.
    Old,
    /// Mobile contact.
    Mobile,
}
