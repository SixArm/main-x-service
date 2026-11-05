//! FHIR R5 wire types for the `Appointment` resource.
//!
//! Slim, self-contained Serde structs for exactly the elements this
//! service populates (per [`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3, **`low` / best-effort fidelity** — schema.org/Event has no clean
//! FHIR analog) plus the shared envelope types every FHIR endpoint
//! returns: [`FhirOperationOutcome`] for errors and [`FhirBundle`] for
//! search sets. Field names and casing follow FHIR JSON (`resourceType`,
//! `fullUrl`, `lastUpdated`, and the reserved-word rename `type`). Absent
//! optionals and empty arrays are omitted so resources stay clean.
//!
//! These types are **copied per project** (drift-accepted, the family's
//! `mxi-events`/`EntityRef` posture); they are not a shared crate.

use serde::{Deserialize, Serialize};

/// A FHIR R5 `Appointment` resource (the elements this service maps).
///
/// This is a deliberately **best-effort** projection of a
/// schema.org/Event: only the time window, a title, the lifecycle
/// status, the parties, the locations, and the external identifiers are
/// carried. Many `Appointment` elements (`serviceType`, `specialty`,
/// `slot`, `minutesDuration`, …) have no `Event` source and are omitted;
/// many `Event` fields (`keywords`, `offers`, capacity, `super_event`, …)
/// have no `Appointment` home and are dropped. See
/// [`crate::fhir::from_fhir_appointment`] for the documented gap list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirAppointment {
    /// FHIR resource-type discriminator — always `"Appointment"`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical id (the record's `pid`). Absent on an inbound create.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    /// Resource metadata (version / last-updated).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<FhirMeta>,
    /// Business identifiers (booking / ticket / encounter …) as
    /// `system|value` tokens.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifier: Vec<FhirIdentifier>,
    /// Lifecycle status (`booked` / `cancelled` / `fulfilled` / …).
    /// Required by FHIR; always populated from `event_status`.
    pub status: String,
    /// Human-readable title — carries the event's `name` (best-effort;
    /// FHIR `Appointment` has no dedicated title element).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Start of the appointment window (`event.start_date`, RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start: Option<String>,
    /// End of the appointment window (`event.end_date`, RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub end: Option<String>,
    /// Participants: the event's parties and locations, each tagged with
    /// a role coding (§ [`crate::fhir`]).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub participant: Vec<FhirParticipant>,
}

impl FhirAppointment {
    /// An empty `Appointment` resource (`resourceType` + the required
    /// `status`, defaulted to `"proposed"`); build it up field by field.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "Appointment".to_string(),
            id: None,
            meta: None,
            identifier: Vec::new(),
            status: "proposed".to_string(),
            description: None,
            start: None,
            end: None,
            participant: Vec::new(),
        }
    }
}

impl Default for FhirAppointment {
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

/// FHIR `Identifier` — a `system|value` business identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirIdentifier {
    /// The namespace URI the value is unique within (derived from the
    /// event identifier's category — see [`crate::fhir`]).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The identifier value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// FHIR `Appointment.participant` — one party or location taking part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirParticipant {
    /// Role coding(s): the family `urn:mxi:event:party-role` system with
    /// a `organizer` / `performer` / `attendee` / `location` code.
    #[serde(rename = "type", skip_serializing_if = "Vec::is_empty", default)]
    pub role_type: Vec<FhirCodeableConcept>,
    /// Who / what is participating (display name + optional reference).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub actor: Option<FhirReference>,
    /// Participation status — required by FHIR; we always emit
    /// `"accepted"`.
    pub status: String,
}

/// FHIR `CodeableConcept` — a set of codings for one concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCodeableConcept {
    /// The codings comprising this concept.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub coding: Vec<FhirCoding>,
}

/// FHIR `Coding` — a `system` + `code` pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirCoding {
    /// The code system URI.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system: Option<String>,
    /// The code within that system.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub code: Option<String>,
}

/// FHIR `Reference` — a pointer to (or display of) another resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirReference {
    /// A literal `Type/id` reference, when the party carries an id.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reference: Option<String>,
    /// The referenced resource type (`Person` / `Organization` / …) —
    /// carries the party kind even when there is no id.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub ref_type: Option<String>,
    /// Human-readable display (the party or location name).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
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
    /// Absolute-ish URL identifying the resource (`Appointment/{id}`).
    #[serde(rename = "fullUrl")]
    pub full_url: String,
    /// The contained resource.
    pub resource: FhirAppointment,
}

impl FhirBundle {
    /// Assemble a `searchset` Bundle from rendered resources.
    #[must_use]
    pub fn searchset(resources: Vec<FhirAppointment>) -> Self {
        let entry = resources
            .into_iter()
            .map(|r| FhirBundleEntry {
                full_url: format!("Appointment/{}", r.id.clone().unwrap_or_default()),
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
