//! FHIR R5 wire types for the `Organization` resource.
//!
//! Slim, self-contained Serde structs for exactly the elements this
//! service populates (per [`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3, `high` fidelity) plus the shared envelope types every FHIR
//! endpoint returns: [`FhirOperationOutcome`] for errors and
//! [`FhirBundle`] for search sets. Field names and casing follow FHIR
//! JSON (`resourceType`, `postalCode`, `fullUrl`, and the reserved-word
//! renames `use`/`type`). Absent optionals and empty arrays are omitted
//! so resources stay clean.
//!
//! These types are **copied per project** (drift-accepted, the family's
//! `mxi-events`/`EntityRef` posture); they are not a shared crate.

use serde::{Deserialize, Serialize};

/// A FHIR R5 `Organization` resource (the elements this service maps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirOrganization {
    /// FHIR resource-type discriminator — always `"Organization"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical id (the record's `pid`). Absent on an inbound create.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    /// Resource metadata (version / last-updated).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (LEI, DUNS, …) as `system|value` tokens.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifier: Vec<FhirIdentifier>,
    /// Whether the organization record is active (soft-delete ⇒ `false`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active: Option<bool>,
    /// The common name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    /// Alternate / legal names.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alias: Vec<String>,
    /// Contact points (phone / email / url).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub telecom: Vec<FhirContactPoint>,
    /// Postal addresses.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub address: Vec<FhirAddress>,
}

impl FhirOrganization {
    /// An empty `Organization` resource (only `resourceType` set); build
    /// it up field by field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "Organization".to_string(),
            id: None,
            meta: None,
            identifier: Vec::new(),
            active: None,
            name: None,
            alias: Vec::new(),
            telecom: Vec::new(),
            address: Vec::new(),
        }
    }
}

impl Default for FhirOrganization {
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
    /// Last-updated instant (the row's `updated_at`).
    #[serde(rename = "lastUpdated", skip_serializing_if = "Option::is_none", default)]
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

/// FHIR `ContactPoint` — a typed contact value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirContactPoint {
    /// `phone` | `email` | `url` | ….
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The contact value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// FHIR `Address` — the postal subset this service carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirAddress {
    /// Street lines (`street_address` split on newlines).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub line: Vec<String>,
    /// City / locality.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub city: Option<String>,
    /// State / region.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<String>,
    /// Postal / ZIP code.
    #[serde(rename = "postalCode", skip_serializing_if = "Option::is_none", default)]
    pub postal_code: Option<String>,
    /// Country.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub country: Option<String>,
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
    /// Absolute-ish URL identifying the resource (`Organization/{id}`).
    #[serde(rename = "fullUrl")]
    pub full_url: String,
    /// The contained resource.
    pub resource: FhirOrganization,
}

impl FhirBundle {
    /// Assemble a `searchset` Bundle from rendered resources.
    #[must_use]
    pub fn searchset(resources: Vec<FhirOrganization>) -> Self {
        let entry = resources
            .into_iter()
            .map(|r| FhirBundleEntry {
                full_url: format!(
                    "Organization/{}",
                    r.id.clone().unwrap_or_default()
                ),
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
