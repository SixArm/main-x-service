//! Privacy and data-masking utilities.
//!
//! Supports the privacy endpoints:
//! [`mask_worker`](crate::privacy::mask_worker) redacts sensitive fields for
//! the masked-view endpoint,
//! [`export_worker_data`](crate::privacy::export_worker_data) serialises a
//! full record for GDPR right-of-access, and
//! [`has_active_consent`](crate::privacy::has_active_consent) gates
//! purpose-bound processing.
//!
//! # Examples
//!
//! ```
//! use worker_service::privacy::mask_worker;
//! use worker_service::models::{Worker, HumanName, Gender};
//!
//! let mut w = Worker::new(
//!     HumanName { use_type: None, family: "Smith".into(),
//!         given: vec!["John".into()], prefix: vec![], suffix: vec![] },
//!     Gender::Male,
//! );
//! w.tax_id = Some("123-45-6789".into());
//! let masked = mask_worker(&w);
//! assert_eq!(masked.tax_id.as_deref(), Some("***-**-6789"));
//! // Non-sensitive fields are untouched.
//! assert_eq!(masked.name.family, "Smith");
//! ```

use crate::models::Worker;

/// Returns a clone of `worker` with sensitive fields redacted: the tax ID,
/// SSN/TAX/passport/driver's-license identifiers, all document numbers, and
/// phone/SMS/fax telecom values are masked to their last four characters.
/// Names, addresses, and email are left untouched.
pub fn mask_worker(worker: &Worker) -> Worker {
    let mut masked = worker.clone();

    // Mask tax ID: show only last 4 characters
    if let Some(ref tid) = masked.tax_id {
        masked.tax_id = Some(mask_value(tid, 4));
    }

    // Mask SSN and other sensitive identifiers
    for id in &mut masked.identifiers {
        match id.identifier_type {
            crate::models::IdentifierType::SSN | crate::models::IdentifierType::TAX => {
                id.value = mask_value(&id.value, 4);
            }
            crate::models::IdentifierType::PPN | crate::models::IdentifierType::DL => {
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

/// Masks all but the last `visible_chars` characters of `value`, replacing
/// each masked *alphanumeric* with `'*'` while leaving punctuation in place
/// (so "123-45-6789" becomes "***-**-6789"). Values no longer than
/// `visible_chars` are returned unchanged.
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

/// Returns `true` when `consents` contains an entry of the given type that is
/// [`Active`](crate::models::ConsentStatus::Active) and not past its expiry
/// date (a `None` expiry is treated as never-expiring).
pub fn has_active_consent(
    consents: &[crate::models::Consent],
    consent_type: crate::models::ConsentType,
) -> bool {
    let today = jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .date();

    consents.iter().any(|c| {
        c.consent_type == consent_type
            && c.status == crate::models::ConsentStatus::Active
            && c.expiry_date.map_or(true, |exp| exp >= today)
    })
}

/// Serialises the full worker record to a JSON value for a GDPR
/// right-of-access export. Falls back to `Value::Null` only if serialisation
/// fails (which, for the always-serializable [`Worker`], does not happen in
/// practice).
pub fn export_worker_data(worker: &Worker) -> serde_json::Value {
    serde_json::to_value(worker).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Masking keeps the last N chars and leaves punctuation in place.
    #[test]
    fn test_mask_value() {
        assert_eq!(mask_value("123-45-6789", 4), "***-**-6789");
        assert_eq!(mask_value("AB12345", 4), "***2345");
        assert_eq!(mask_value("short", 10), "short");
    }

    /// Tax ID and SSN are masked while the family name is preserved.
    #[test]
    fn test_mask_worker() {
        use crate::models::*;

        let mut worker = Worker::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        worker.tax_id = Some("123-45-6789".into());
        worker
            .identifiers
            .push(Identifier::ssn("123-45-6789".into()));

        let masked = mask_worker(&worker);
        assert_eq!(masked.tax_id.as_deref(), Some("***-**-6789"));
        assert_eq!(masked.identifiers[0].value, "***-**-6789");
        // Family name should NOT be masked
        assert_eq!(masked.name.family, "Smith");
    }

    /// Masking an email keeps the last four characters visible.
    #[test]
    fn test_mask_email() {
        // mask_value on an email-like string
        let masked = mask_value("john.doe@example.com", 4);
        assert!(
            masked.ends_with(".com"),
            "Should keep last 4 chars visible, got {}",
            masked
        );
        assert!(masked.contains('*'), "Should contain masked characters");
    }

    /// Masking a phone keeps the last four digits visible.
    #[test]
    fn test_mask_phone() {
        let masked = mask_value("+1-555-123-4567", 4);
        assert!(
            masked.ends_with("4567"),
            "Last 4 digits should be visible, got {}",
            masked
        );
    }

    /// Masking an SSN yields the canonical "***-**-NNNN" form.
    #[test]
    fn test_mask_ssn() {
        let masked = mask_value("123-45-6789", 4);
        assert_eq!(masked, "***-**-6789");
    }

    /// Values no longer than the visible count are returned unchanged.
    #[test]
    fn test_mask_short_value() {
        // Value shorter than visible_chars should be returned as-is
        assert_eq!(mask_value("AB", 4), "AB");
        assert_eq!(mask_value("", 4), "");
        assert_eq!(mask_value("ABCD", 4), "ABCD");
    }

    /// The GDPR export is a JSON object containing the core worker fields.
    #[test]
    fn test_export_worker_data_includes_all_fields() {
        use crate::models::*;

        let mut worker = Worker::new(
            HumanName {
                use_type: None,
                family: "Doe".into(),
                given: vec!["Jane".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        );
        worker.tax_id = Some("987-65-4321".into());
        worker.birth_date = Some(jiff::civil::date(1990, 5, 20));

        let export = export_worker_data(&worker);
        assert!(export.is_object(), "Export should be a JSON object");
        let obj = export.as_object().unwrap();
        assert!(obj.contains_key("name"), "Export should contain name");
        assert!(obj.contains_key("gender"), "Export should contain gender");
        assert!(obj.contains_key("tax_id"), "Export should contain tax_id");
        assert!(
            obj.contains_key("birth_date"),
            "Export should contain birth_date"
        );
        assert!(obj.contains_key("id"), "Export should contain id");
    }

    /// An active, unexpired consent of the right type is detected.
    #[test]
    fn test_consent_active_check() {
        use crate::models::{Consent, ConsentStatus, ConsentType};

        let consent = Consent {
            id: uuid::Uuid::new_v4(),
            worker_id: uuid::Uuid::new_v4(),
            consent_type: ConsentType::DataProcessing,
            status: ConsentStatus::Active,
            granted_date: jiff::civil::date(2024, 1, 1),
            expiry_date: Some(jiff::civil::date(2099, 12, 31)),
            revoked_date: None,
            purpose: Some("General data processing".into()),
            method: Some("electronic".into()),
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
        };

        assert!(has_active_consent(&[consent], ConsentType::DataProcessing));
    }

    /// A consent past its expiry date is not treated as active.
    #[test]
    fn test_consent_expired_check() {
        use crate::models::{Consent, ConsentStatus, ConsentType};

        let expired_consent = Consent {
            id: uuid::Uuid::new_v4(),
            worker_id: uuid::Uuid::new_v4(),
            consent_type: ConsentType::Marketing,
            status: ConsentStatus::Active,
            granted_date: jiff::civil::date(2020, 1, 1),
            expiry_date: Some(jiff::civil::date(2021, 1, 1)), // expired
            revoked_date: None,
            purpose: None,
            method: None,
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
        };

        assert!(
            !has_active_consent(&[expired_consent], ConsentType::Marketing),
            "Expired consent should not be considered active"
        );
    }
}
