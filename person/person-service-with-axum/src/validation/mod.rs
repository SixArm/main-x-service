//! Data-quality validation and normalization for person records.
//!
//! The REST create/update handlers call [`validate_person`](crate::validation::validate_person) at the
//! request boundary and reject (HTTP 422) any person that returns a
//! non-empty [`ValidationError`](crate::validation::ValidationError) list. The same boundary also
//! *normalizes* inputs — [`normalize_phone`](crate::validation::normalize_phone) produces an E.164-like
//! string and [`standardize_address`](crate::validation::standardize_address) title-cases the locality,
//! uppercases region/country, and expands street-type abbreviations.
//!
//! All functions here are pure (no I/O), so they are cheap to unit test
//! and can be reused outside the HTTP layer.
//!
//! # Examples
//!
//! ```
//! use person_service::validation::normalize_phone;
//!
//! assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
//! ```

use crate::models::{
    Address, ContactPoint, ContactPointSystem, HumanName, IdentityDocument, Person,
};
use chrono::Utc;

/// A single validation failure: the offending field path and a
/// human-readable message.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationError {
    /// Dotted/indexed path to the field (e.g. `"telecom[0].value"`).
    pub field: String,
    /// Human-readable explanation suitable for an API response.
    pub message: String,
}

/// SEC-M1 — maximum length, in Unicode scalar values (`.chars().count()`),
/// of any single scalar text field. Bounds the per-field cost of the
/// matcher's character-level string comparisons (Jaro-Winkler /
/// Levenshtein) so one huge string cannot be a CPU/memory `DoS` (amplified
/// across the `check-duplicates` / `deduplicate` scan).
const MAX_TEXT_LEN: usize = 1024;
/// SEC-M1 — maximum number of entries in any array field. Bounds the
/// O(n·m) Jaccard / overlap work the matcher does over arrays.
const MAX_ARRAY_LEN: usize = 256;
/// SEC-M1 — maximum length of any single string entry inside an array.
const MAX_ITEM_LEN: usize = 512;

/// SEC-M1: push an error when a scalar text `field` exceeds [`MAX_TEXT_LEN`].
fn cap_text(errs: &mut Vec<ValidationError>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        errs.push(ValidationError {
            field: field.to_string(),
            message: format!("must not exceed {MAX_TEXT_LEN} characters"),
        });
    }
}

/// [`cap_text`] for an optional field; a no-op when absent.
fn cap_opt_text(errs: &mut Vec<ValidationError>, field: &str, value: Option<&String>) {
    if let Some(v) = value {
        cap_text(errs, field, v);
    }
}

/// SEC-M1: push an error when an array `field` holds more than
/// [`MAX_ARRAY_LEN`] entries.
fn cap_array(errs: &mut Vec<ValidationError>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        errs.push(ValidationError {
            field: field.to_string(),
            message: format!("must not exceed {MAX_ARRAY_LEN} entries"),
        });
    }
}

/// SEC-M1: push an error when the `index`-th entry of array `field` exceeds
/// [`MAX_ITEM_LEN`]; the field is reported as `field[index]`.
fn cap_item(errs: &mut Vec<ValidationError>, field: &str, index: usize, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        errs.push(ValidationError {
            field: format!("{field}[{index}]"),
            message: format!("must not exceed {MAX_ITEM_LEN} characters"),
        });
    }
}

/// SEC-M1: cap the cardinality of a string array and the length of each
/// entry — the common shape for the person's `Vec<String>` fields.
fn cap_string_array(errs: &mut Vec<ValidationError>, field: &str, values: &[String]) {
    cap_array(errs, field, values.len());
    for (i, v) in values.iter().enumerate() {
        cap_item(errs, field, i, v);
    }
}

/// SEC-M1: apply the caps to a [`HumanName`]'s scalar + array fields
/// (used for both the primary `name` and each `additional_names` entry).
fn cap_human_name(errs: &mut Vec<ValidationError>, prefix: &str, name: &HumanName) {
    cap_text(errs, &format!("{prefix}.family"), &name.family);
    cap_string_array(errs, &format!("{prefix}.given"), &name.given);
    cap_string_array(errs, &format!("{prefix}.prefix"), &name.prefix);
    cap_string_array(errs, &format!("{prefix}.suffix"), &name.suffix);
}

/// SEC-M1: cap each optional text component of an [`Address`].
fn cap_address(errs: &mut Vec<ValidationError>, prefix: &str, addr: &Address) {
    cap_opt_text(errs, &format!("{prefix}.line1"), addr.line1.as_ref());
    cap_opt_text(errs, &format!("{prefix}.line2"), addr.line2.as_ref());
    cap_opt_text(errs, &format!("{prefix}.city"), addr.city.as_ref());
    cap_opt_text(errs, &format!("{prefix}.state"), addr.state.as_ref());
    cap_opt_text(
        errs,
        &format!("{prefix}.postal_code"),
        addr.postal_code.as_ref(),
    );
    cap_opt_text(errs, &format!("{prefix}.country"), addr.country.as_ref());
}

/// SEC-M1: cap a telecom list's cardinality and each contact `value`.
fn cap_contacts(errs: &mut Vec<ValidationError>, prefix: &str, telecom: &[ContactPoint]) {
    cap_array(errs, prefix, telecom.len());
    for (i, cp) in telecom.iter().enumerate() {
        cap_text(errs, &format!("{prefix}[{i}].value"), &cp.value);
    }
}

/// SEC-M1: apply the input-size caps to every scalar text field and array
/// reachable on a [`Person`]. Split out of [`validate_person`] to keep that
/// function within clippy's `too_many_lines` budget.
fn person_size_caps(errs: &mut Vec<ValidationError>, person: &Person) {
    // Names
    cap_human_name(errs, "name", &person.name);
    cap_array(errs, "additional_names", person.additional_names.len());
    for (i, n) in person.additional_names.iter().enumerate() {
        cap_human_name(errs, &format!("additional_names[{i}]"), n);
    }

    // Scalar optionals
    cap_opt_text(errs, "tax_id", person.tax_id.as_ref());
    cap_opt_text(errs, "marital_status", person.marital_status.as_ref());

    // Photo references
    cap_string_array(errs, "photo", &person.photo);

    // Telecom
    cap_contacts(errs, "telecom", &person.telecom);

    // Identifiers
    cap_array(errs, "identifiers", person.identifiers.len());
    for (i, id) in person.identifiers.iter().enumerate() {
        cap_text(errs, &format!("identifiers[{i}].system"), &id.system);
        cap_text(errs, &format!("identifiers[{i}].value"), &id.value);
        cap_opt_text(
            errs,
            &format!("identifiers[{i}].assigner"),
            id.assigner.as_ref(),
        );
    }

    // Addresses
    cap_array(errs, "addresses", person.addresses.len());
    for (i, addr) in person.addresses.iter().enumerate() {
        cap_address(errs, &format!("addresses[{i}]"), addr);
    }

    // Documents
    cap_array(errs, "documents", person.documents.len());
    for (i, doc) in person.documents.iter().enumerate() {
        cap_text(errs, &format!("documents[{i}].number"), &doc.number);
        cap_opt_text(
            errs,
            &format!("documents[{i}].issuing_country"),
            doc.issuing_country.as_ref(),
        );
        cap_opt_text(
            errs,
            &format!("documents[{i}].issuing_authority"),
            doc.issuing_authority.as_ref(),
        );
    }

    // Emergency contacts
    cap_array(errs, "emergency_contacts", person.emergency_contacts.len());
    for (i, ec) in person.emergency_contacts.iter().enumerate() {
        cap_text(errs, &format!("emergency_contacts[{i}].name"), &ec.name);
        cap_text(
            errs,
            &format!("emergency_contacts[{i}].relationship"),
            &ec.relationship,
        );
        cap_contacts(
            errs,
            &format!("emergency_contacts[{i}].telecom"),
            &ec.telecom,
        );
        if let Some(addr) = ec.address.as_ref() {
            cap_address(errs, &format!("emergency_contacts[{i}].address"), addr);
        }
    }
}

/// Validate a person record, returning every error found.
///
/// Checks required name fields, a non-future birth date, tax-ID format,
/// and recurses into telecom, addresses, documents, and emergency
/// contacts. An empty result means the record is valid.
///
/// # Examples
///
/// ```
/// use person_service::models::{Person, HumanName, Gender};
/// use person_service::validation::validate_person;
///
/// let ok = Person::new(
///     HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] },
///     Gender::Male,
/// );
/// assert!(validate_person(&ok).is_empty());
/// ```
#[must_use]
pub fn validate_person(person: &Person) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Required: family name
    if person.name.family.trim().is_empty() {
        errors.push(ValidationError {
            field: "name.family".into(),
            message: "Family name is required".into(),
        });
    }

    // Required: at least one given name
    if person.name.given.is_empty() || person.name.given.iter().all(|g| g.trim().is_empty()) {
        errors.push(ValidationError {
            field: "name.given".into(),
            message: "At least one given name is required".into(),
        });
    }

    // Validate birth_date is not in the future
    if let Some(dob) = person.birth_date
        && dob > Utc::now().date_naive()
    {
        errors.push(ValidationError {
            field: "birth_date".into(),
            message: "Birth date cannot be in the future".into(),
        });
    }

    // Validate tax_id format if present
    if let Some(ref tid) = person.tax_id {
        let cleaned: String = tid.chars().filter(char::is_ascii_alphanumeric).collect();
        if cleaned.is_empty() {
            errors.push(ValidationError {
                field: "tax_id".into(),
                message: "Tax ID must contain at least one alphanumeric character".into(),
            });
        }
    }

    // Validate contact points
    for (i, cp) in person.telecom.iter().enumerate() {
        errors.extend(validate_contact_point(cp, &format!("telecom[{i}]")));
    }

    // Validate addresses
    for (i, addr) in person.addresses.iter().enumerate() {
        errors.extend(validate_address(addr, &format!("addresses[{i}]")));
    }

    // Validate documents
    for (i, doc) in person.documents.iter().enumerate() {
        errors.extend(validate_document(doc, &format!("documents[{i}]")));
    }

    // Validate emergency contacts
    for (i, ec) in person.emergency_contacts.iter().enumerate() {
        if ec.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("emergency_contacts[{i}].name"),
                message: "Emergency contact name is required".into(),
            });
        }
        if ec.relationship.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("emergency_contacts[{i}].relationship"),
                message: "Emergency contact relationship is required".into(),
            });
        }
    }

    // SEC-M1 input-size caps (scalar text + array cardinality + per-entry).
    person_size_caps(&mut errors, person);

    errors
}

/// Validate a single contact point under the given field `prefix`.
///
/// The value must be non-empty; emails must contain `@` and `.`; phone,
/// SMS, and fax values must carry at least seven digits.
fn validate_contact_point(cp: &ContactPoint, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if cp.value.trim().is_empty() {
        errors.push(ValidationError {
            field: format!("{prefix}.value"),
            message: "Contact value is required".into(),
        });
        return errors;
    }

    match cp.system {
        ContactPointSystem::Email if (!cp.value.contains('@') || !cp.value.contains('.')) => {
            errors.push(ValidationError {
                field: format!("{prefix}.value"),
                message: "Invalid email format".into(),
            });
        }
        ContactPointSystem::Phone | ContactPointSystem::Sms | ContactPointSystem::Fax => {
            let digits: String = cp.value.chars().filter(char::is_ascii_digit).collect();
            if digits.len() < 7 {
                errors.push(ValidationError {
                    field: format!("{prefix}.value"),
                    message: "Phone number must have at least 7 digits".into(),
                });
            }
        }
        _ => {}
    }

    errors
}

/// Validate an address under the given field `prefix`.
///
/// Requires at least one of city, postal code, or country to be present
/// and non-blank.
fn validate_address(addr: &Address, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // At minimum, a country or postal code should be present
    let has_location = addr.city.as_ref().is_some_and(|s| !s.trim().is_empty())
        || addr
            .postal_code
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        || addr.country.as_ref().is_some_and(|s| !s.trim().is_empty());

    if !has_location {
        errors.push(ValidationError {
            field: prefix.to_string(),
            message: "Address must have at least a city, postal code, or country".into(),
        });
    }

    errors
}

/// Validate an identity document under the given field `prefix`.
///
/// Requires a non-empty number, rejects an already-expired document,
/// and rejects an issue date later than the expiry date.
fn validate_document(doc: &IdentityDocument, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if doc.number.trim().is_empty() {
        errors.push(ValidationError {
            field: format!("{prefix}.number"),
            message: "Document number is required".into(),
        });
    }

    // Check expiry
    if let Some(expiry) = doc.expiry_date
        && expiry < Utc::now().date_naive()
    {
        errors.push(ValidationError {
            field: format!("{prefix}.expiry_date"),
            message: "Document has expired".into(),
        });
    }

    // Check issue date before expiry date
    if let (Some(issue), Some(expiry)) = (doc.issue_date, doc.expiry_date)
        && issue > expiry
    {
        errors.push(ValidationError {
            field: format!("{prefix}.issue_date"),
            message: "Issue date cannot be after expiry date".into(),
        });
    }

    errors
}

/// Normalize a phone number to an E.164-like `+<digits>` string.
///
/// Strips all non-digit characters, then prepends
/// `default_country_code` when the number appears to lack one (e.g. a
/// bare 10-digit US number). Returns an empty string for input with no
/// digits.
///
/// # Examples
///
/// ```
/// use person_service::validation::normalize_phone;
///
/// assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
/// assert_eq!(normalize_phone("", "1"), "");
/// ```
#[must_use]
pub fn normalize_phone(phone: &str, default_country_code: &str) -> String {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();

    if digits.is_empty() {
        return String::new();
    }

    // If already has a country code (10+ digits starting with country code)
    if digits.len() >= 10 && digits.starts_with(default_country_code) {
        return format!("+{digits}");
    }

    // If exactly 10 digits (US format), prepend country code
    if digits.len() == 10 {
        return format!("+{default_country_code}{digits}");
    }

    // If starts with +, keep as-is but clean
    if phone.starts_with('+') {
        return format!("+{digits}");
    }

    // Return cleaned digits
    format!("+{default_country_code}{digits}")
}

/// Return a standardized copy of an address.
///
/// Trims whitespace, expands street-type abbreviations on line 1,
/// title-cases the city, and uppercases the state and country. The
/// `use_type` and postal code pass through unchanged.
///
/// # Examples
///
/// ```
/// use person_service::models::Address;
/// use person_service::validation::standardize_address;
///
/// let addr = Address {
///     use_type: None,
///     line1: Some("123 main st.".into()),
///     line2: None,
///     city: Some("new york".into()),
///     state: Some("ny".into()),
///     postal_code: Some("10001".into()),
///     country: Some("us".into()),
/// };
/// let std = standardize_address(&addr);
/// assert_eq!(std.city.as_deref(), Some("New York"));
/// assert_eq!(std.state.as_deref(), Some("NY"));
/// ```
#[must_use]
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

/// Expand common street-type abbreviations (`St.`→`Street`,
/// `Ave.`→`Avenue`, …) after trimming. Used by [`standardize_address`].
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

/// Title-case each whitespace-separated word (first letter upper, rest
/// lower). Used by [`standardize_address`] for the city field.
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{upper}{rest}")
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
    use chrono::NaiveDate;

    /// A blank family name produces a `name.family` error.
    #[test]
    fn test_validate_missing_family_name() {
        let person = Person::new(
            HumanName {
                use_type: None,
                family: String::new(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let errors = validate_person(&person);
        assert!(errors.iter().any(|e| e.field == "name.family"));
    }

    /// A well-formed person produces no errors.
    #[test]
    fn test_validate_valid_person() {
        let person = Person::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let errors = validate_person(&person);
        assert!(errors.is_empty());
    }

    /// US 10-digit and +1-prefixed numbers normalize identically.
    #[test]
    fn test_normalize_phone_us() {
        assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
        assert_eq!(normalize_phone("+1-555-123-4567", "1"), "+15551234567");
    }

    /// Standardization title-cases city and uppercases state/country.
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

    /// A future birth date produces a `birth_date` error.
    #[test]
    fn test_validate_future_birth_date() {
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
        // Set birth date to far in the future
        person.birth_date = Some(NaiveDate::from_ymd_opt(2099, 1, 1).unwrap());
        let errors = validate_person(&person);
        assert!(
            errors.iter().any(|e| e.field == "birth_date"),
            "Future birth date should produce validation error"
        );
    }

    /// An email lacking `@`/`.` produces a telecom error.
    #[test]
    fn test_validate_invalid_email() {
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
        person.telecom.push(ContactPoint {
            system: ContactPointSystem::Email,
            value: "not-an-email".into(),
            use_type: None,
        });
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("telecom") && e.message.contains("email")),
            "Invalid email should produce validation error"
        );
    }

    /// A phone with fewer than 7 digits produces a telecom error.
    #[test]
    fn test_validate_invalid_phone() {
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
        person.telecom.push(ContactPoint {
            system: ContactPointSystem::Phone,
            value: "123".into(),
            use_type: None,
        });
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("telecom") && e.message.contains("7 digits")),
            "Short phone number should produce validation error"
        );
    }

    /// A tax ID with no alphanumerics produces a `tax_id` error.
    #[test]
    fn test_validate_tax_id_format() {
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
        person.tax_id = Some("---".into()); // No alphanumeric chars
        let errors = validate_person(&person);
        assert!(
            errors.iter().any(|e| e.field == "tax_id"),
            "Tax ID with no alphanumeric chars should fail"
        );
    }

    /// An empty document number produces a `number` error.
    #[test]
    fn test_validate_document_missing_number() {
        use crate::models::{DocumentType, IdentityDocument};
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
        person.documents.push(IdentityDocument {
            document_type: DocumentType::Passport,
            number: String::new(),
            issuing_country: Some("US".into()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: false,
        });
        let errors = validate_person(&person);
        assert!(
            errors.iter().any(|e| e.field.contains("number")),
            "Empty document number should fail"
        );
    }

    /// A past expiry date produces an "expired" error.
    #[test]
    fn test_validate_document_expired() {
        use crate::models::{DocumentType, IdentityDocument};
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
        person.documents.push(IdentityDocument {
            document_type: DocumentType::Passport,
            number: "X12345678".into(),
            issuing_country: Some("US".into()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            verified: false,
        });
        let errors = validate_person(&person);
        assert!(
            errors.iter().any(|e| e.message.contains("expired")),
            "Expired document should produce error"
        );
    }

    /// A blank emergency-contact name produces an error.
    #[test]
    fn test_validate_emergency_contact_missing_name() {
        use crate::models::EmergencyContact;
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
        person.emergency_contacts.push(EmergencyContact {
            name: String::new(),
            relationship: "spouse".into(),
            telecom: vec![],
            address: None,
            is_primary: true,
        });
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("emergency_contacts") && e.message.contains("name")),
            "Missing emergency contact name should produce error"
        );
    }

    /// An address with only a street line produces an error.
    #[test]
    fn test_validate_address_incomplete() {
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
        person.addresses.push(Address {
            use_type: None,
            line1: Some("123 Main St".into()),
            line2: None,
            city: None,
            state: None,
            postal_code: None,
            country: None,
        });
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("addresses") && e.message.contains("city")),
            "Address without city/postal/country should produce error"
        );
    }

    /// A number already carrying its country code is preserved.
    #[test]
    fn test_normalize_phone_international() {
        // 11 digits starting with country code
        assert_eq!(normalize_phone("+44 20 7946 0958", "44"), "+442079460958");
    }

    /// Extension text is dropped, leaving only `+` and digits.
    #[test]
    fn test_normalize_phone_with_extensions() {
        // Extensions should be stripped (only digits kept)
        let result = normalize_phone("555-123-4567 ext. 100", "1");
        assert!(
            result.starts_with('+'),
            "Normalized phone should start with +"
        );
        assert!(
            result.chars().skip(1).all(|c| c.is_ascii_digit()),
            "Should contain only digits after +"
        );
    }

    /// `Ave.` expands to `Avenue` during standardization.
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
        assert!(
            std.line1.as_ref().unwrap().contains("Avenue"),
            "Ave. should expand to Avenue, got {:?}",
            std.line1
        );
    }

    /// City title-cases while state/country uppercase.
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

    /// SEC-M1: an oversized scalar text field (family name over
    /// `MAX_TEXT_LEN`) yields a cap error on that exact field.
    #[test]
    fn test_cap_oversized_scalar_text() {
        let person = Person::new(
            HumanName {
                use_type: None,
                family: "x".repeat(MAX_TEXT_LEN + 1),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "name.family" && e.message.contains("exceed")),
            "Oversized family name should produce a cap error on name.family"
        );
    }

    /// SEC-M1: an over-long array (photo cardinality over `MAX_ARRAY_LEN`)
    /// yields a cardinality error on the array field.
    #[test]
    fn test_cap_over_long_array() {
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
        person.photo = vec!["ok".to_string(); MAX_ARRAY_LEN + 1];
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "photo" && e.message.contains("entries")),
            "Over-long photo array should produce a cardinality cap error"
        );
    }

    /// SEC-M1: an oversized single array entry yields an indexed error.
    #[test]
    fn test_cap_oversized_array_entry() {
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
        person.photo = vec!["ok".into(), "x".repeat(MAX_ITEM_LEN + 1)];
        let errors = validate_person(&person);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "photo[1]" && e.message.contains("exceed")),
            "Oversized photo entry should produce an indexed cap error"
        );
    }

    /// SEC-M1: a large-but-within-caps record (fields exactly at their
    /// limits) yields NO cap errors.
    #[test]
    fn test_cap_within_limits_no_error() {
        let mut person = Person::new(
            HumanName {
                use_type: None,
                family: "x".repeat(MAX_TEXT_LEN),
                given: vec!["g".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        person.tax_id = Some("t".repeat(MAX_TEXT_LEN));
        person.marital_status = Some("m".repeat(MAX_TEXT_LEN));
        person.photo = vec!["p".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN];
        let errors = validate_person(&person);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("exceed") || e.message.contains("entries")),
            "A record with fields exactly at their caps should produce no cap errors, got {errors:?}"
        );
    }
}
