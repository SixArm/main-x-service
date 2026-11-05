//! FHIR R5 wire types for the `PlanDefinition` resource.
//!
//! Slim, self-contained Serde structs for exactly the elements this
//! service populates (per [`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3, `medium` fidelity — a clinical pathway *template*) plus the shared
//! envelope types every FHIR endpoint returns: [`FhirOperationOutcome`]
//! for errors and [`FhirBundle`] for search sets. Field names and casing
//! follow FHIR JSON (`resourceType`, `useContext`, `valueCodeableConcept`,
//! `relatedArtifact`, `fullUrl`, and the reserved-word renames `type`).
//! Absent optionals and empty arrays are omitted so resources stay clean.
//!
//! These types are **copied per project** (drift-accepted, the family's
//! `mxi-events`/`EntityRef` posture); they are not a shared crate.

use serde::{Deserialize, Serialize};

/// A FHIR R5 `PlanDefinition` resource (the elements this service maps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirPlanDefinition {
    /// FHIR resource-type discriminator — always `"PlanDefinition"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical id (the record's `pid`). Absent on an inbound create.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    /// Resource metadata (version / last-updated).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (pathway code, provider, DOI, guideline id, …)
    /// as `system|value` tokens.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifier: Vec<FhirIdentifier>,
    /// Publication status (`active` for a live record, `retired` for a
    /// soft-deleted one). Required by FHIR; optional inbound.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<String>,
    /// Human-friendly pathway title (the record's `name`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    /// Resource type classification — a constant "clinical protocol".
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<FhirCodeableConcept>,
    /// Usage contexts: target conditions, care setting, and keyword topics.
    #[serde(rename = "useContext", skip_serializing_if = "Vec::is_empty", default)]
    pub use_context: Vec<FhirUsageContext>,
    /// Pathway actions (the interventions / steps).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub action: Vec<FhirPlanAction>,
    /// Related artifacts (cross-system identity URLs from `same_as`).
    #[serde(
        rename = "relatedArtifact",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub related_artifact: Vec<FhirRelatedArtifact>,
}

impl FhirPlanDefinition {
    /// An empty `PlanDefinition` resource (only `resourceType` set); build
    /// it up field by field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "PlanDefinition".to_string(),
            id: None,
            meta: None,
            identifier: Vec::new(),
            status: None,
            title: None,
            kind: None,
            use_context: Vec::new(),
            action: Vec::new(),
            related_artifact: Vec::new(),
        }
    }
}

impl Default for FhirPlanDefinition {
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
    #[serde(
        rename = "lastUpdated",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub last_updated: Option<String>,
    /// `StructureDefinition` canonicals this resource claims conformance
    /// to — always [`crate::fhir::profile::PROFILE_URL`] on rendering. A
    /// resource that declares no profile makes no conformance claim, which
    /// is what ONC-style profile validation checks against.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub profile: Vec<String>,
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

/// FHIR `Coding` — a `system` + `code` (+ optional `display`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCoding {
    /// The code system URI.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The code value within `system`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub code: Option<String>,
    /// Human-readable label for the code.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
}

/// FHIR `CodeableConcept` — one or more codings plus optional free text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCodeableConcept {
    /// The codings for this concept.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub coding: Vec<FhirCoding>,
    /// Plain-text representation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text: Option<String>,
}

/// FHIR `UsageContext` — a `code` (what kind of context) plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirUsageContext {
    /// The context type (`focus` = condition, `setting`, `topic`).
    pub code: FhirCoding,
    /// The context value as a codeable concept.
    #[serde(rename = "valueCodeableConcept")]
    pub value_codeable_concept: FhirCodeableConcept,
}

/// FHIR `PlanDefinition.action` — one pathway step / intervention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirPlanAction {
    /// The action's title (the intervention text).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
}

/// FHIR `RelatedArtifact` — a `type` + `url` reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirRelatedArtifact {
    /// The artifact relationship type (always `"documentation"` here).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub artifact_type: Option<String>,
    /// The artifact URL.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
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
    /// Absolute-ish URL identifying the resource (`PlanDefinition/{id}`).
    #[serde(rename = "fullUrl")]
    pub full_url: String,
    /// The contained resource.
    pub resource: FhirPlanDefinition,
}

impl FhirBundle {
    /// Assemble a `searchset` Bundle from rendered resources.
    #[must_use]
    pub fn searchset(resources: Vec<FhirPlanDefinition>) -> Self {
        let entry = resources
            .into_iter()
            .map(|r| FhirBundleEntry {
                full_url: format!("PlanDefinition/{}", r.id.clone().unwrap_or_default()),
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
