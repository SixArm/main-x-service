//! GDPR consent records tied to a [`Place`](crate::models::place::Place).
//!
//! A [`Consent`] captures one purpose-scoped permission ([`ConsentType`])
//! with a lifecycle [`status`](Consent::status) and an optional expiry.
//! [`Consent::is_active`] is the single decision point downstream code uses
//! to gate data processing — it accounts for both an explicit
//! revoked/expired status and a passed expiry timestamp.
//!
//! # Examples
//!
//! ```
//! use jiff::{SignedDuration, Timestamp};
//! use uuid::Uuid;
//! use place_service::models::consent::{Consent, ConsentStatus, ConsentType};
//!
//! let consent = Consent {
//!     id: Uuid::new_v4(),
//!     place_id: Uuid::new_v4(),
//!     consent_type: ConsentType::DataProcessing,
//!     status: ConsentStatus::Active,
//!     granted_at: Timestamp::now(),
//!     expires_at: Some(Timestamp::now() + SignedDuration::from_hours(24 * 365)),
//! };
//! assert!(consent.is_active());
//! ```

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The purpose a consent covers (GDPR lawful-basis category).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentType {
    /// Consent to process the data at all.
    DataProcessing,
    /// Consent to share the data with third parties.
    DataSharing,
    /// Consent to use the data for marketing.
    Marketing,
    /// Consent to use the data for research.
    Research,
}

/// The lifecycle state of a consent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentStatus {
    /// Currently granted and in force.
    Active,
    /// Explicitly withdrawn by the subject.
    Revoked,
    /// No longer valid (e.g. past its expiry).
    Expired,
}

/// A single purpose-scoped consent record for a place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consent {
    /// Unique id of this consent record.
    pub id: Uuid,
    /// The place this consent pertains to.
    pub place_id: Uuid,
    /// What the consent authorizes.
    pub consent_type: ConsentType,
    /// The current lifecycle state.
    pub status: ConsentStatus,
    /// When the consent was granted.
    pub granted_at: Timestamp,
    /// When the consent expires, if ever. `None` means open-ended.
    pub expires_at: Option<Timestamp>,
}

impl Consent {
    /// Returns whether the consent is currently in force.
    ///
    /// A consent is active only when its [`status`](Self::status) is
    /// [`Active`](ConsentStatus::Active) **and** it has not passed any
    /// [`expires_at`](Self::expires_at) timestamp. A `None` expiry is treated
    /// as never-expiring.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::{SignedDuration, Timestamp};
    /// use uuid::Uuid;
    /// use place_service::models::consent::{Consent, ConsentStatus, ConsentType};
    ///
    /// let expired = Consent {
    ///     id: Uuid::new_v4(),
    ///     place_id: Uuid::new_v4(),
    ///     consent_type: ConsentType::Marketing,
    ///     status: ConsentStatus::Active,
    ///     granted_at: Timestamp::now() - SignedDuration::from_hours(24 * 2),
    ///     expires_at: Some(Timestamp::now() - SignedDuration::from_hours(24)),
    /// };
    /// assert!(!expired.is_active());
    /// ```
    pub fn is_active(&self) -> bool {
        // A non-Active status is decisive regardless of any expiry.
        if self.status != ConsentStatus::Active {
            return false;
        }
        // An Active status can still be effectively expired by its deadline.
        if let Some(expires) = self.expires_at {
            return Timestamp::now() < expires;
        }
        // Active with no expiry: in force indefinitely.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::SignedDuration;

    /// An active, never-expiring consent reports active.
    #[test]
    fn test_consent_active() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::DataProcessing,
            status: ConsentStatus::Active,
            granted_at: Timestamp::now(),
            expires_at: None,
        };
        assert!(consent.is_active());
    }

    /// A revoked consent reports inactive even with no expiry.
    #[test]
    fn test_consent_revoked() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::Marketing,
            status: ConsentStatus::Revoked,
            granted_at: Timestamp::now(),
            expires_at: None,
        };
        assert!(!consent.is_active());
    }

    /// An Active-status consent past its expiry reports inactive.
    #[test]
    fn test_consent_expired_by_date() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::DataSharing,
            status: ConsentStatus::Active,
            granted_at: Timestamp::now() - SignedDuration::from_hours(24 * 365),
            expires_at: Some(Timestamp::now() - SignedDuration::from_hours(24)),
        };
        assert!(!consent.is_active());
    }

    /// An Active consent with a future expiry reports active.
    #[test]
    fn test_consent_not_yet_expired() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::Research,
            status: ConsentStatus::Active,
            granted_at: Timestamp::now(),
            expires_at: Some(Timestamp::now() + SignedDuration::from_hours(24 * 365)),
        };
        assert!(consent.is_active());
    }
}
