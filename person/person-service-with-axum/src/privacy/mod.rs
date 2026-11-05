//! Privacy: data masking, consent checking, and GDPR export.
//!
//! These helpers back the privacy REST endpoints. [`mask_person`](crate::privacy::mask_person)
//! redacts sensitive fields (tax ID, SSN/TAX/passport/DL identifiers,
//! document numbers, phone/SMS/fax values) for the masked-view
//! endpoint; [`export_person_data`](crate::privacy::export_person_data) serializes the full record for the
//! GDPR right-of-access export; and [`has_active_consent`](crate::privacy::has_active_consent) answers
//! whether a person has granted a given consent type that has not
//! expired.
//!
//! Masking keeps the last few characters visible (see `mask_value`) so
//! operators can still disambiguate records without exposing the full
//! secret. All functions are pure.
//!
//! # Examples
//!
//! ```
//! use person_service::models::{Person, HumanName, Gender};
//! use person_service::privacy::mask_person;
//!
//! let mut p = Person::new(
//!     HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
//!     Gender::Male,
//! );
//! p.tax_id = Some("123-45-6789".into());
//! assert_eq!(mask_person(&p).tax_id.as_deref(), Some("***-**-6789"));
//! ```

use crate::models::Person;
use chrono::Utc;

/// Return a copy of `person` with sensitive fields masked for display.
///
/// Redacts the tax ID, SSN/TAX/passport/driver-license identifier
/// values, all document numbers, and phone/SMS/fax contact values,
/// keeping only the last four characters of each visible. Names,
/// addresses, and emails are left intact.
#[must_use]
pub fn mask_person(person: &Person) -> Person {
    let mut masked = person.clone();

    // Mask tax ID: show only last 4 characters
    if let Some(ref tid) = masked.tax_id {
        masked.tax_id = Some(mask_value(tid, 4));
    }

    // Mask SSN and other sensitive identifiers
    for id in &mut masked.identifiers {
        match id.identifier_type {
            crate::models::IdentifierType::SSN
            | crate::models::IdentifierType::TAX
            | crate::models::IdentifierType::PPN
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

/// Mask a string, keeping only its last `visible_chars` characters.
///
/// Alphanumeric characters in the hidden prefix become `*` while
/// punctuation (e.g. hyphens) is preserved for readability, so
/// `"123-45-6789"` with `visible_chars = 4` becomes `"***-**-6789"`.
/// Values no longer than `visible_chars` are returned unchanged.
///
/// Counts Unicode scalar values (`char`s), not bytes, so multibyte
/// input (accented names, non-Latin identifiers) is masked correctly
/// and never slices across a UTF-8 char boundary.
fn mask_value(value: &str, visible_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= visible_chars {
        return value.to_string();
    }

    let hidden = char_count - visible_chars;
    value
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i < hidden && c.is_alphanumeric() {
                '*'
            } else {
                c
            }
        })
        .collect()
}

/// Return `true` if `consents` contains an active, unexpired consent of
/// the given type.
///
/// A consent counts when its `consent_type` matches, its status is
/// [`ConsentStatus::Active`](crate::models::ConsentStatus::Active), and
/// its `expiry_date` is absent or not in the past.
#[must_use]
pub fn has_active_consent(
    consents: &[crate::models::Consent],
    consent_type: &crate::models::ConsentType,
) -> bool {
    let today = Utc::now().date_naive();

    consents.iter().any(|c| {
        c.consent_type == *consent_type
            && c.status == crate::models::ConsentStatus::Active
            && c.expiry_date.is_none_or(|exp| exp >= today)
    })
}

/// Serialize a full (unmasked) person to JSON for GDPR data export.
///
/// Supports the data-subject right of access. Returns
/// [`serde_json::Value::Null`] only if serialization unexpectedly fails.
#[must_use]
pub fn export_person_data(person: &Person) -> serde_json::Value {
    serde_json::to_value(person).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// Masking hides the alphanumeric prefix but keeps separators and tail.
    #[test]
    fn test_mask_value() {
        assert_eq!(mask_value("123-45-6789", 4), "***-**-6789");
        assert_eq!(mask_value("AB12345", 4), "***2345");
        assert_eq!(mask_value("short", 10), "short");
    }

    /// Masking is char-based, not byte-based: a value whose tail-cut byte
    /// boundary would split a multibyte UTF-8 char must not panic, and must
    /// keep exactly the last `visible_chars` *characters* visible.
    /// Regression test for the byte-indexed `mask_value` that panicked on
    /// `"1é345"` (cut at byte 2, inside `é`).
    #[test]
    fn test_mask_value_multibyte_does_not_panic() {
        // 'é' is two UTF-8 bytes; the naive byte cut at len-4 lands inside it.
        assert_eq!(mask_value("1é345", 4), "*é345");
        // Wholly non-ASCII prefix: alphanumerics masked, last 4 chars kept.
        assert_eq!(mask_value("naïve12", 4), "***ve12");
        // Char count, not byte count, drives the short-circuit.
        assert_eq!(mask_value("café", 4), "café");
        // A long accented value is masked end-to-end without panicking.
        assert_eq!(mask_value("Müller-9981", 4), "******-9981");
    }

    /// `mask_person` redacts tax ID and SSN but leaves the family name.
    #[test]
    fn test_mask_person() {
        use crate::models::*;

        let mut person = Person::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        person.tax_id = Some("123-45-6789".into());
        person
            .identifiers
            .push(Identifier::ssn("123-45-6789".into()));

        let masked = mask_person(&person);
        assert_eq!(masked.tax_id.as_deref(), Some("***-**-6789"));
        assert_eq!(masked.identifiers[0].value, "***-**-6789");
        // Family name should NOT be masked
        assert_eq!(masked.name.family, "Smith");
    }

    /// Masking an email keeps the last 4 chars and inserts `*`.
    #[test]
    fn test_mask_email() {
        // mask_value on an email-like string
        let masked = mask_value("john.doe@example.com", 4);
        assert_eq!(
            &masked[masked.len() - 4..],
            ".com",
            "Should keep last 4 chars visible, got {masked}"
        );
        assert!(masked.contains('*'), "Should contain masked characters");
    }

    /// Masking a phone keeps the last 4 digits visible.
    #[test]
    fn test_mask_phone() {
        let masked = mask_value("+1-555-123-4567", 4);
        assert!(
            masked.ends_with("4567"),
            "Last 4 digits should be visible, got {masked}"
        );
    }

    /// An SSN masks to `***-**-6789`.
    #[test]
    fn test_mask_ssn() {
        let masked = mask_value("123-45-6789", 4);
        assert_eq!(masked, "***-**-6789");
    }

    /// Values at or under the visible length are returned unchanged.
    #[test]
    fn test_mask_short_value() {
        // Value shorter than visible_chars should be returned as-is
        assert_eq!(mask_value("AB", 4), "AB");
        assert_eq!(mask_value("", 4), "");
        assert_eq!(mask_value("ABCD", 4), "ABCD");
    }

    /// The GDPR export contains all core person fields.
    #[test]
    fn test_export_person_data_includes_all_fields() {
        use crate::models::*;

        let mut person = Person::new(
            HumanName {
                use_type: None,
                family: "Doe".into(),
                given: vec!["Jane".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        );
        person.tax_id = Some("987-65-4321".into());
        person.birth_date = Some(NaiveDate::from_ymd_opt(1990, 5, 20).unwrap());

        let export = export_person_data(&person);
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

    /// An active, unexpired consent is detected.
    #[test]
    fn test_consent_active_check() {
        use crate::models::{Consent, ConsentStatus, ConsentType};

        let consent = Consent {
            id: uuid::Uuid::new_v4(),
            person_id: uuid::Uuid::new_v4(),
            consent_type: ConsentType::DataProcessing,
            status: ConsentStatus::Active,
            granted_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            expiry_date: Some(NaiveDate::from_ymd_opt(2099, 12, 31).unwrap()),
            revoked_date: None,
            purpose: Some("General data processing".into()),
            method: Some("electronic".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(has_active_consent(&[consent], &ConsentType::DataProcessing));
    }

    /// A past-expiry consent is not considered active.
    #[test]
    fn test_consent_expired_check() {
        use crate::models::{Consent, ConsentStatus, ConsentType};

        let expired_consent = Consent {
            id: uuid::Uuid::new_v4(),
            person_id: uuid::Uuid::new_v4(),
            consent_type: ConsentType::Marketing,
            status: ConsentStatus::Active,
            granted_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            expiry_date: Some(NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()), // expired
            revoked_date: None,
            purpose: None,
            method: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(
            !has_active_consent(&[expired_consent], &ConsentType::Marketing),
            "Expired consent should not be considered active"
        );
    }
}
