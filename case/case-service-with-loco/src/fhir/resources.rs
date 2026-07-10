//! FHIR R5 wire types for the `Task` resource (the case service's
//! best-effort mapping — [`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3, `low` fidelity).
//!
//! Slim, self-contained Serde structs for exactly the elements this
//! service populates, plus the shared envelope types every FHIR endpoint
//! returns: [`FhirOperationOutcome`] for errors and [`FhirBundle`] for
//! search sets. Field names and casing follow FHIR JSON (`resourceType`,
//! `fullUrl`, the reserved-word rename `for` → `for_` on the wire `for`).
//! Absent optionals and empty arrays are omitted so resources stay clean.
//!
//! These types are **copied per project** (drift-accepted, the family's
//! `mxi-events`/`EntityRef` posture); they are not a shared crate.

use serde::{Deserialize, Serialize};

/// A FHIR R5 `Task` resource (the elements this service maps).
///
/// A governmental case has no exact FHIR analog; `Task` is the closest
/// tracked-unit-of-work resource (status/priority/subject), so the
/// mapping is deliberately **best-effort** and lossy — every gap is
/// documented on [`crate::fhir::from_fhir_task`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirTask {
    /// FHIR resource-type discriminator — always `"Task"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical id (the record's `pid`). Absent on an inbound create.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    /// Resource metadata (version / last-updated).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (docket, external id, agency case number, …)
    /// as `system|value` tokens.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifier: Vec<FhirIdentifier>,
    /// Workflow status (FHIR `TaskStatus`), derived from the case status.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<String>,
    /// FHIR requires an `intent`; a registered case is always `"order"`.
    pub intent: String,
    /// Priority (FHIR `RequestPriority`: `routine`/`urgent`/`asap`/`stat`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub priority: Option<String>,
    /// The kind of case (mapped from `case_type`) as a `CodeableConcept`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub code: Option<FhirCodeableConcept>,
    /// Human description — the case `title`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// The beneficiary / subject the case is about (first `subjects`
    /// entry). **High-sensitivity** — see [`crate::fhir`] and
    /// [`crate::controllers::fhir`].
    #[serde(rename = "for", skip_serializing_if = "Option::is_none", default)]
    pub for_: Option<FhirReference>,
}

impl FhirTask {
    /// An empty `Task` resource (only `resourceType` + required `intent`);
    /// build it up field by field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "Task".to_string(),
            id: None,
            meta: None,
            identifier: Vec::new(),
            status: None,
            intent: "order".to_string(),
            priority: None,
            code: None,
            description: None,
            for_: None,
        }
    }
}

impl Default for FhirTask {
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
}

/// FHIR `Identifier` — a `system|value` business identifier, optionally
/// `assigner`-scoped (used to carry the agency of an agency case number).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirIdentifier {
    /// The namespace URI the value is unique within.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The identifier value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
    /// Organization that issued/assigned the id (carries the case's
    /// `agency_id` / `agency_name` for a scoped case number).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub assigner: Option<FhirReference>,
}

/// FHIR `Reference` — a pointer to another resource (subject / assigner).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirReference {
    /// The literal reference (e.g. an `EntityRef` URN `person:<uuid>`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reference: Option<String>,
    /// A human-readable display label.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
}

/// FHIR `CodeableConcept` — one coding under a system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCodeableConcept {
    /// The codings (this service emits exactly one).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub coding: Vec<FhirCoding>,
}

/// FHIR `Coding` — a `system` + `code` pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCoding {
    /// The code system URI.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The code within the system.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub code: Option<String>,
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
    /// Absolute-ish URL identifying the resource (`Task/{id}`).
    #[serde(rename = "fullUrl")]
    pub full_url: String,
    /// The contained resource.
    pub resource: FhirTask,
}

impl FhirBundle {
    /// Assemble a `searchset` Bundle from rendered resources.
    #[must_use]
    pub fn searchset(resources: Vec<FhirTask>) -> Self {
        let entry = resources
            .into_iter()
            .map(|r| FhirBundleEntry {
                full_url: format!("Task/{}", r.id.clone().unwrap_or_default()),
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
