//! Data quality validation for worker records
//!
//! Provides validation rules, address standardization,
//! phone number formatting, and document validation.

use crate::models::{Worker, Address, ContactPoint, ContactPointSystem, IdentityDocument};

/// Validation error with field path and message
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Validate a worker record, returning all validation errors found.
pub fn validate_worker(worker: &Worker) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Required: family name
    if worker.name.family.trim().is_empty() {
        errors.push(ValidationError {
            field: "name.family".into(),
            message: "Family name is required".into(),
        });
    }

    // Required: at least one given name
    if worker.name.given.is_empty() || worker.name.given.iter().all(|g| g.trim().is_empty()) {
        errors.push(ValidationError {
            field: "name.given".into(),
            message: "At least one given name is required".into(),
        });
    }

    // Validate birth_date is not in the future
    if let Some(dob) = worker.birth_date {
        if dob > chrono::Utc::now().date_naive() {
            errors.push(ValidationError {
                field: "birth_date".into(),
                message: "Birth date cannot be in the future".into(),
            });
        }
    }

    // Validate tax_id format if present
    if let Some(ref tid) = worker.tax_id {
        let cleaned: String = tid.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if cleaned.is_empty() {
            errors.push(ValidationError {
                field: "tax_id".into(),
                message: "Tax ID must contain at least one alphanumeric character".into(),
            });
        }
    }

    // Validate contact points
    for (i, cp) in worker.telecom.iter().enumerate() {
        errors.extend(validate_contact_point(cp, &format!("telecom[{}]", i)));
    }

    // Validate addresses
    for (i, addr) in worker.addresses.iter().enumerate() {
        errors.extend(validate_address(addr, &format!("addresses[{}]", i)));
    }

    // Validate documents
    for (i, doc) in worker.documents.iter().enumerate() {
        errors.extend(validate_document(doc, &format!("documents[{}]", i)));
    }

    // Validate emergency contacts
    for (i, ec) in worker.emergency_contacts.iter().enumerate() {
        if ec.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("emergency_contacts[{}].name", i),
                message: "Emergency contact name is required".into(),
            });
        }
        if ec.relationship.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("emergency_contacts[{}].relationship", i),
                message: "Emergency contact relationship is required".into(),
            });
        }
    }

    errors
}

/// Validate a contact point
fn validate_contact_point(cp: &ContactPoint, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if cp.value.trim().is_empty() {
        errors.push(ValidationError {
            field: format!("{}.value", prefix),
            message: "Contact value is required".into(),
        });
        return errors;
    }

    match cp.system {
        ContactPointSystem::Email => {
            if !cp.value.contains('@') || !cp.value.contains('.') {
                errors.push(ValidationError {
                    field: format!("{}.value", prefix),
                    message: "Invalid email format".into(),
                });
            }
        }
        ContactPointSystem::Phone | ContactPointSystem::Sms | ContactPointSystem::Fax => {
            let digits: String = cp.value.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() < 7 {
                errors.push(ValidationError {
                    field: format!("{}.value", prefix),
                    message: "Phone number must have at least 7 digits".into(),
                });
            }
        }
        _ => {}
    }

    errors
}

/// Validate an address
fn validate_address(addr: &Address, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // At minimum, a country or postal code should be present
    let has_location = addr.city.as_ref().is_some_and(|s| !s.trim().is_empty())
        || addr.postal_code.as_ref().is_some_and(|s| !s.trim().is_empty())
        || addr.country.as_ref().is_some_and(|s| !s.trim().is_empty());

    if !has_location {
        errors.push(ValidationError {
            field: format!("{}", prefix),
            message: "Address must have at least a city, postal code, or country".into(),
        });
    }

    errors
}

/// Validate an identity document
fn validate_document(doc: &IdentityDocument, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if doc.number.trim().is_empty() {
        errors.push(ValidationError {
            field: format!("{}.number", prefix),
            message: "Document number is required".into(),
        });
    }

    // Check expiry
    if let Some(expiry) = doc.expiry_date {
        if expiry < chrono::Utc::now().date_naive() {
            errors.push(ValidationError {
                field: format!("{}.expiry_date", prefix),
                message: "Document has expired".into(),
            });
        }
    }

    // Check issue date before expiry date
    if let (Some(issue), Some(expiry)) = (doc.issue_date, doc.expiry_date) {
        if issue > expiry {
            errors.push(ValidationError {
                field: format!("{}.issue_date", prefix),
                message: "Issue date cannot be after expiry date".into(),
            });
        }
    }

    errors
}

/// Normalize/standardize a phone number to E.164-like format.
/// Strips non-digit characters and prepends country code if missing.
pub fn normalize_phone(phone: &str, default_country_code: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.is_empty() {
        return String::new();
    }

    // If already has a country code (10+ digits starting with country code)
    if digits.len() >= 10 && digits.starts_with(default_country_code) {
        return format!("+{}", digits);
    }

    // If exactly 10 digits (US format), prepend country code
    if digits.len() == 10 {
        return format!("+{}{}", default_country_code, digits);
    }

    // If starts with +, keep as-is but clean
    if phone.starts_with('+') {
        return format!("+{}", digits);
    }

    // Return cleaned digits
    format!("+{}{}", default_country_code, digits)
}

/// Standardize an address (trim whitespace, normalize casing, expand abbreviations)
pub fn standardize_address(addr: &Address) -> Address {
    Address {
        use_type: addr.use_type.clone(),
        line1: addr.line1.as_ref().map(|s| normalize_street_address(s)),
        line2: addr.line2.as_ref().map(|s| s.trim().to_string()),
        city: addr.city.as_ref().map(|s| title_case(s.trim())),
        state: addr.state.as_ref().map(|s| s.trim().to_uppercase()),
        postal_code: addr.postal_code.as_ref().map(|s| s.trim().to_string()),
        country: addr.country.as_ref().map(|s| s.trim().to_uppercase()),
    }
}

fn normalize_street_address(street: &str) -> String {
    let s = street.trim().to_string();
    // Expand common abbreviations
    s.replace("St.", "Street")
        .replace("St ", "Street ")
        .replace("Ave.", "Avenue")
        .replace("Ave ", "Avenue ")
        .replace("Rd.", "Road")
        .replace("Rd ", "Road ")
        .replace("Dr.", "Drive")
        .replace("Blvd.", "Boulevard")
        .replace("Ln.", "Lane")
        .replace("Ct.", "Court")
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{}{}", upper, rest)
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName};

    #[test]
    fn test_validate_missing_family_name() {
        let worker = Worker::new(
            HumanName { use_type: None, family: "".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field == "name.family"));
    }

    #[test]
    fn test_validate_valid_worker() {
        let worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        let errors = validate_worker(&worker);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_normalize_phone_us() {
        assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
        assert_eq!(normalize_phone("+1-555-123-4567", "1"), "+15551234567");
    }

    #[test]
    fn test_standardize_address() {
        let addr = Address {
            use_type: None,
            line1: Some("123 main st.".into()),
            line2: None,
            city: Some("new york".into()),
            state: Some("ny".into()),
            postal_code: Some("10001".into()),
            country: Some("us".into()),
        };
        let std = standardize_address(&addr);
        assert_eq!(std.city.as_deref(), Some("New York"));
        assert_eq!(std.state.as_deref(), Some("NY"));
        assert_eq!(std.country.as_deref(), Some("US"));
    }

    #[test]
    fn test_validate_future_birth_date() {
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        // Set birth date to far in the future
        worker.birth_date = Some(chrono::NaiveDate::from_ymd_opt(2099, 1, 1).unwrap());
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field == "birth_date"), "Future birth date should produce validation error");
    }

    #[test]
    fn test_validate_invalid_email() {
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.telecom.push(ContactPoint {
            system: ContactPointSystem::Email,
            value: "not-an-email".into(),
            use_type: None,
        });
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field.contains("telecom") && e.message.contains("email")),
            "Invalid email should produce validation error");
    }

    #[test]
    fn test_validate_invalid_phone() {
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.telecom.push(ContactPoint {
            system: ContactPointSystem::Phone,
            value: "123".into(),
            use_type: None,
        });
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field.contains("telecom") && e.message.contains("7 digits")),
            "Short phone number should produce validation error");
    }

    #[test]
    fn test_validate_tax_id_format() {
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.tax_id = Some("---".into()); // No alphanumeric chars
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field == "tax_id"), "Tax ID with no alphanumeric chars should fail");
    }

    #[test]
    fn test_validate_document_missing_number() {
        use crate::models::{IdentityDocument, DocumentType};
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.documents.push(IdentityDocument {
            document_type: DocumentType::Passport,
            number: "".into(),
            issuing_country: Some("US".into()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: false,
        });
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field.contains("number")), "Empty document number should fail");
    }

    #[test]
    fn test_validate_document_expired() {
        use crate::models::{IdentityDocument, DocumentType};
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.documents.push(IdentityDocument {
            document_type: DocumentType::Passport,
            number: "X12345678".into(),
            issuing_country: Some("US".into()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            verified: false,
        });
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.message.contains("expired")), "Expired document should produce error");
    }

    #[test]
    fn test_validate_emergency_contact_missing_name() {
        use crate::models::EmergencyContact;
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.emergency_contacts.push(EmergencyContact {
            name: "".into(),
            relationship: "spouse".into(),
            telecom: vec![],
            address: None,
            is_primary: true,
        });
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field.contains("emergency_contacts") && e.message.contains("name")),
            "Missing emergency contact name should produce error");
    }

    #[test]
    fn test_validate_address_incomplete() {
        let mut worker = Worker::new(
            HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
            Gender::Male,
        );
        worker.addresses.push(Address {
            use_type: None,
            line1: Some("123 Main St".into()),
            line2: None,
            city: None,
            state: None,
            postal_code: None,
            country: None,
        });
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field.contains("addresses") && e.message.contains("city")),
            "Address without city/postal/country should produce error");
    }

    #[test]
    fn test_normalize_phone_international() {
        // 11 digits starting with country code
        assert_eq!(normalize_phone("+44 20 7946 0958", "44"), "+442079460958");
    }

    #[test]
    fn test_normalize_phone_with_extensions() {
        // Extensions should be stripped (only digits kept)
        let result = normalize_phone("555-123-4567 ext. 100", "1");
        assert!(result.starts_with('+'), "Normalized phone should start with +");
        assert!(result.chars().skip(1).all(|c| c.is_ascii_digit()), "Should contain only digits after +");
    }

    #[test]
    fn test_standardize_address_abbreviations() {
        let addr = Address {
            use_type: None,
            line1: Some("100 Oak Ave.".into()),
            line2: None,
            city: Some("los angeles".into()),
            state: Some("ca".into()),
            postal_code: Some("90001".into()),
            country: Some("us".into()),
        };
        let std = standardize_address(&addr);
        assert!(std.line1.as_ref().unwrap().contains("Avenue"), "Ave. should expand to Avenue, got {:?}", std.line1);
    }

    #[test]
    fn test_standardize_address_case() {
        let addr = Address {
            use_type: None,
            line1: None,
            line2: None,
            city: Some("SAN FRANCISCO".into()),
            state: Some("california".into()),
            postal_code: None,
            country: Some("united states".into()),
        };
        let std = standardize_address(&addr);
        assert_eq!(std.city.as_deref(), Some("San Francisco"));
        assert_eq!(std.state.as_deref(), Some("CALIFORNIA"));
        assert_eq!(std.country.as_deref(), Some("UNITED STATES"));
    }
}
