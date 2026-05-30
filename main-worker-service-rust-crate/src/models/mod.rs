//! Data models for the MPI system

use serde::{Deserialize, Serialize};

pub mod worker;
pub mod organization;
pub mod identifier;
pub mod document;
pub mod emergency_contact;
pub mod merge;
pub mod review_queue;
pub mod consent;
pub mod ods;
pub mod geography;
pub mod codesystem;

pub use worker::{Worker, HumanName, NameUse, WorkerLink, LinkType, WorkerType};
pub use organization::Organization;
pub use identifier::{Identifier, IdentifierType, IdentifierUse};
pub use ods::{
    OdsStatus, RecordClass, RecordUseType, PeriodType, DatePeriod,
    OrganizationRole, OrganizationRelationship, OrganizationSuccession, SuccessionType,
};
pub use geography::PostcodeGeography;
pub use codesystem::{
    OdsRoleReference, OdsRelationshipReference, OdsRecordClassReference,
    OdsRecordUseTypeReference, PractitionerRoleReference, GeographyNameReference,
};
pub use document::{IdentityDocument, DocumentType};
pub use emergency_contact::EmergencyContact;
pub use merge::{MergeRecord, MergeRequest, MergeResponse, MergeStatus};
pub use review_queue::{ReviewQueueItem, ReviewStatus, BatchDeduplicationRequest, BatchDeduplicationResponse};
pub use consent::{Consent, ConsentType, ConsentStatus};

/// Gender enumeration per FHIR specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Other,
    Unknown,
}

/// Address information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Address {
    pub use_type: Option<AddressUse>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Address use type
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AddressUse {
    Home,
    Work,
    Temp,
    Old,
    Billing,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContactPoint {
    pub system: ContactPointSystem,
    pub value: String,
    pub use_type: Option<ContactPointUse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointSystem {
    Phone,
    Fax,
    Email,
    Pager,
    Url,
    Sms,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointUse {
    Home,
    Work,
    Temp,
    Old,
    Mobile,
}
