use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentType {
    DataProcessing,
    DataSharing,
    Marketing,
    Research,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consent {
    pub id: Uuid,
    pub place_id: Uuid,
    pub consent_type: ConsentType,
    pub status: ConsentStatus,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Consent {
    pub fn is_active(&self) -> bool {
        if self.status != ConsentStatus::Active {
            return false;
        }
        if let Some(expires) = self.expires_at {
            return Utc::now() < expires;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_consent_active() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::DataProcessing,
            status: ConsentStatus::Active,
            granted_at: Utc::now(),
            expires_at: None,
        };
        assert!(consent.is_active());
    }

    #[test]
    fn test_consent_revoked() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::Marketing,
            status: ConsentStatus::Revoked,
            granted_at: Utc::now(),
            expires_at: None,
        };
        assert!(!consent.is_active());
    }

    #[test]
    fn test_consent_expired_by_date() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::DataSharing,
            status: ConsentStatus::Active,
            granted_at: Utc::now() - Duration::days(365),
            expires_at: Some(Utc::now() - Duration::days(1)),
        };
        assert!(!consent.is_active());
    }

    #[test]
    fn test_consent_not_yet_expired() {
        let consent = Consent {
            id: Uuid::new_v4(),
            place_id: Uuid::new_v4(),
            consent_type: ConsentType::Research,
            status: ConsentStatus::Active,
            granted_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::days(365)),
        };
        assert!(consent.is_active());
    }
}
