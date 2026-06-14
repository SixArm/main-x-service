//! Consent management models (GDPR / privacy).
//!
//! A [`Consent`] record captures one grant of permission by a worker for a
//! specific [`ConsentType`] (data processing, sharing, marketing, research,
//! emergency access). Its [`ConsentStatus`] tracks the lifecycle —
//! [`Active`](ConsentStatus::Active), [`Revoked`](ConsentStatus::Revoked), or
//! [`Expired`](ConsentStatus::Expired) — and the dates record when each
//! transition happened. The privacy layer (`crate::privacy`) consults these
//! records before sharing or processing data.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Type of consent — the purpose for which a worker granted permission.
///
/// Serializes in lowercase (`"dataprocessing"`, `"datasharing"`, …) to match
/// the JSON wire format. The privacy layer (`crate::privacy`) gates each
/// data-handling action on the presence of an [`Active`](ConsentStatus::Active)
/// consent of the matching type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
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

/// Status of a consent record — its position in the grant/revoke/expire
/// lifecycle. Serializes in lowercase (`"active"`, `"revoked"`, `"expired"`).
/// Only [`Active`](Self::Active) consent authorizes data handling.
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

/// A consent record for a worker — one grant of permission of a given
/// [`ConsentType`], with the dates that bound its validity.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Consent {
    /// Unique consent record ID (a fresh v4 UUID).
    pub id: Uuid,

    /// The [`Worker`](crate::models::Worker) this consent belongs to, by ID.
    pub worker_id: Uuid,

    /// What the worker consented to (data processing, sharing, …).
    pub consent_type: ConsentType,

    /// Where the consent sits in its lifecycle (active / revoked / expired).
    pub status: ConsentStatus,

    /// Date the consent was granted (always present).
    pub granted_date: NaiveDate,

    /// Date the consent expires; `None` for open-ended consent.
    pub expiry_date: Option<NaiveDate>,

    /// Date the consent was revoked; `None` unless `status` is
    /// [`ConsentStatus::Revoked`].
    pub revoked_date: Option<NaiveDate>,

    /// Free-text description of the purpose the consent covers.
    pub purpose: Option<String>,

    /// How consent was obtained (e.g., "written", "electronic", "verbal")
    pub method: Option<String>,

    /// When this consent record was created.
    pub created_at: DateTime<Utc>,
    /// When this consent record was last updated.
    pub updated_at: DateTime<Utc>,
}
