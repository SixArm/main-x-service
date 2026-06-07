//! Data models for the MPI (Master Patient Index) system.
//!
//! This module is the canonical home for the service's domain types —
//! the plain-data Rust structs and enums that flow through every layer
//! (REST handlers, matching engine, search index, repositories). They
//! are intentionally decoupled from the database row types in
//! [`crate::db::models`]; the repository layer translates between the
//! two so that storage concerns never leak into the domain.
//!
//! Submodules group the larger types ([`person`](crate::models::person), [`identifier`](crate::models::identifier),
//! [`document`](crate::models::document), …) while this file holds the small shared value
//! objects reused across them: [`Gender`](crate::models::Gender), [`Address`](crate::models::Address),
//! [`ContactPoint`](crate::models::ContactPoint), and their FHIR-aligned `*Use` / `*System` enums.
//!
//! All serde representations use `lowercase` renaming so that the JSON
//! shape matches the HL7 FHIR conventions the FHIR API layer expects
//! (e.g. `Gender::Male` ⇄ `"male"`).
//!
//! # Examples
//!
//! ```
//! use person_service::models::{Address, AddressUse};
//!
//! let addr = Address {
//!     use_type: Some(AddressUse::Home),
//!     line1: Some("1 Main St".to_string()),
//!     line2: None,
//!     city: Some("Springfield".to_string()),
//!     state: Some("IL".to_string()),
//!     postal_code: Some("62701".to_string()),
//!     country: Some("US".to_string()),
//! };
//! assert_eq!(addr.city.as_deref(), Some("Springfield"));
//! ```

use serde::{Deserialize, Serialize};

/// Central person identity record and its name/link helper types.
pub mod person;
/// Healthcare / managing organization record.
pub mod organization;
/// External identifiers (MRN, SSN, passport, tax, …).
pub mod identifier;
/// Identity documents (passport, driver's license, …).
pub mod document;
/// Emergency-contact records attached to a person.
pub mod emergency_contact;
/// Merge request / response / history types for deduplication.
pub mod merge;
/// Deduplication review-queue items and batch-scan request/response.
pub mod review_queue;
/// GDPR-style consent records and their type / status enums.
pub mod consent;

pub use person::{Person, HumanName, NameUse, PersonLink, LinkType};
pub use organization::Organization;
pub use identifier::{Identifier, IdentifierType, IdentifierUse};
pub use document::{IdentityDocument, DocumentType};
pub use emergency_contact::EmergencyContact;
pub use merge::{MergeRecord, MergeRequest, MergeResponse, MergeStatus};
pub use review_queue::{ReviewQueueItem, ReviewStatus, BatchDeduplicationRequest, BatchDeduplicationResponse};
pub use consent::{Consent, ConsentType, ConsentStatus};

/// Administrative gender, modeled on the HL7 FHIR `AdministrativeGender`
/// value set.
///
/// Serializes lowercase (`Gender::Male` ⇄ `"male"`), which is also the
/// shape persisted to the `persons.gender` column (the DB enforces a
/// lowercase `CHECK` constraint). [`Unknown`](Gender::Unknown) is the
/// safe default when source data is absent or unparseable, and the
/// gender matcher treats it as a partial (rather than failing) match.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    /// Male.
    Male,
    /// Female.
    Female,
    /// A gender that is neither exclusively male nor female.
    Other,
    /// Gender is not known / not recorded.
    Unknown,
}

/// A postal address.
///
/// Every field is optional because real-world source records are often
/// partial; the matcher and validator both tolerate missing components.
/// Field names mirror the FHIR `Address` element.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Address {
    /// Intended use of this address (home, work, …), if known.
    pub use_type: Option<AddressUse>,
    /// First street-address line (number + street).
    pub line1: Option<String>,
    /// Second street-address line (apartment, suite, …).
    pub line2: Option<String>,
    /// City / locality.
    pub city: Option<String>,
    /// State / province / region.
    pub state: Option<String>,
    /// Postal / ZIP code.
    pub postal_code: Option<String>,
    /// Country (typically an ISO country code).
    pub country: Option<String>,
}

/// Intended use of an [`Address`], mirroring the FHIR `address-use`
/// value set.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AddressUse {
    /// Residential address.
    Home,
    /// Workplace address.
    Work,
    /// Temporary address.
    Temp,
    /// No longer in use.
    Old,
    /// Address used for billing.
    Billing,
}

/// A single point of contact — a phone number, email, fax, etc.
///
/// The `(system, value)` pair identifies the channel and its address;
/// `use_type` records its intended use. Mirrors the FHIR `ContactPoint`
/// element.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContactPoint {
    /// The communication channel (phone, email, …).
    pub system: ContactPointSystem,
    /// The contact value (e.g. the phone number or email address).
    pub value: String,
    /// Intended use of this contact point, if known.
    pub use_type: Option<ContactPointUse>,
}

/// The communication channel of a [`ContactPoint`], mirroring the FHIR
/// `contact-point-system` value set.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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
    /// Any other channel.
    Other,
}

/// Intended use of a [`ContactPoint`], mirroring the FHIR
/// `contact-point-use` value set.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointUse {
    /// Home contact.
    Home,
    /// Work contact.
    Work,
    /// Temporary contact.
    Temp,
    /// No longer in use.
    Old,
    /// Mobile / cell contact.
    Mobile,
}
