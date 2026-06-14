//! Serde structs mirroring the HL7 FHIR R5 wire format.
//!
//! Each type maps one-to-one onto a FHIR element, and `#[serde(rename_all =
//! "camelCase")]` plus the trailing-underscore fields (`use_`, `type_`) make
//! the JSON match FHIR's `camelCase` / reserved-keyword naming. Optional
//! fields use `skip_serializing_if = "Option::is_none"` so absent data is
//! omitted rather than emitted as `null`, per FHIR convention. Field names are
//! intentionally identical to the FHIR R5 element names (see the FHIR spec for
//! their semantics), so per-field docs are omitted to avoid restating the
//! standard; the conversions to/from the internal model live in
//! [`super::to_fhir_worker`] / [`super::from_fhir_worker`].

use serde::{Deserialize, Serialize};

/// FHIR R5 `Patient`-shaped Worker resource — the top-level resource exchanged
/// by the FHIR endpoints. Fields mirror FHIR R5 element names (see module docs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirWorker {
    /// FHIR `resourceType` discriminator.
    pub resource_type: String,
    /// FHIR `id` — logical resource identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// FHIR `meta` — version and last-updated metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<FhirMeta>,
    /// FHIR `identifier` — business identifiers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<FhirIdentifier>>,
    /// FHIR `active` — whether the record is in active use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// FHIR `name` — human names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Vec<FhirHumanName>>,
    /// FHIR `telecom` — contact points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<FhirContactPoint>>,
    /// FHIR `gender` — administrative gender code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    /// FHIR `birthDate` — date of birth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    /// FHIR `deceased[x]` — deceased flag or date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased: Option<FhirDeceased>,
    /// FHIR `address` — postal addresses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<FhirAddress>>,
    /// FHIR `maritalStatus` — marital status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marital_status: Option<FhirCodeableConcept>,
    /// FHIR `multipleBirth[x]` — multiple-birth flag or order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_birth: Option<FhirMultipleBirth>,
    /// FHIR `photo` — attached photos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<Vec<FhirAttachment>>,
    /// FHIR `link` — links to related worker resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Vec<FhirWorkerLink>>,
    /// FHIR `managingOrganization` — reference to the managing organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<FhirReference>,
}

/// FHIR `Meta` element — resource version and last-updated timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirMeta {
    /// FHIR `versionId` — resource version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    /// FHIR `lastUpdated` — last-modified instant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// FHIR `Identifier` element — a typed, system-scoped business identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirIdentifier {
    /// FHIR `use` — identifier purpose code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// FHIR `type` — coded identifier type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<FhirCodeableConcept>,
    /// FHIR `system` — namespace URI for the value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// FHIR `value` — the identifier value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// FHIR `assigner` — organization that issued the identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigner: Option<FhirReference>,
}

/// FHIR `HumanName` element — family/given/prefix/suffix name parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirHumanName {
    /// FHIR `use` — name purpose code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// FHIR `text` — full name rendered as text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// FHIR `family` — family (last) name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// FHIR `given` — given (first/middle) names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<Vec<String>>,
    /// FHIR `prefix` — name prefixes (e.g. titles).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<Vec<String>>,
    /// FHIR `suffix` — name suffixes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<Vec<String>>,
}

/// FHIR `ContactPoint` element — a telecom endpoint (phone, email, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirContactPoint {
    /// FHIR `system` — telecom system code (phone, email, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// FHIR `value` — the contact value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// FHIR `use` — contact-point purpose code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
}

/// FHIR `Address` element — postal address parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAddress {
    /// FHIR `use` — address purpose code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    /// FHIR `type` — address type code (postal, physical, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// FHIR `text` — full address rendered as text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// FHIR `line` — street address lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Vec<String>>,
    /// FHIR `city` — city name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// FHIR `state` — state or province.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// FHIR `postalCode` — postal/ZIP code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// FHIR `country` — country code or name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// FHIR `CodeableConcept` element — one or more codings plus free text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCodeableConcept {
    /// FHIR `coding` — codes from one or more code systems.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<FhirCoding>>,
    /// FHIR `text` — plain-text representation of the concept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// FHIR `Coding` element — a single `system` + `code` (+ display) triple.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCoding {
    /// FHIR `system` — code system URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// FHIR `code` — the symbol in the code system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// FHIR `display` — human-readable label for the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR `Reference` element — a literal reference (`Resource/{id}`) and/or
/// display text pointing at another resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirReference {
    /// FHIR `reference` — literal reference (`Resource/{id}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// FHIR `display` — human-readable label for the target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR `Patient.link` element — a link to another worker plus its link type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirWorkerLink {
    /// Reference to the linked worker.
    pub other: FhirReference,
    /// Link type code (e.g. `replaces`, `seealso`), lowercased.
    pub type_: String,
}

/// FHIR `Attachment` element — inline or referenced binary content (photos).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAttachment {
    /// FHIR `contentType` — MIME type of the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// FHIR `data` — base64-encoded inline content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// FHIR `url` — location of the referenced content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// FHIR `deceased[x]` choice element — either a boolean or a dateTime.
/// `#[serde(untagged)]` lets it deserialize from whichever JSON form arrives.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FhirDeceased {
    /// `deceasedBoolean` — whether the worker is deceased.
    Boolean(bool),
    /// `deceasedDateTime` — RFC 3339 instant of death.
    DateTime(String),
}

/// FHIR `multipleBirth[x]` choice element — a boolean flag or birth-order int.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FhirMultipleBirth {
    /// `multipleBirthBoolean` — whether part of a multiple birth.
    Boolean(bool),
    /// `multipleBirthInteger` — the birth order.
    Integer(i32),
}

/// FHIR `OperationOutcome` resource — the standard error/diagnostic envelope
/// returned by the FHIR endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcome {
    /// Always `"OperationOutcome"`.
    pub resource_type: String,
    /// One or more issues describing what went wrong.
    pub issue: Vec<FhirOperationOutcomeIssue>,
}

/// A single entry inside an [`FhirOperationOutcome`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcomeIssue {
    /// Severity code (e.g. `error`, `warning`).
    pub severity: String,
    /// Issue type code (e.g. `not-found`, `invalid`).
    pub code: String,
    /// Optional coded details about the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<FhirCodeableConcept>,
    /// Optional human-readable diagnostic message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

impl FhirOperationOutcome {
    /// Builds an `error`-severity outcome with the given issue `code` and
    /// diagnostic message.
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

    /// Builds a `not-found` outcome for the given resource type and id.
    pub fn not_found(resource_type: &str, id: &str) -> Self {
        Self::error(
            "not-found",
            &format!("{} with id '{}' not found", resource_type, id),
        )
    }

    /// Builds an `invalid` outcome carrying a validation message.
    pub fn invalid(message: &str) -> Self {
        Self::error("invalid", message)
    }
}

impl FhirWorker {
    /// Creates an empty `Worker` resource with every optional element unset
    /// (only `resourceType` is populated). Build it up field by field.
    pub fn new() -> Self {
        Self {
            // Wire discriminator: every emitted resource carries
            // `"resourceType": "Worker"`.
            resource_type: "Worker".to_string(),
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

impl Default for FhirWorker {
    /// Same as [`FhirWorker::new`] — an empty resource.
    fn default() -> Self {
        Self::new()
    }
}
