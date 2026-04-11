//! Consent management models

use chrono::{DateTime, NaiveDate, Utc};
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
    /// Consent has been revoked by the worker
    Revoked,
    /// Consent has expired
    Expired,
}

/// A consent record for a worker
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Consent {
    /// Unique consent record ID
    pub id: Uuid,

    /// Worker who granted (or revoked) consent
    pub worker_id: Uuid,

    /// Type of consent
    pub consent_type: ConsentType,

    /// Current status
    pub status: ConsentStatus,

    /// Date consent was granted
    pub granted_date: NaiveDate,

    /// Date consent expires (if applicable)
    pub expiry_date: Option<NaiveDate>,

    /// Date consent was revoked (if applicable)
    pub revoked_date: Option<NaiveDate>,

    /// Purpose description
    pub purpose: Option<String>,

    /// How consent was obtained (e.g., "written", "electronic", "verbal")
    pub method: Option<String>,

    /// Record timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
