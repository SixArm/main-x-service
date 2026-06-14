//! Consent management models.
//!
//! A [`Consent`] record captures permission attached to an event —
//! for example, consent to share data about a clinical encounter, or a
//! media-release consent for recording a performance. Consent has a
//! [`ConsentType`] (what it covers) and a [`ConsentStatus`]
//! (active / revoked / expired). The privacy layer consults these
//! records when honoring GDPR and data-sharing rules.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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
    /// Consent has been revoked by the event
    Revoked,
    /// Consent has expired
    Expired,
}

/// A consent record attached to an event (e.g. consent for data
/// sharing about a clinical encounter, or a media-release consent
/// for a performance recording).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Consent {
    /// Unique consent record ID
    pub id: Uuid,

    /// The event this consent applies to
    pub event_id: Uuid,

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

    /// When this consent record was created.
    pub created_at: DateTime<Utc>,
    /// When this consent record was last modified.
    pub updated_at: DateTime<Utc>,
}
