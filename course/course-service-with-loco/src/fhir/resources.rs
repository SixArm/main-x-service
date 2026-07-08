//! FHIR R5 wire types for the **non-standard** `Basic` Course wrapper.
//!
//! There is **no standard FHIR R5 resource for an educational course**, so
//! the Course registry is exposed as a deliberately **non-standard** FHIR
//! `Basic` resource (per [`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3, `best-effort` fidelity): `code` carries a local coding
//! `{system: "urn:mxi:resource", code: "course"}` and a documented profile,
//! the course name + educational level + keywords + `teaches` ride in
//! documented extensions, and `course_code` + `CourseIdentifier`s become
//! `identifier` tokens. This is **not interoperable** with a standards-only
//! FHIR client; it is a courtesy shape, clearly labelled.
//!
//! Slim, self-contained Serde structs for exactly the elements this service
//! populates, plus the shared envelope types every FHIR endpoint returns:
//! [`FhirOperationOutcome`] for errors and [`FhirBundle`] for search sets.
//! Field names and casing follow FHIR JSON (`resourceType`, `valueString`,
//! `fullUrl`). Absent optionals and empty arrays are omitted.
//!
//! These types are **copied per project** (drift-accepted, the family's
//! `mxi-events`/`EntityRef` posture); they are not a shared crate.

use serde::{Deserialize, Serialize};

/// FHIR `resourceType` discriminator for the Course wrapper — the
/// **non-standard** `Basic` resource.
pub const RESOURCE_TYPE: &str = "Basic";
/// Coding system for the `Basic.code` that marks this as a course.
pub const RESOURCE_CODE_SYSTEM: &str = "urn:mxi:resource";
/// Coding code for the `Basic.code` that marks this as a course.
pub const RESOURCE_CODE: &str = "course";
/// The documented (non-standard) profile URL advertised in `meta.profile`
/// and the `CapabilityStatement`.
pub const PROFILE_URL: &str = "https://sixarm.com/fhir/StructureDefinition/mxi-course";

/// A **non-standard** FHIR R5 `Basic` resource wrapping a Course.
///
/// `Basic` is FHIR's escape hatch for a concept with no dedicated resource:
/// its only meaningful required element is `code`. Everything course-specific
/// is carried in `identifier` tokens and documented `extension`s (see the
/// module docs and the `mod.rs` conversion functions for the gaps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirBasic {
    /// FHIR resource-type discriminator — always `"Basic"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical id (the record's `pid`). Absent on an inbound create.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    /// Resource metadata (last-updated + the non-standard profile).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (`course_code`, DOI, Wikidata, …) as
    /// `system|value` tokens.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifier: Vec<FhirIdentifier>,
    /// The `Basic.code` marking the resource kind — the local
    /// `{urn:mxi:resource | course}` coding. Defaulted on parse so a
    /// caller that omits it still deserialises (we re-assert it on output).
    #[serde(default)]
    pub code: FhirCodeableConcept,
    /// Course-specific data with no `Basic` home: the course name
    /// (`urn:mxi:course:name`), educational level, keywords, and `teaches`.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub extension: Vec<FhirExtension>,
}

impl FhirBasic {
    /// An empty course `Basic` resource — `resourceType` set and `code`
    /// pre-populated with the `{urn:mxi:resource | course}` coding; build
    /// the rest up field by field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: RESOURCE_TYPE.to_string(),
            id: None,
            meta: None,
            identifier: Vec::new(),
            code: FhirCodeableConcept::course(),
            extension: Vec::new(),
        }
    }
}

impl Default for FhirBasic {
    fn default() -> Self {
        Self::new()
    }
}

/// FHIR `Meta` — the subset we populate (`lastUpdated` + `profile`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirMeta {
    /// Version id — not tracked yet (no `_history`), always absent.
    #[serde(rename = "versionId", skip_serializing_if = "Option::is_none", default)]
    pub version_id: Option<String>,
    /// Last-updated instant (the record's `updated_at`).
    #[serde(rename = "lastUpdated", skip_serializing_if = "Option::is_none", default)]
    pub last_updated: Option<String>,
    /// Asserted profiles — carries the non-standard course profile URL.
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

/// FHIR `CodeableConcept` — the subset carrying `Basic.code`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FhirCodeableConcept {
    /// One or more codings; we emit exactly the course coding.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub coding: Vec<FhirCoding>,
    /// Human-readable label.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text: Option<String>,
}

impl FhirCodeableConcept {
    /// The `{urn:mxi:resource | course}` concept that marks a `Basic`
    /// resource as an MXI course.
    #[must_use]
    pub fn course() -> Self {
        Self {
            coding: vec![FhirCoding {
                system: Some(RESOURCE_CODE_SYSTEM.to_string()),
                code: Some(RESOURCE_CODE.to_string()),
                display: Some("Course (non-standard MXI resource)".to_string()),
            }],
            text: Some("Course".to_string()),
        }
    }
}

/// FHIR `Coding` — a `system` + `code` (+ optional `display`) triple.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FhirCoding {
    /// The code system URI.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The code within the system.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub code: Option<String>,
    /// Human-readable display for the code.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
}

/// A FHIR `Extension` carrying one `valueString`. Used for the
/// course-specific fields `Basic` cannot model natively (name, level,
/// keywords, `teaches`). Repeatable fields emit one extension per value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirExtension {
    /// The extension definition URL (a `urn:mxi:course:*` namespace).
    pub url: String,
    /// The string value.
    #[serde(rename = "valueString", skip_serializing_if = "Option::is_none", default)]
    pub value_string: Option<String>,
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
    /// Relative URL identifying the resource (`Basic/{id}`).
    #[serde(rename = "fullUrl")]
    pub full_url: String,
    /// The contained resource.
    pub resource: FhirBasic,
}

impl FhirBundle {
    /// Assemble a `searchset` Bundle from rendered resources.
    #[must_use]
    pub fn searchset(resources: Vec<FhirBasic>) -> Self {
        let entry = resources
            .into_iter()
            .map(|r| FhirBundleEntry {
                full_url: format!("Basic/{}", r.id.clone().unwrap_or_default()),
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
