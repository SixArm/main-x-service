//! FHIR R5 resource and data-type structs (serde `camelCase`).
//!
//! These mirror the HL7 FHIR R5 wire shapes for the Person resource and
//! its nested data types (Identifier, HumanName, ContactPoint, Address,
//! CodeableConcept, …) plus [`FhirOperationOutcome`] for errors. Field
//! names follow the FHIR spec; Rust keywords are escaped with a trailing
//! underscore (`use_`, `type_`) and renamed back via serde. Optional
//! elements skip serialization when `None`. The domain ⇄ FHIR mapping
//! lives in [`super::to_fhir_person`](crate::api::fhir::to_fhir_person) / [`super::from_fhir_person`](crate::api::fhir::from_fhir_person).

use serde::{Deserialize, Serialize};

/// FHIR R5 resource for the domain `Person`.
///
/// The primary, clinically-expected representation is **`Patient`**
/// (`resourceType: "Patient"`); the same struct also backs the thin
/// **`Person`** demographic alias (`resourceType: "Person"`). Both are
/// built from the same domain `Person` — only the `resource_type`
/// discriminator differs (per
/// [`agents/share/fhir.md`](../../../../agents/share/fhir.md) §3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirPerson {
    /// FHIR resource-type discriminator — `"Patient"` (primary) or
    /// `"Person"` (demographic alias).
    pub resource_type: String,
    /// Logical id of the resource (the person UUID as a string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Resource metadata (version, last-updated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (MRN, SSN, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<FhirIdentifier>>,
    /// Whether this person record is in active use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// One or more names for the person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Vec<FhirHumanName>>,
    /// Telecom contact points (phone, email, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<FhirContactPoint>>,
    /// Administrative gender code (`male`/`female`/`other`/`unknown`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    /// Date of birth as `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    /// Deceased indicator (boolean or dateTime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased: Option<FhirDeceased>,
    /// Postal/physical addresses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<FhirAddress>>,
    /// Marital status as a coded concept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marital_status: Option<FhirCodeableConcept>,
    /// Multiple-birth indicator (boolean or integer).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_birth: Option<FhirMultipleBirth>,
    /// Photo attachments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<Vec<FhirAttachment>>,
    /// Links to other Person resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Vec<FhirPersonLink>>,
    /// Reference to the organization managing this record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<FhirReference>,
}

/// FHIR `Meta` element: per-resource metadata (version, last-updated).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirMeta {
    /// Version-specific identifier for this resource state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    /// Instant the resource was last changed (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// FHIR `Identifier` data type: a business identifier (MRN, SSN, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirIdentifier {
    /// Purpose of this identifier (`usual`/`official`/`temp`/`secondary`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// Coded type of identifier (e.g. MRN, SSN).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<FhirCodeableConcept>,
    /// Namespace URI that scopes `value`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The identifier value itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Organization that issued/assigned the identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigner: Option<FhirReference>,
}

/// FHIR `HumanName` data type: a name with parts and usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirHumanName {
    /// Purpose of this name (`official`/`maiden`/`nickname`/…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// Full text representation of the name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Family (sur-) name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Given (first/middle) names, in order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<Vec<String>>,
    /// Honorific prefixes (Dr., Mr., …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<Vec<String>>,
    /// Honorific suffixes (Jr., `PhD`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<Vec<String>>,
}

/// FHIR `ContactPoint` data type: phone/email/etc. plus its usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirContactPoint {
    /// Channel (`phone`/`email`/`fax`/`url`/…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The actual contact value (number, address, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Purpose of this contact point (`home`/`work`/`mobile`/…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
}

/// FHIR `Address` data type: a postal/physical address.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAddress {
    /// Purpose of this address (`home`/`work`/`temp`/…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// Address kind (`postal`/`physical`/`both`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Full text representation of the address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Street address lines, in order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Vec<String>>,
    /// City / locality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// State / province / region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Postal / ZIP code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// Country (ISO code or name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// FHIR `CodeableConcept` data type: codings plus free text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCodeableConcept {
    /// Coded representations from one or more code systems.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<FhirCoding>>,
    /// Human-readable text for the concept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// FHIR `Coding` data type: one code from a code system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCoding {
    /// Code system URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Symbol/code within the system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Human-readable display for the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR `Reference` data type: a pointer to another resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirReference {
    /// Literal reference (e.g. `Organization/123`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Human-readable label for the referenced resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR `Person.link`: a typed link to another Person resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirPersonLink {
    /// The other Person/Patient/Practitioner being linked.
    pub other: FhirReference,
    /// Link type (`replaces`/`refer`/`seealso`).
    pub type_: String,
}

/// FHIR `Attachment` data type: inline or referenced binary content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAttachment {
    /// MIME type of the content (e.g. `image/png`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Base64-encoded inline data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// URL where the content can be retrieved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// FHIR `deceased[x]` choice type: boolean flag or date-time of death.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FhirDeceased {
    /// `deceasedBoolean` — whether the person is deceased.
    Boolean(bool),
    /// `deceasedDateTime` — the instant of death (ISO 8601).
    DateTime(String),
}

/// FHIR `multipleBirth[x]` choice type: boolean flag or birth order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FhirMultipleBirth {
    /// `multipleBirthBoolean` — whether part of a multiple birth.
    Boolean(bool),
    /// `multipleBirthInteger` — the birth order (1 = first, …).
    Integer(i32),
}

/// FHIR `OperationOutcome` resource: structured error/warning report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcome {
    /// FHIR resource type discriminator (always `"OperationOutcome"`).
    pub resource_type: String,
    /// One or more issues describing the outcome.
    pub issue: Vec<FhirOperationOutcomeIssue>,
}

/// FHIR `OperationOutcome.issue`: a single error/warning entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcomeIssue {
    /// Severity (`fatal`/`error`/`warning`/`information`).
    pub severity: String,
    /// Machine-readable issue code (e.g. `not-found`, `invalid`).
    pub code: String,
    /// Optional coded details for the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<FhirCodeableConcept>,
    /// Free-text diagnostics for human consumption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

impl FhirOperationOutcome {
    /// Build a single-issue `error`-severity `OperationOutcome` with the
    /// given machine `code` and human-readable `diagnostics` text.
    #[must_use]
    pub fn error(code: &str, diagnostics: &str) -> Self {
        Self {
            resource_type: "OperationOutcome".to_string(),
            issue: vec![FhirOperationOutcomeIssue {
                severity: "error".to_string(),
                code: code.to_string(),
                details: None,
                diagnostics: Some(diagnostics.to_string()),
            }],
        }
    }

    /// Build a `not-found` `OperationOutcome` for a missing resource of the
    /// given `resource_type` and `id` (maps to HTTP 404).
    #[must_use]
    pub fn not_found(resource_type: &str, id: &str) -> Self {
        Self::error(
            "not-found",
            &format!("{resource_type} with id '{id}' not found"),
        )
    }

    /// Build an `invalid` `OperationOutcome` carrying the given message
    /// (maps to HTTP 400 for malformed/invalid input).
    #[must_use]
    pub fn invalid(message: &str) -> Self {
        Self::error("invalid", message)
    }
}

impl FhirPerson {
    /// Construct a minimal [`FhirPerson`] with `resourceType` set to
    /// `"Patient"` (the primary resource type) and every optional
    /// element left as `None`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "Patient".to_string(),
            id: None,
            meta: None,
            identifier: None,
            active: None,
            name: None,
            telecom: None,
            gender: None,
            birth_date: None,
            deceased: None,
            address: None,
            marital_status: None,
            multiple_birth: None,
            photo: None,
            link: None,
            managing_organization: None,
        }
    }
}

impl Default for FhirPerson {
    /// Delegates to [`FhirPerson::new`] for an empty Person resource.
    fn default() -> Self {
        Self::new()
    }
}
