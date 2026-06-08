//! Consent-management models for GDPR-style data-governance.
//!
//! A [`Consent`] record captures that a person granted (or later
//! revoked) permission of a given [`ConsentType`] (data processing,
//! sharing, marketing, research, emergency access). The privacy layer
//! ([`crate::privacy`]) checks active consent before performing
//! consent-gated operations.
//!
//! All enums serialize lowercase to match the wire/DB contract.

use jiff::{Timestamp, civil::Date};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

/// Type of consent
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsentType {
    /// Consent for data processing
    DataProcessing,
    /// Consent for data sharing with third parties
    DataSharing,
    /// Consent for marketing communications
    Marketing,
    /// Consent for research use of data
    Research,
    /// Consent for emergency access to data
    EmergencyAccess,
}

/// Status of a consent record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsentStatus {
    /// Consent is active
    Active,
    /// Consent has been revoked by the person
    Revoked,
    /// Consent has expired
    Expired,
}

/// A consent record for a person
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Consent {
    /// Unique consent record ID
    pub id: Uuid,

    /// Person who granted (or revoked) consent
    pub person_id: Uuid,

    /// Type of consent
    pub consent_type: ConsentType,

    /// Current status
    pub status: ConsentStatus,

    /// Date consent was granted
    pub granted_date: Date,

    /// Date consent expires (if applicable)
    pub expiry_date: Option<Date>,

    /// Date consent was revoked (if applicable)
    pub revoked_date: Option<Date>,

    /// Purpose description
    pub purpose: Option<String>,

    /// How consent was obtained (e.g., "written", "electronic", "verbal")
    pub method: Option<String>,

    /// When this consent record was created.
    pub created_at: Timestamp,
    /// When this consent record was last modified.
    pub updated_at: Timestamp,
}
