//! Consent records for data-protection compliance (GDPR).
//!
//! Some things — a personal device, a personally-identifying record — are
//! subject to data-protection regimes. A [`Consent`](crate::models::consent::Consent)
//! captures a single grant of a particular
//! [`ConsentType`](crate::models::consent::ConsentType) against a
//! [`Thing`](crate::models::thing::Thing), tracking its
//! [`ConsentStatus`](crate::models::consent::ConsentStatus) and optional expiry.
//! [`Consent::is_active`](crate::models::consent::Consent::is_active)
//! is the one predicate callers need: it folds status and expiry into a
//! single boolean.
//!
//! # Examples
//!
//! ```
//! use jiff::Timestamp;
//! use uuid::Uuid;
//! use thing_service::models::consent::{Consent, ConsentStatus, ConsentType};
//!
//! let consent = Consent {
//!     id: Uuid::new_v4(),
//!     thing_id: Uuid::new_v4(),
//!     consent_type: ConsentType::DataProcessing,
//!     status: ConsentStatus::Active,
//!     granted_at: Timestamp::now(),
//!     expires_at: None, // never expires
//! };
//! assert!(consent.is_active());
//! ```

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The purpose a consent grant covers, mirroring common GDPR processing
/// bases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentType {
    /// Consent to process the subject's data.
    DataProcessing,
    /// Consent to share the subject's data with third parties.
    DataSharing,
    /// Consent to use the data for marketing.
    Marketing,
    /// Consent to use the data for research.
    Research,
}

/// The lifecycle state of a consent grant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentStatus {
    /// Granted and in force (subject to any expiry date).
    Active,
    /// Withdrawn by the subject; no longer in force.
    Revoked,
    /// Past its expiry date; no longer in force.
    Expired,
}

/// A single consent grant against a [`Thing`](crate::models::thing::Thing).
///
/// # Examples
///
/// ```
/// use jiff::Timestamp;
/// use uuid::Uuid;
/// use thing_service::models::consent::{Consent, ConsentStatus, ConsentType};
///
/// // A revoked grant is never active, regardless of expiry.
/// let consent = Consent {
///     id: Uuid::new_v4(),
///     thing_id: Uuid::new_v4(),
///     consent_type: ConsentType::Marketing,
///     status: ConsentStatus::Revoked,
///     granted_at: Timestamp::now(),
///     expires_at: Some(Timestamp::now() + jiff::SignedDuration::from_hours(24 * (30))),
/// };
/// assert!(!consent.is_active());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consent {
    /// Stable identifier for this consent record.
    pub id: Uuid,
    /// The [`Thing`](crate::models::thing::Thing) this consent applies to.
    pub thing_id: Uuid,
    /// What the grant authorises.
    pub consent_type: ConsentType,
    /// The grant's lifecycle state.
    pub status: ConsentStatus,
    /// When the grant was given.
    pub granted_at: Timestamp,
    /// When the grant lapses, or `None` for an open-ended grant.
    pub expires_at: Option<Timestamp>,
}

impl Consent {
    /// Returns `true` only if the grant is currently in force.
    ///
    /// A grant is active when its [`status`](Self::status) is
    /// [`Active`](ConsentStatus::Active) **and** it has not passed its
    /// [`expires_at`](Self::expires_at) (an open-ended grant with `None`
    /// never expires). Any other status returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::Timestamp;
    /// use uuid::Uuid;
    /// use thing_service::models::consent::{Consent, ConsentStatus, ConsentType};
    ///
    /// let expired = Consent {
    ///     id: Uuid::new_v4(),
    ///     thing_id: Uuid::new_v4(),
    ///     consent_type: ConsentType::Research,
    ///     status: ConsentStatus::Active,
    ///     granted_at: Timestamp::now() - jiff::SignedDuration::from_hours(24 * (2)),
    ///     expires_at: Some(Timestamp::now() - jiff::SignedDuration::from_hours(24 * (1))), // already lapsed
    /// };
    /// assert!(!expired.is_active());
    /// ```
    pub fn is_active(&self) -> bool {
        // A non-Active status (Revoked / Expired) is conclusive.
        if self.status != ConsentStatus::Active {
            return false;
        }
        // Status is Active; honour the expiry date if one is set.
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

    /// An Active grant with no expiry is active.
    #[test]
    fn test_consent_active() {
        let consent = Consent {
            id: Uuid::new_v4(),
            thing_id: Uuid::new_v4(),
            consent_type: ConsentType::DataProcessing,
            status: ConsentStatus::Active,
            granted_at: Timestamp::now(),
            expires_at: None,
        };
        assert!(consent.is_active());
    }

    /// A Revoked grant is never active.
    #[test]
    fn test_consent_revoked() {
        let consent = Consent {
            id: Uuid::new_v4(),
            thing_id: Uuid::new_v4(),
            consent_type: ConsentType::Marketing,
            status: ConsentStatus::Revoked,
            granted_at: Timestamp::now(),
            expires_at: None,
        };
        assert!(!consent.is_active());
    }

    /// An Active grant past its expiry date is not active.
    #[test]
    fn test_consent_expired_by_date() {
        let consent = Consent {
            id: Uuid::new_v4(),
            thing_id: Uuid::new_v4(),
            consent_type: ConsentType::DataSharing,
            status: ConsentStatus::Active,
            granted_at: Timestamp::now() - jiff::SignedDuration::from_hours(24 * (365)),
            expires_at: Some(Timestamp::now() - jiff::SignedDuration::from_hours(24 * (1))),
        };
        assert!(!consent.is_active());
    }

    /// An Active grant with a future expiry is still active.
    #[test]
    fn test_consent_not_yet_expired() {
        let consent = Consent {
            id: Uuid::new_v4(),
            thing_id: Uuid::new_v4(),
            consent_type: ConsentType::Research,
            status: ConsentStatus::Active,
            granted_at: Timestamp::now(),
            expires_at: Some(Timestamp::now() + jiff::SignedDuration::from_hours(24 * (365))),
        };
        assert!(consent.is_active());
    }
}
