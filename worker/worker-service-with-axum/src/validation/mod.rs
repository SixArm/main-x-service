//! Data-quality validation, normalization, and standardization for worker
//! records.
//!
//! [`validate_worker`](crate::validation::validate_worker) runs the full rule
//! set and returns every problem found (rather than failing fast) so the API
//! can report them all at once with a 422.
//! [`normalize_phone`](crate::validation::normalize_phone) and
//! [`standardize_address`](crate::validation::standardize_address) canonicalise
//! input at the service boundary so downstream matching sees consistent values.
//!
//! # Examples
//!
//! ```
//! use worker_service::validation::normalize_phone;
//!
//! assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
//! ```

use crate::models::assessment::{Assessment, AssessmentScale, AssessmentStatus};
use crate::models::{
    Address, ContactPoint, ContactPointSystem, EmergencyContact, HumanName, Identifier,
    IdentityDocument, Worker,
};

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single scalar text field (`name.family`, `tax_id`, `marital_status`, and
/// the text fields of nested structs). Bounds the per-field cost of the
/// matcher's character-level string comparisons (SEC-M1 input-size caps).
const MAX_TEXT_LEN: usize = 1024;

/// Maximum number of entries in any array field (`identifiers`, `telecom`,
/// `addresses`, `documents`, `emergency_contacts`, `additional_names`,
/// `links`, `photo`, and the name-component arrays). Bounds the O(n·m)
/// Jaccard / overlap work the matcher does over arrays (SEC-M1 input-size
/// caps).
const MAX_ARRAY_LEN: usize = 256;

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single string entry inside an array field (SEC-M1 input-size caps).
const MAX_ITEM_LEN: usize = 512;

/// A single validation failure: the dotted `field` path that failed and a
/// human-readable `message`. Serializable so it can be returned in API errors.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationError {
    /// Dotted/indexed path to the offending field (e.g. `"telecom[0].value"`).
    pub field: String,
    /// Human-readable description of what is wrong.
    pub message: String,
}

/// Validates a worker against all data-quality rules and returns every error
/// found (empty vec means valid).
///
/// Rules cover required name fields, a non-future birth date, tax-ID format,
/// and per-element checks of telecom, addresses, documents, and emergency
/// contacts.
///
/// # Examples
///
/// ```
/// use worker_service::validation::validate_worker;
/// use worker_service::models::{Worker, HumanName, Gender};
///
/// let w = Worker::new(
///     HumanName { use_type: None, family: "Smith".into(),
///         given: vec!["John".into()], prefix: vec![], suffix: vec![] },
///     Gender::Male,
/// );
/// assert!(validate_worker(&w).is_empty());
/// ```
#[must_use]
pub fn validate_worker(worker: &Worker) -> Vec<ValidationError> {
    // Collect every failure rather than returning on the first, so the API can
    // surface a complete 422 list in one round-trip.
    let mut errors = Vec::new();

    // Required: family name — the matcher and search index both key on it, so a
    // blank family name would leave a record effectively unfindable.
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
    if let Some(dob) = worker.birth_date
        && dob > chrono::Utc::now().date_naive()
    {
        errors.push(ValidationError {
            field: "birth_date".into(),
            message: "Birth date cannot be in the future".into(),
        });
    }

    // Validate tax_id format if present
    if let Some(ref tid) = worker.tax_id {
        let cleaned: String = tid.chars().filter(char::is_ascii_alphanumeric).collect();
        if cleaned.is_empty() {
            errors.push(ValidationError {
                field: "tax_id".into(),
                message: "Tax ID must contain at least one alphanumeric character".into(),
            });
        }
    }

    // Validate contact points
    for (i, cp) in worker.telecom.iter().enumerate() {
        errors.extend(validate_contact_point(cp, &format!("telecom[{i}]")));
    }

    // Validate addresses
    for (i, addr) in worker.addresses.iter().enumerate() {
        errors.extend(validate_address(addr, &format!("addresses[{i}]")));
    }

    // Validate documents
    for (i, doc) in worker.documents.iter().enumerate() {
        errors.extend(validate_document(doc, &format!("documents[{i}]")));
    }

    // Validate emergency contacts
    for (i, ec) in worker.emergency_contacts.iter().enumerate() {
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

    // Input-size caps (SEC-M1): bound every scalar text field, array
    // cardinality, and per-entry string length so a single huge string or
    // array cannot be used as a CPU/memory DoS against the matcher's O(n·m)
    // scoring (amplified across check-duplicates / deduplicate scans).
    worker_size_caps(&mut errors, worker);

    errors
}

/// Applies SEC-M1 input-size caps to every scalar text field and array
/// reachable on `worker`, pushing a [`ValidationError`] for each field that
/// exceeds [`MAX_TEXT_LEN`], [`MAX_ARRAY_LEN`], or [`MAX_ITEM_LEN`]. Factored
/// out of [`validate_worker`] to keep that function under the line cap.
fn worker_size_caps(errs: &mut Vec<ValidationError>, worker: &Worker) {
    // Top-level scalar text.
    cap_opt_text(errs, "tax_id", worker.tax_id.as_ref());
    cap_opt_text(errs, "marital_status", worker.marital_status.as_ref());

    // Names (primary + additional).
    cap_human_name(errs, "name", &worker.name);
    cap_array(errs, "additional_names", worker.additional_names.len());
    for (i, n) in worker.additional_names.iter().enumerate() {
        cap_human_name(errs, &format!("additional_names[{i}]"), n);
    }

    // Simple Vec<String> array.
    cap_string_array(errs, "photo", &worker.photo);

    // Struct arrays: cardinality + inner text.
    cap_array(errs, "identifiers", worker.identifiers.len());
    for (i, id) in worker.identifiers.iter().enumerate() {
        cap_identifier(errs, i, id);
    }
    cap_array(errs, "telecom", worker.telecom.len());
    for (i, cp) in worker.telecom.iter().enumerate() {
        cap_text(errs, &format!("telecom[{i}].value"), &cp.value);
    }
    cap_array(errs, "addresses", worker.addresses.len());
    for (i, a) in worker.addresses.iter().enumerate() {
        cap_address(errs, &format!("addresses[{i}]"), a);
    }
    cap_array(errs, "documents", worker.documents.len());
    for (i, d) in worker.documents.iter().enumerate() {
        cap_document(errs, i, d);
    }
    cap_array(errs, "emergency_contacts", worker.emergency_contacts.len());
    for (i, ec) in worker.emergency_contacts.iter().enumerate() {
        cap_emergency_contact(errs, i, ec);
    }

    // `links` carries only a Uuid + enum (no text); cap cardinality only.
    cap_array(errs, "links", worker.links.len());
}

/// Caps the text and component arrays of a [`HumanName`] under `prefix`.
fn cap_human_name(errs: &mut Vec<ValidationError>, prefix: &str, name: &HumanName) {
    cap_text(errs, &format!("{prefix}.family"), &name.family);
    cap_string_array(errs, &format!("{prefix}.given"), &name.given);
    cap_string_array(errs, &format!("{prefix}.prefix"), &name.prefix);
    cap_string_array(errs, &format!("{prefix}.suffix"), &name.suffix);
}

/// Caps the text fields of the `index`-th [`Identifier`].
fn cap_identifier(errs: &mut Vec<ValidationError>, index: usize, id: &Identifier) {
    cap_text(errs, &format!("identifiers[{index}].system"), &id.system);
    cap_text(errs, &format!("identifiers[{index}].value"), &id.value);
    cap_opt_text(
        errs,
        &format!("identifiers[{index}].assigner"),
        id.assigner.as_ref(),
    );
}

/// Caps the optional text fields of an [`Address`] under `prefix`.
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

/// Caps the text fields of the `index`-th [`IdentityDocument`].
fn cap_document(errs: &mut Vec<ValidationError>, index: usize, doc: &IdentityDocument) {
    cap_text(errs, &format!("documents[{index}].number"), &doc.number);
    cap_opt_text(
        errs,
        &format!("documents[{index}].issuing_country"),
        doc.issuing_country.as_ref(),
    );
    cap_opt_text(
        errs,
        &format!("documents[{index}].issuing_authority"),
        doc.issuing_authority.as_ref(),
    );
}

/// Caps the text fields (and nested telecom / address) of the `index`-th
/// [`EmergencyContact`].
fn cap_emergency_contact(errs: &mut Vec<ValidationError>, index: usize, ec: &EmergencyContact) {
    cap_text(errs, &format!("emergency_contacts[{index}].name"), &ec.name);
    cap_text(
        errs,
        &format!("emergency_contacts[{index}].relationship"),
        &ec.relationship,
    );
    cap_array(
        errs,
        &format!("emergency_contacts[{index}].telecom"),
        ec.telecom.len(),
    );
    for (j, cp) in ec.telecom.iter().enumerate() {
        cap_text(
            errs,
            &format!("emergency_contacts[{index}].telecom[{j}].value"),
            &cp.value,
        );
    }
    if let Some(addr) = &ec.address {
        cap_address(errs, &format!("emergency_contacts[{index}].address"), addr);
    }
}

/// Pushes an error when a scalar text `field` exceeds [`MAX_TEXT_LEN`] Unicode
/// scalar values.
fn cap_text(errs: &mut Vec<ValidationError>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        errs.push(ValidationError {
            field: field.to_string(),
            message: format!("Exceeds maximum length of {MAX_TEXT_LEN} characters"),
        });
    }
}

/// Pushes an error when an optional scalar text `field`, if present, exceeds
/// [`MAX_TEXT_LEN`] Unicode scalar values.
fn cap_opt_text(errs: &mut Vec<ValidationError>, field: &str, value: Option<&String>) {
    if let Some(v) = value {
        cap_text(errs, field, v);
    }
}

/// Pushes an error when an array `field` holds more than [`MAX_ARRAY_LEN`]
/// entries.
fn cap_array(errs: &mut Vec<ValidationError>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        errs.push(ValidationError {
            field: field.to_string(),
            message: format!("Exceeds maximum of {MAX_ARRAY_LEN} entries"),
        });
    }
}

/// Pushes an error when the `index`-th entry of an array `field` exceeds
/// [`MAX_ITEM_LEN`] Unicode scalar values (reported as `field[index]`).
fn cap_item(errs: &mut Vec<ValidationError>, field: &str, index: usize, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        errs.push(ValidationError {
            field: format!("{field}[{index}]"),
            message: format!("Exceeds maximum length of {MAX_ITEM_LEN} characters"),
        });
    }
}

/// Caps both the cardinality of a `Vec<String>` `field` and the length of each
/// of its entries.
fn cap_string_array(errs: &mut Vec<ValidationError>, field: &str, values: &[String]) {
    cap_array(errs, field, values.len());
    for (i, v) in values.iter().enumerate() {
        cap_item(errs, field, i, v);
    }
}

/// Validates one contact point under the given field `prefix`: the value must
/// be non-empty, emails must look like `x@y.z`, and phone/SMS/fax values must
/// contain at least seven digits.
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

/// Validates one address under the given field `prefix`: it must carry at
/// least one locating field (city, postal code, or country) so it is more than
/// a bare street line.
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

/// Validates one identity document under the given field `prefix`: the number
/// is required, an expiry date in the past is flagged, and the issue date must
/// not be after the expiry date.
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
        && expiry < chrono::Utc::now().date_naive()
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

/// Normalizes a phone number toward E.164 (`+<digits>`).
///
/// Strips every non-digit character, then prepends `default_country_code` when
/// the input lacks one (treating a leading `+` or a country-code prefix as
/// "already has one"). Returns an empty string for input with no digits.
///
/// # Examples
///
/// ```
/// use worker_service::validation::normalize_phone;
///
/// assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
/// assert_eq!(normalize_phone("+44 20 7946 0958", "44"), "+442079460958");
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

/// Standardizes an address: trims whitespace, expands street abbreviations in
/// line 1, title-cases the city, and upper-cases the state and country. The
/// use type, line 2, and postal code are passed through (trimmed).
///
/// # Examples
///
/// ```
/// use worker_service::validation::standardize_address;
/// use worker_service::models::Address;
///
/// let a = Address {
///     use_type: None, line1: Some("123 main st.".into()), line2: None,
///     city: Some("new york".into()), state: Some("ny".into()),
///     postal_code: Some("10001".into()), country: Some("us".into()),
/// };
/// let s = standardize_address(&a);
/// assert_eq!(s.city.as_deref(), Some("New York"));
/// assert_eq!(s.state.as_deref(), Some("NY"));
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

/// Expands common street abbreviations to their full words ("St."→"Street",
/// "Ave."→"Avenue", …) after trimming, so standardized lines read uniformly.
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

/// Title-cases a string word-by-word: the first letter of each
/// whitespace-separated word is upper-cased and the rest lower-cased.
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

/// Validates a workforce [`Assessment`] and returns every problem found
/// (empty vec means valid), mirroring [`validate_worker`]'s
/// collect-them-all contract so the API can answer one complete `422`.
///
/// Rules:
///
/// - `instrument` is required (a nameless test cannot be interpreted)
///   and every text field is length-capped (SEC-M1 input-size caps).
/// - Each result's `scale` must be **permitted by the assessment's
///   category** ([`AssessmentCategory::permits`]) — psychometric spans
///   aptitude and personality; the other categories accept only their
///   own scales. This is the rule that stops a mis-filed result from
///   silently polluting the profile view.
/// - A scale may appear at most once per assessment.
/// - `percentile` is within `[0, 100]`; `raw_score` is non-negative and
///   not above `max_score`; `max_score` is positive.
/// - `expires_on` is not before `administered_on`.
/// - A [`Completed`](crate::models::AssessmentStatus::Completed)
///   assessment carries an `administered_on` date and at least one
///   result — otherwise "completed" would assert a scoring that never
///   happened.
///
/// # Examples
///
/// ```
/// use worker_service::models::assessment::{
///     Assessment, AssessmentCategory, AssessmentResult, AssessmentScale,
/// };
/// use worker_service::validation::validate_assessment;
/// use uuid::Uuid;
///
/// let mut a = Assessment::new(Uuid::new_v4(), AssessmentCategory::Aptitude, "SHL Verify");
/// a.results.push(AssessmentResult::percentile(AssessmentScale::NumericalReasoning, 80.0));
/// assert!(validate_assessment(&a).is_empty());
///
/// // A personality scale does not belong on an aptitude assessment.
/// a.results.push(AssessmentResult::percentile(AssessmentScale::WorkStyle, 50.0));
/// assert_eq!(validate_assessment(&a).len(), 1);
/// ```
#[must_use]
pub fn validate_assessment(assessment: &Assessment) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Required: the instrument's name. Results are meaningless without
    // knowing which test produced them.
    if assessment.instrument.trim().is_empty() {
        errors.push(ValidationError {
            field: "instrument".into(),
            message: "Instrument name is required".into(),
        });
    }
    cap_text(&mut errors, "instrument", &assessment.instrument);
    cap_opt_text(&mut errors, "provider", assessment.provider.as_ref());
    cap_opt_text(
        &mut errors,
        "administered_by",
        assessment.administered_by.as_ref(),
    );
    cap_opt_text(&mut errors, "notes", assessment.notes.as_ref());
    cap_array(&mut errors, "results", assessment.results.len());

    // Expiry cannot precede administration.
    if let (Some(administered), Some(expires)) = (assessment.administered_on, assessment.expires_on)
        && expires < administered
    {
        errors.push(ValidationError {
            field: "expires_on".into(),
            message: "Expiry date cannot be before the administration date".into(),
        });
    }

    // A completed assessment must actually carry its scoring.
    if assessment.status == AssessmentStatus::Completed {
        if assessment.administered_on.is_none() {
            errors.push(ValidationError {
                field: "administered_on".into(),
                message: "A completed assessment requires an administration date".into(),
            });
        }
        if assessment.results.is_empty() {
            errors.push(ValidationError {
                field: "results".into(),
                message: "A completed assessment requires at least one result".into(),
            });
        }
    }

    let mut seen: Vec<AssessmentScale> = Vec::new();
    for (index, result) in assessment.results.iter().enumerate() {
        // The scale must be in scope for this category.
        if !assessment.category.permits(result.scale) {
            errors.push(ValidationError {
                field: format!("results[{index}].scale"),
                message: format!(
                    "Scale '{}' is not measured by a '{}' assessment",
                    result.scale, assessment.category
                ),
            });
        }
        // One reading per scale.
        if seen.contains(&result.scale) {
            errors.push(ValidationError {
                field: format!("results[{index}].scale"),
                message: format!("Scale '{}' is reported more than once", result.scale),
            });
        } else {
            seen.push(result.scale);
        }
        // Percentiles are norm-referenced: [0, 100].
        if let Some(percentile) = result.percentile
            && !(0.0..=100.0).contains(&percentile)
        {
            errors.push(ValidationError {
                field: format!("results[{index}].percentile"),
                message: "Percentile must be between 0 and 100".into(),
            });
        }
        if let Some(max) = result.max_score
            && max <= 0.0
        {
            errors.push(ValidationError {
                field: format!("results[{index}].max_score"),
                message: "Maximum score must be greater than zero".into(),
            });
        }
        if let Some(raw) = result.raw_score {
            if raw < 0.0 {
                errors.push(ValidationError {
                    field: format!("results[{index}].raw_score"),
                    message: "Raw score cannot be negative".into(),
                });
            }
            if let Some(max) = result.max_score
                && raw > max
            {
                errors.push(ValidationError {
                    field: format!("results[{index}].raw_score"),
                    message: "Raw score cannot exceed the maximum score".into(),
                });
            }
        }
        if let Some(narrative) = &result.narrative {
            cap_item(&mut errors, "results.narrative", index, narrative);
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName};

    /// A blank family name produces a `name.family` error.
    #[test]
    fn test_validate_missing_family_name() {
        let worker = Worker::new(
            HumanName {
                use_type: None,
                family: String::new(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let errors = validate_worker(&worker);
        assert!(errors.iter().any(|e| e.field == "name.family"));
    }

    /// A well-formed worker passes with no errors.
    #[test]
    fn test_validate_valid_worker() {
        let worker = Worker::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let errors = validate_worker(&worker);
        assert!(errors.is_empty());
    }

    /// US numbers normalize to "+1" E.164 form.
    #[test]
    fn test_normalize_phone_us() {
        assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
        assert_eq!(normalize_phone("+1-555-123-4567", "1"), "+15551234567");
    }

    /// City is title-cased, state and country upper-cased.
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
        // Set birth date to far in the future
        worker.birth_date = Some(chrono::NaiveDate::from_ymd_opt(2099, 1, 1).unwrap());
        let errors = validate_worker(&worker);
        assert!(
            errors.iter().any(|e| e.field == "birth_date"),
            "Future birth date should produce validation error"
        );
    }

    /// A malformed email value produces a telecom error.
    #[test]
    fn test_validate_invalid_email() {
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
        worker.telecom.push(ContactPoint {
            system: ContactPointSystem::Email,
            value: "not-an-email".into(),
            use_type: None,
        });
        let errors = validate_worker(&worker);
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("telecom") && e.message.contains("email")),
            "Invalid email should produce validation error"
        );
    }

    /// A too-short phone number produces a telecom error.
    #[test]
    fn test_validate_invalid_phone() {
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
        worker.telecom.push(ContactPoint {
            system: ContactPointSystem::Phone,
            value: "123".into(),
            use_type: None,
        });
        let errors = validate_worker(&worker);
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
        worker.tax_id = Some("---".into()); // No alphanumeric chars
        let errors = validate_worker(&worker);
        assert!(
            errors.iter().any(|e| e.field == "tax_id"),
            "Tax ID with no alphanumeric chars should fail"
        );
    }

    /// A document with an empty number produces a `number` error.
    #[test]
    fn test_validate_document_missing_number() {
        use crate::models::{DocumentType, IdentityDocument};
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
        worker.documents.push(IdentityDocument {
            document_type: DocumentType::Passport,
            number: String::new(),
            issuing_country: Some("US".into()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: false,
        });
        let errors = validate_worker(&worker);
        assert!(
            errors.iter().any(|e| e.field.contains("number")),
            "Empty document number should fail"
        );
    }

    /// A past expiry date produces an "expired" error.
    #[test]
    fn test_validate_document_expired() {
        use crate::models::{DocumentType, IdentityDocument};
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
        assert!(
            errors.iter().any(|e| e.message.contains("expired")),
            "Expired document should produce error"
        );
    }

    /// An emergency contact with a blank name produces an error.
    #[test]
    fn test_validate_emergency_contact_missing_name() {
        use crate::models::EmergencyContact;
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
        worker.emergency_contacts.push(EmergencyContact {
            name: String::new(),
            relationship: "spouse".into(),
            telecom: vec![],
            address: None,
            is_primary: true,
        });
        let errors = validate_worker(&worker);
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("emergency_contacts") && e.message.contains("name")),
            "Missing emergency contact name should produce error"
        );
    }

    /// A street-only address with no city/postal/country produces an error.
    #[test]
    fn test_validate_address_incomplete() {
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
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("addresses") && e.message.contains("city")),
            "Address without city/postal/country should produce error"
        );
    }

    /// An international number already carrying its country code normalizes cleanly.
    #[test]
    fn test_normalize_phone_international() {
        // 11 digits starting with country code
        assert_eq!(normalize_phone("+44 20 7946 0958", "44"), "+442079460958");
    }

    /// Extension text is dropped; output is "+" followed by digits only.
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

    /// Street abbreviations expand (Ave. -> Avenue) during standardization.
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

    /// City is title-cased while state and country are upper-cased.
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

    /// Builds a minimally valid worker for exercising the size caps.
    fn capped_worker() -> Worker {
        Worker::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        )
    }

    /// SEC-M1: an oversized scalar text field produces a length-cap error.
    #[test]
    fn test_cap_oversized_scalar_text() {
        let mut worker = capped_worker();
        worker.tax_id = Some("x".repeat(MAX_TEXT_LEN + 1));
        let errors = validate_worker(&worker);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "tax_id" && e.message.contains("maximum length")),
            "Oversized tax_id should produce a length-cap error"
        );
    }

    /// SEC-M1: an over-long array produces a cardinality-cap error.
    #[test]
    fn test_cap_over_long_array() {
        let mut worker = capped_worker();
        worker.photo = vec!["p".to_string(); MAX_ARRAY_LEN + 1];
        let errors = validate_worker(&worker);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "photo" && e.message.contains("maximum of")),
            "Over-long photo array should produce a cardinality-cap error"
        );
    }

    /// SEC-M1: a single oversized array entry produces an indexed length-cap error.
    #[test]
    fn test_cap_oversized_array_entry() {
        let mut worker = capped_worker();
        worker.photo = vec!["ok".into(), "x".repeat(MAX_ITEM_LEN + 1)];
        let errors = validate_worker(&worker);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "photo[1]" && e.message.contains("maximum length")),
            "Oversized array entry should produce an indexed length-cap error"
        );
    }

    /// SEC-M1: a large record with every field exactly at its cap produces no
    /// cap errors (the boundary is inclusive).
    #[test]
    fn test_cap_within_limits_ok() {
        let mut worker = Worker::new(
            HumanName {
                use_type: None,
                family: "x".repeat(MAX_TEXT_LEN),
                given: vec!["y".repeat(MAX_ITEM_LEN)],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        worker.tax_id = Some("a".repeat(MAX_TEXT_LEN));
        worker.marital_status = Some("m".repeat(MAX_TEXT_LEN));
        worker.photo = vec!["p".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN];
        let errors = validate_worker(&worker);
        assert!(
            errors.is_empty(),
            "A within-caps record should produce no errors, got {errors:?}"
        );
    }

    // ─── Assessment validation ──────────────────────────────────────────────

    use crate::models::assessment::{Assessment, AssessmentCategory, AssessmentResult};

    /// A completed assessment with an in-scope, well-scored result is
    /// valid.
    #[test]
    fn test_valid_assessment() {
        let mut a = Assessment::new(
            uuid::Uuid::new_v4(),
            AssessmentCategory::Aptitude,
            "SHL Verify G+",
        );
        a.status = AssessmentStatus::Completed;
        a.administered_on = chrono::NaiveDate::from_ymd_opt(2026, 5, 4);
        a.expires_on = chrono::NaiveDate::from_ymd_opt(2028, 5, 4);
        a.results.push(AssessmentResult::percentile(
            AssessmentScale::NumericalReasoning,
            74.5,
        ));
        let errors = validate_assessment(&a);
        assert!(
            errors.is_empty(),
            "expected a valid assessment, got {errors:?}"
        );
    }

    /// A scale outside the category's scope is rejected, naming the
    /// offending index — the rule that keeps the profile view honest.
    #[test]
    fn test_assessment_scale_must_suit_the_category() {
        let mut a = Assessment::new(
            uuid::Uuid::new_v4(),
            AssessmentCategory::Personality,
            "Big Five Inventory",
        );
        a.results.push(AssessmentResult::percentile(
            AssessmentScale::NumericalReasoning,
            50.0,
        ));
        let errors = validate_assessment(&a);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "results[0].scale");
        assert!(errors[0].message.contains("numerical_reasoning"));

        // The same result is fine on a psychometric assessment, which
        // spans aptitude and personality.
        let mut psycho = Assessment::new(
            uuid::Uuid::new_v4(),
            AssessmentCategory::Psychometric,
            "Battery",
        );
        psycho.results.push(AssessmentResult::percentile(
            AssessmentScale::NumericalReasoning,
            50.0,
        ));
        assert!(validate_assessment(&psycho).is_empty());
    }

    /// Out-of-range percentiles, negative or over-maximum raw scores, a
    /// non-positive maximum, and a repeated scale are each reported.
    #[test]
    fn test_assessment_score_rules() {
        let mut a = Assessment::new(
            uuid::Uuid::new_v4(),
            AssessmentCategory::Selection,
            "Assessment centre",
        );
        a.results.push(AssessmentResult {
            scale: AssessmentScale::JobSimulation,
            raw_score: Some(-1.0),
            max_score: Some(0.0),
            percentile: Some(101.0),
            band: None,
            narrative: None,
        });
        // The same scale twice.
        a.results.push(AssessmentResult::percentile(
            AssessmentScale::JobSimulation,
            50.0,
        ));
        // Raw above the maximum.
        a.results.push(AssessmentResult {
            scale: AssessmentScale::SkillsAssessment,
            raw_score: Some(12.0),
            max_score: Some(10.0),
            percentile: None,
            band: None,
            narrative: None,
        });

        let fields: Vec<String> = validate_assessment(&a)
            .into_iter()
            .map(|e| e.field)
            .collect();
        for expected in [
            "results[0].percentile",
            "results[0].max_score",
            "results[0].raw_score",
            "results[1].scale",
            "results[2].raw_score",
        ] {
            assert!(
                fields.iter().any(|f| f == expected),
                "missing {expected} in {fields:?}"
            );
        }
    }

    /// A completed assessment must carry its administration date and at
    /// least one result; an expiry cannot precede administration.
    #[test]
    fn test_assessment_completion_and_date_rules() {
        let mut a = Assessment::new(
            uuid::Uuid::new_v4(),
            AssessmentCategory::Psychometric,
            "Hogan HPI",
        );
        a.status = AssessmentStatus::Completed;
        let errors = validate_assessment(&a);
        assert!(errors.iter().any(|e| e.field == "administered_on"));
        assert!(errors.iter().any(|e| e.field == "results"));

        // A scheduled assessment with neither is fine — nothing is
        // asserted about scoring yet.
        a.status = AssessmentStatus::Scheduled;
        assert!(validate_assessment(&a).is_empty());

        // Expiry before administration is refused.
        a.administered_on = chrono::NaiveDate::from_ymd_opt(2026, 5, 4);
        a.expires_on = chrono::NaiveDate::from_ymd_opt(2026, 1, 1);
        let errors = validate_assessment(&a);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "expires_on");
    }

    /// SEC-M1: assessment text fields and the result array are capped.
    #[test]
    fn test_assessment_caps() {
        let mut a = Assessment::new(
            uuid::Uuid::new_v4(),
            AssessmentCategory::Aptitude,
            "x".repeat(MAX_TEXT_LEN + 1),
        );
        a.notes = Some("n".repeat(MAX_TEXT_LEN + 1));
        a.results =
            vec![AssessmentResult::new(AssessmentScale::NumericalReasoning); MAX_ARRAY_LEN + 1];
        let errors = validate_assessment(&a);
        assert!(errors.iter().any(|e| e.field == "instrument"));
        assert!(errors.iter().any(|e| e.field == "notes"));
        assert!(errors.iter().any(|e| e.field == "results"));

        // A blank instrument name is required, not merely capped.
        let blank = Assessment::new(uuid::Uuid::new_v4(), AssessmentCategory::Aptitude, "   ");
        assert!(
            validate_assessment(&blank)
                .iter()
                .any(|e| e.field == "instrument" && e.message.contains("required"))
        );
    }
}
