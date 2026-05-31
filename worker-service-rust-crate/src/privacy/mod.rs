//! Privacy and data masking utilities
//!
//! Provides data masking for sensitive fields, access control helpers,
//! and consent checking for GDPR compliance.

use crate::models::Worker;

/// Mask sensitive fields in a worker record for display.
/// Returns a new worker with masked data.
pub fn mask_worker(worker: &Worker) -> Worker {
    let mut masked = worker.clone();

    // Mask tax ID: show only last 4 characters
    if let Some(ref tid) = masked.tax_id {
        masked.tax_id = Some(mask_value(tid, 4));
    }

    // Mask SSN and other sensitive identifiers
    for id in &mut masked.identifiers {
        match id.identifier_type {
            crate::models::IdentifierType::SSN
            | crate::models::IdentifierType::TAX => {
                id.value = mask_value(&id.value, 4);
            }
            crate::models::IdentifierType::PPN
            | crate::models::IdentifierType::DL => {
                id.value = mask_value(&id.value, 4);
            }
            _ => {}
        }
    }

    // Mask document numbers
    for doc in &mut masked.documents {
        doc.number = mask_value(&doc.number, 4);
    }

    // Mask phone numbers: show only last 4 digits
    for cp in &mut masked.telecom {
        match cp.system {
            crate::models::ContactPointSystem::Phone
            | crate::models::ContactPointSystem::Sms
            | crate::models::ContactPointSystem::Fax => {
                cp.value = mask_value(&cp.value, 4);
            }
            _ => {}
        }
    }

    masked
}

/// Mask a value, keeping only the last `visible_chars` characters visible.
/// E.g., "123-45-6789" with visible=4 becomes "***-**-6789"
fn mask_value(value: &str, visible_chars: usize) -> String {
    if value.len() <= visible_chars {
        return value.to_string();
    }

    let visible_start = value.len() - visible_chars;
    let masked_part: String = value[..visible_start]
        .chars()
        .map(|c| if c.is_alphanumeric() { '*' } else { c })
        .collect();

    format!("{}{}", masked_part, &value[visible_start..])
}

/// Check whether a worker has active consent for a given purpose.
pub fn has_active_consent(
    consents: &[crate::models::Consent],
    consent_type: crate::models::ConsentType,
) -> bool {
    let today = chrono::Utc::now().date_naive();

    consents.iter().any(|c| {
        c.consent_type == consent_type
            && c.status == crate::models::ConsentStatus::Active
            && c.expiry_date.map_or(true, |exp| exp >= today)
    })
}

/// Generate a GDPR data export for a worker (right of access).
/// Returns a JSON value containing all stored worker data.
pub fn export_worker_data(worker: &Worker) -> serde_json::Value {
    serde_json::to_value(worker).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_value() {
        assert_eq!(mask_value("123-45-6789", 4), "***-**-6789");
        assert_eq!(mask_value("AB12345", 4), "***2345");
        assert_eq!(mask_value("short", 10), "short");
    }

    #[test]
    fn test_mask_worker() {
        use crate::models::*;

        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.tax_id = Some("123-45-6789".into());
        worker.identifiers.push(Identifier::ssn("123-45-6789".into()));

        let masked = mask_worker(&worker);
        assert_eq!(masked.tax_id.as_deref(), Some("***-**-6789"));
        assert_eq!(masked.identifiers[0].value, "***-**-6789");
        // Family name should NOT be masked
        assert_eq!(masked.name.family, "Smith");
    }

    #[test]
    fn test_mask_email() {
        // mask_value on an email-like string
        let masked = mask_value("john.doe@example.com", 4);
        assert!(masked.ends_with(".com"), "Should keep last 4 chars visible, got {}", masked);
        assert!(masked.contains('*'), "Should contain masked characters");
    }

    #[test]
    fn test_mask_phone() {
        let masked = mask_value("+1-555-123-4567", 4);
        assert!(masked.ends_with("4567"), "Last 4 digits should be visible, got {}", masked);
    }

    #[test]
    fn test_mask_ssn() {
        let masked = mask_value("123-45-6789", 4);
        assert_eq!(masked, "***-**-6789");
    }

    #[test]
    fn test_mask_short_value() {
        // Value shorter than visible_chars should be returned as-is
        assert_eq!(mask_value("AB", 4), "AB");
        assert_eq!(mask_value("", 4), "");
        assert_eq!(mask_value("ABCD", 4), "ABCD");
    }

    #[test]
    fn test_export_worker_data_includes_all_fields() {
        use crate::models::*;

        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Doe".into(), given: vec!["Jane".into()], prefix: vec![], suffix: vec![] },
            Gender::Female,
        );
        worker.tax_id = Some("987-65-4321".into());
        worker.birth_date = Some(chrono::NaiveDate::from_ymd_opt(1990, 5, 20).unwrap());

        let export = export_worker_data(&worker);
        assert!(export.is_object(), "Export should be a JSON object");
        let obj = export.as_object().unwrap();
        assert!(obj.contains_key("name"), "Export should contain name");
        assert!(obj.contains_key("gender"), "Export should contain gender");
        assert!(obj.contains_key("tax_id"), "Export should contain tax_id");
        assert!(obj.contains_key("birth_date"), "Export should contain birth_date");
        assert!(obj.contains_key("id"), "Export should contain id");
    }

    #[test]
    fn test_consent_active_check() {
        use crate::models::{Consent, ConsentType, ConsentStatus};

        let consent = Consent {
            id: uuid::Uuid::new_v4(),
            worker_id: uuid::Uuid::new_v4(),
            consent_type: ConsentType::DataProcessing,
            status: ConsentStatus::Active,
            granted_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            expiry_date: Some(chrono::NaiveDate::from_ymd_opt(2099, 12, 31).unwrap()),
            revoked_date: None,
            purpose: Some("General data processing".into()),
            method: Some("electronic".into()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(has_active_consent(&[consent], ConsentType::DataProcessing));
    }

    #[test]
    fn test_consent_expired_check() {
        use crate::models::{Consent, ConsentType, ConsentStatus};

        let expired_consent = Consent {
            id: uuid::Uuid::new_v4(),
            worker_id: uuid::Uuid::new_v4(),
            consent_type: ConsentType::Marketing,
            status: ConsentStatus::Active,
            granted_date: chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            expiry_date: Some(chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()), // expired
            revoked_date: None,
            purpose: None,
            method: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(!has_active_consent(&[expired_consent], ConsentType::Marketing),
            "Expired consent should not be considered active");
    }
}
