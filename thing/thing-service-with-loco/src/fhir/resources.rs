//! FHIR R5 wire types for the `Device` resource.
//!
//! Slim, self-contained Serde structs for exactly the elements this
//! service populates (per [`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3, `medium` fidelity) plus the shared envelope types every FHIR
//! endpoint returns: [`FhirOperationOutcome`] for errors and
//! [`FhirBundle`] for search sets. Field names and casing follow FHIR
//! JSON (`resourceType`, `modelNumber`, `fullUrl`, and the reserved-word
//! rename `type`). Absent optionals and empty arrays are omitted so
//! resources stay clean.
//!
//! These types are **copied per project** (drift-accepted, the family's
//! `mxi-events`/`EntityRef` posture); they are not a shared crate.

use serde::{Deserialize, Serialize};

/// A FHIR R5 `Device` resource (the elements this service maps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirDevice {
    /// FHIR resource-type discriminator — always `"Device"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical id (the record's `id`). Absent on an inbound create.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    /// Resource metadata (version / last-updated).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (DOI, ISBN, GTIN, serial, …) as `system|value`
    /// tokens.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifier: Vec<FhirIdentifier>,
    /// Availability status (`active` while the record is not soft-deleted,
    /// else `inactive`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<String>,
    /// Human names for the device (first entry is the primary `name`;
    /// the rest are `alternate_names`).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub name: Vec<FhirDeviceName>,
    /// The kind of device (mapped from the schema.org `additional_type`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub device_type: Option<FhirCodeableConcept>,
    /// Manufacturer (approximate mapping from the domain `owner`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub manufacturer: Option<String>,
    /// Model number (approximate mapping from the domain
    /// `disambiguating_description`).
    #[serde(
        rename = "modelNumber",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub model_number: Option<String>,
    /// Free-text notes (carries the domain `description`).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub note: Vec<FhirAnnotation>,
}

impl FhirDevice {
    /// An empty `Device` resource (only `resourceType` set); build it up
    /// field by field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "Device".to_string(),
            id: None,
            meta: None,
            identifier: Vec::new(),
            status: None,
            name: Vec::new(),
            device_type: None,
            manufacturer: None,
            model_number: None,
            note: Vec::new(),
        }
    }
}

impl Default for FhirDevice {
    fn default() -> Self {
        Self::new()
    }
}

/// FHIR `Meta` — the subset we populate (`versionId` unused today).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirMeta {
    /// Version id — not tracked yet (no `_history`), always absent.
    #[serde(rename = "versionId", skip_serializing_if = "Option::is_none", default)]
    pub version_id: Option<String>,
    /// Last-updated instant (the record's `updated_at`).
    #[serde(
        rename = "lastUpdated",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub last_updated: Option<String>,
}

/// FHIR `Identifier` — a `system|value` business identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirIdentifier {
    /// The namespace URI the value is unique within.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The identifier value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// FHIR R5 `Device.name` — a human name with a type discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirDeviceName {
    /// The name text.
    pub value: String,
    /// `registered-name` (the primary name) | `user-friendly-name`
    /// (aliases).
    #[serde(rename = "type")]
    pub name_type: String,
}

/// FHIR `CodeableConcept` — the `text` subset this service carries
/// (plus any `coding` an inbound resource supplied, for round-trip).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCodeableConcept {
    /// Structured codings (never emitted by this service; preserved if
    /// an inbound resource sent them).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub coding: Vec<FhirCoding>,
    /// Plain-text representation (the schema.org `additional_type` URL).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text: Option<String>,
}

/// FHIR `Coding` — a `system`/`code`/`display` triple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCoding {
    /// Code system URI.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The symbol in that system.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub code: Option<String>,
    /// Human display for the code.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
}

/// FHIR `Annotation` — the `text` subset (carries the `description`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirAnnotation {
    /// The note text.
    pub text: String,
}

/// A FHIR `OperationOutcome` — the body of every non-2xx FHIR response
/// ([`agents/share/fhir.md`](../../../../agents/share/fhir.md) §5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirOperationOutcome {
    /// Always `"OperationOutcome"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// One issue per problem (validation errors are one-per-issue).
    pub issue: Vec<FhirIssue>,
}

impl FhirOperationOutcome {
    /// Build an outcome carrying a single `error`-severity issue.
    #[must_use]
    pub fn error(code: &str, diagnostics: impl Into<String>) -> Self {
        Self {
            resource_type: "OperationOutcome".to_string(),
            issue: vec![FhirIssue {
                severity: "error".to_string(),
                code: code.to_string(),
                diagnostics: Some(diagnostics.into()),
            }],
        }
    }

    /// Build an outcome carrying one `error`-severity issue per diagnostic
    /// (used to surface every validation failure at once).
    #[must_use]
    pub fn errors(code: &str, diagnostics: impl IntoIterator<Item = String>) -> Self {
        let issue = diagnostics
            .into_iter()
            .map(|d| FhirIssue {
                severity: "error".to_string(),
                code: code.to_string(),
                diagnostics: Some(d),
            })
            .collect::<Vec<_>>();
        Self {
            resource_type: "OperationOutcome".to_string(),
            issue,
        }
    }
}

/// One `OperationOutcome.issue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirIssue {
    /// `fatal` | `error` | `warning` | `information`.
    pub severity: String,
    /// The FHIR issue-type code (`not-found`, `invalid`, `processing`, …).
    pub code: String,
    /// Human-readable detail.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub diagnostics: Option<String>,
}

/// A FHIR `searchset` `Bundle` wrapping search results
/// ([`agents/share/fhir.md`](../../../../agents/share/fhir.md) §6).
#[derive(Debug, Clone, Serialize)]
pub struct FhirBundle {
    /// Always `"Bundle"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Always `"searchset"` here.
    #[serde(rename = "type")]
    pub bundle_type: String,
    /// Total matching resources (of this page's query).
    pub total: usize,
    /// One entry per resource.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entry: Vec<FhirBundleEntry>,
}

/// One `Bundle.entry`.
#[derive(Debug, Clone, Serialize)]
pub struct FhirBundleEntry {
    /// Absolute-ish URL identifying the resource (`Device/{id}`).
    #[serde(rename = "fullUrl")]
    pub full_url: String,
    /// The contained resource.
    pub resource: FhirDevice,
}

impl FhirBundle {
    /// Assemble a `searchset` Bundle from rendered resources.
    #[must_use]
    pub fn searchset(resources: Vec<FhirDevice>) -> Self {
        let entry = resources
            .into_iter()
            .map(|r| FhirBundleEntry {
                full_url: format!("Device/{}", r.id.clone().unwrap_or_default()),
                resource: r,
            })
            .collect::<Vec<_>>();
        Self {
            resource_type: "Bundle".to_string(),
            bundle_type: "searchset".to_string(),
            total: entry.len(),
            entry,
        }
    }
}
