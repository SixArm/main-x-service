//! HL7 FHIR R5 interop for the `Task` resource (best-effort case mapping).
//!
//! The case service's adoption of the family FHIR contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)) — copied
//! from the organization reference and adapted for a **`low`-fidelity**
//! `Task`. It provides the bidirectional mapping between the stored
//! `case_matcher::Case` DTO and the wire-level [`resources::FhirTask`]:
//! [`to_fhir_task`] for outbound responses and [`from_fhir_task`] for
//! inbound requests. Resource shapes live in [`resources`], search-parameter
//! parsing in [`search`], and the mounted Axum endpoints in
//! [`crate::controllers::fhir`].
//!
//! Conversions are **lossy where the DTO has no FHIR home** — documented
//! inline and gathered in [`from_fhir_task`]'s doc — never silent.
//!
//! ## Sensitivity
//!
//! A governmental case's subject person (`Task.for`) inherits the elevated
//! `case ↔ person` (`subject_of`) governance
//! ([fhir.md §8](../../../../agents/share/fhir.md),
//! [cross-service-linking.md §10](../../../../agents/share/cross-service-linking.md)).
//! The case service has **no field-masking layer implemented today**, so —
//! consistent with the crate's existing deferred privacy layer — masking
//! the subject reference for unauthorised callers is a documented
//! **DEFERRED** gap (tracked by the spec §13 FHIR task), not implemented
//! here. The blanket auth+ABAC guard still gates `/fhir/*`.

/// FHIR resource + envelope wire types (`Task`, `OperationOutcome`,
/// `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use case_matcher::{Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, Priority};
use resources::{
    FhirCodeableConcept, FhirCoding, FhirIdentifier, FhirMeta, FhirReference, FhirTask,
};

/// FHIR `system` URI for the case's agency-scoped `case_number` field.
/// Distinct from the [`IdentifierScheme::AgencyCaseNumber`] system so the
/// two round-trip without collision.
const CASE_NUMBER_SYSTEM: &str = "urn:mxi:case:case-number";
/// FHIR code-system URI for the `case_type` → `Task.code` coding.
const CASE_TYPE_SYSTEM: &str = "urn:mxi:case:case-type";

/// Map an identifier `scheme` to the FHIR `identifier.system` URI.
/// [`system_to_scheme`] is the exact inverse, so a scheme round-trips
/// through FHIR unchanged.
#[must_use]
pub fn scheme_to_system(scheme: &IdentifierScheme) -> String {
    match scheme {
        IdentifierScheme::Docket => "urn:mxi:case:docket".to_string(),
        IdentifierScheme::ExternalCaseId => "urn:mxi:case:external-case-id".to_string(),
        IdentifierScheme::Uri => "urn:mxi:case:uri".to_string(),
        IdentifierScheme::Uuid => "urn:mxi:case:uuid".to_string(),
        IdentifierScheme::AgencyCaseNumber => "urn:mxi:case:agency-case-number".to_string(),
        IdentifierScheme::LocalId => "urn:mxi:case:local-id".to_string(),
        IdentifierScheme::Custom(label) => format!("urn:mxi:case:custom:{label}"),
    }
}

/// Map a FHIR `identifier.system` URI back to an identifier `scheme` — the
/// inverse of [`scheme_to_system`]. An unrecognised system is preserved as
/// `Custom(system)` so an inbound identifier from a foreign namespace is
/// never dropped.
#[must_use]
pub fn system_to_scheme(system: &str) -> IdentifierScheme {
    match system {
        "urn:mxi:case:docket" => IdentifierScheme::Docket,
        "urn:mxi:case:external-case-id" => IdentifierScheme::ExternalCaseId,
        "urn:mxi:case:uri" => IdentifierScheme::Uri,
        "urn:mxi:case:uuid" => IdentifierScheme::Uuid,
        "urn:mxi:case:agency-case-number" => IdentifierScheme::AgencyCaseNumber,
        "urn:mxi:case:local-id" => IdentifierScheme::LocalId,
        other => other.strip_prefix("urn:mxi:case:custom:").map_or_else(
            || IdentifierScheme::Custom(other.to_string()),
            |label| IdentifierScheme::Custom(label.to_string()),
        ),
    }
}

/// Map a `case_type` to a `Task.code` coding `code`. [`code_to_case_type`]
/// is the exact inverse (unknown codes become `Custom`), so `case_type`
/// round-trips.
#[must_use]
pub fn case_type_to_code(t: &CaseType) -> String {
    match t {
        CaseType::Benefit => "benefit".to_string(),
        CaseType::Legal => "legal".to_string(),
        CaseType::SocialServices => "social-services".to_string(),
        CaseType::Healthcare => "healthcare".to_string(),
        CaseType::Housing => "housing".to_string(),
        CaseType::Immigration => "immigration".to_string(),
        CaseType::Licensing => "licensing".to_string(),
        CaseType::Complaint => "complaint".to_string(),
        CaseType::Appeal => "appeal".to_string(),
        CaseType::Investigation => "investigation".to_string(),
        CaseType::Tax => "tax".to_string(),
        CaseType::Employment => "employment".to_string(),
        CaseType::Custom(label) => label.clone(),
    }
}

/// Inverse of [`case_type_to_code`]. An unrecognised code becomes
/// `Custom(code)`, so a foreign code is never dropped.
#[must_use]
pub fn code_to_case_type(code: &str) -> CaseType {
    match code {
        "benefit" => CaseType::Benefit,
        "legal" => CaseType::Legal,
        "social-services" => CaseType::SocialServices,
        "healthcare" => CaseType::Healthcare,
        "housing" => CaseType::Housing,
        "immigration" => CaseType::Immigration,
        "licensing" => CaseType::Licensing,
        "complaint" => CaseType::Complaint,
        "appeal" => CaseType::Appeal,
        "investigation" => CaseType::Investigation,
        "tax" => CaseType::Tax,
        "employment" => CaseType::Employment,
        other => CaseType::Custom(other.to_string()),
    }
}

/// Map a [`CaseStatus`] to a FHIR `Task.status` code.
///
/// **Lossy** (documented): `Closed` and `Resolved` both map to
/// `completed`, and a `Custom(label)` status maps to `draft` (the label is
/// dropped). See [`fhir_status_to_case`] for which statuses round-trip.
#[must_use]
pub fn case_status_to_fhir(s: &CaseStatus) -> &'static str {
    match s {
        CaseStatus::Open => "requested",
        CaseStatus::InProgress => "in-progress",
        CaseStatus::Pending => "received",
        CaseStatus::OnHold => "on-hold",
        CaseStatus::Closed | CaseStatus::Resolved => "completed",
        CaseStatus::Rejected => "rejected",
        CaseStatus::Withdrawn => "cancelled",
        CaseStatus::Custom(_) => "draft",
    }
}

/// Map a FHIR `Task.status` code back to a [`CaseStatus`]. Returns `None`
/// for an unrecognised status. `completed` resolves to `Closed` (the
/// `Closed`/`Resolved` collision in [`case_status_to_fhir`]).
#[must_use]
pub fn fhir_status_to_case(status: &str) -> Option<CaseStatus> {
    match status {
        "requested" => Some(CaseStatus::Open),
        "in-progress" => Some(CaseStatus::InProgress),
        "received" => Some(CaseStatus::Pending),
        "on-hold" => Some(CaseStatus::OnHold),
        "completed" => Some(CaseStatus::Closed),
        "rejected" => Some(CaseStatus::Rejected),
        "cancelled" => Some(CaseStatus::Withdrawn),
        _ => None,
    }
}

/// Map a [`Priority`] to a FHIR `RequestPriority` code.
///
/// **Lossy** (documented): `Low` and `Normal` both map to `routine`; the
/// inverse resolves `routine` to `Normal`, so `Low` does not round-trip.
#[must_use]
pub fn priority_to_fhir(p: &Priority) -> &'static str {
    match p {
        Priority::Low | Priority::Normal => "routine",
        Priority::High => "urgent",
        Priority::Urgent => "stat",
    }
}

/// Map a FHIR `RequestPriority` code back to a [`Priority`]. Returns `None`
/// for an unrecognised priority. `routine` resolves to `Normal`.
#[must_use]
pub fn fhir_priority_to_case(priority: &str) -> Option<Priority> {
    match priority {
        "routine" => Some(Priority::Normal),
        "urgent" | "asap" => Some(Priority::High),
        "stat" => Some(Priority::Urgent),
        _ => None,
    }
}

/// Render a stored [`Case`] as a FHIR R5 [`FhirTask`].
///
/// `id` is the record's public `pid`; `last_updated` comes from the row.
/// `title` → `description`; `status` → `status`; `priority` → `priority`;
/// `case_type` → `code`; the agency-scoped `case_number` (+ `agency_id` /
/// `agency_name`) → an `assigner`-scoped `identifier`; each `identifiers`
/// entry → an `identifier` token; the first `subjects` entry → `for`.
///
/// **Fidelity gaps** (no FHIR `Task` home): `alternate_titles`,
/// `opened_date`, `keywords`, `same_as`, `in_language`, and the second and
/// later `subjects` are not emitted.
#[must_use]
pub fn to_fhir_task(case: &Case, id: &str, last_updated: Option<String>) -> FhirTask {
    let mut fhir = FhirTask::new();
    fhir.id = Some(id.to_string());
    fhir.meta = last_updated.map(|lu| FhirMeta {
        version_id: None,
        last_updated: Some(lu),
    });
    fhir.description = Some(case.title.clone());
    fhir.status = case.status.as_ref().map(|s| case_status_to_fhir(s).to_string());
    fhir.priority = case.priority.as_ref().map(|p| priority_to_fhir(p).to_string());
    fhir.code = case.case_type.as_ref().map(|t| FhirCodeableConcept {
        coding: vec![FhirCoding {
            system: Some(CASE_TYPE_SYSTEM.to_string()),
            code: Some(case_type_to_code(t)),
        }],
    });

    let mut identifier = Vec::new();
    // Agency-scoped case number, carrying agency in `assigner`.
    if let Some(ref number) = case.case_number {
        let assigner = (case.agency_id.is_some() || case.agency_name.is_some()).then(|| {
            FhirReference {
                reference: case.agency_id.clone(),
                display: case.agency_name.clone(),
            }
        });
        identifier.push(FhirIdentifier {
            system: Some(CASE_NUMBER_SYSTEM.to_string()),
            value: Some(number.clone()),
            assigner,
        });
    }
    // General external identifiers.
    identifier.extend(case.identifiers.iter().map(|id| FhirIdentifier {
        system: Some(scheme_to_system(&id.scheme)),
        value: Some(id.value.clone()),
        assigner: None,
    }));
    fhir.identifier = identifier;

    // First subject → `for` (high-sensitivity; masking deferred — see
    // the module doc).
    if let Some(subject) = case.subjects.first() {
        fhir.for_ = Some(FhirReference {
            reference: Some(subject.clone()),
            display: None,
        });
    }

    fhir
}

/// Parse an inbound [`FhirTask`] into a stored [`Case`].
///
/// `description` is required (a FHIR `Task` with no `description` is a
/// `400`) and maps to `title`. `status`/`priority`/`code` map back to
/// `status`/`priority`/`case_type`; a `CASE_NUMBER_SYSTEM` identifier maps
/// back to `case_number` (+ `agency_id`/`agency_name` from its
/// `assigner`); every other identifier maps to `identifiers`; a `for`
/// reference maps to a single-element `subjects`.
///
/// **Fidelity gaps** (the resource can't carry them, or the reverse is
/// lossy): `alternate_titles`, `opened_date`, `keywords`, `same_as`, and
/// `in_language` default to empty; a `Closed`/`Resolved` status both
/// resolve to `Closed`; a `Custom` status is not recovered; `Low` priority
/// resolves to `Normal`; only a single subject is recovered.
///
/// # Errors
///
/// Returns the missing-`description` diagnostic string when the resource
/// has no non-empty `description`.
pub fn from_fhir_task(fhir: &FhirTask) -> Result<Case, String> {
    let title = fhir
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .ok_or_else(|| "Task.description is required".to_string())?;

    let mut case = Case::new(title);
    case.status = fhir.status.as_deref().and_then(fhir_status_to_case);
    case.priority = fhir.priority.as_deref().and_then(fhir_priority_to_case);
    case.case_type = fhir
        .code
        .as_ref()
        .and_then(|c| c.coding.first())
        .and_then(|c| c.code.as_deref())
        .map(code_to_case_type);

    let mut identifiers = Vec::new();
    for id in &fhir.identifier {
        let Some(value) = id.value.clone() else {
            continue;
        };
        let system = id.system.as_deref().unwrap_or_default();
        if system == CASE_NUMBER_SYSTEM {
            case.case_number = Some(value);
            if let Some(ref assigner) = id.assigner {
                case.agency_id.clone_from(&assigner.reference);
                case.agency_name.clone_from(&assigner.display);
            }
        } else {
            identifiers.push(CaseIdentifier {
                scheme: system_to_scheme(system),
                value,
            });
        }
    }
    case.identifiers = identifiers;

    if let Some(reference) = fhir.for_.as_ref().and_then(|r| r.reference.clone()) {
        case.subjects = vec![reference];
    }

    Ok(case)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every identifier scheme round-trips through the FHIR `system` URI
    /// unchanged (`scheme → system → scheme` is the identity).
    #[test]
    fn scheme_system_round_trips() {
        let schemes = [
            IdentifierScheme::Docket,
            IdentifierScheme::ExternalCaseId,
            IdentifierScheme::Uri,
            IdentifierScheme::Uuid,
            IdentifierScheme::AgencyCaseNumber,
            IdentifierScheme::LocalId,
            IdentifierScheme::Custom("bespoke".to_string()),
        ];
        for scheme in schemes {
            let system = scheme_to_system(&scheme);
            assert_eq!(system_to_scheme(&system), scheme, "round-trip {scheme:?}");
        }
    }

    /// An unknown inbound `system` is preserved as `Custom(system)`.
    #[test]
    fn unknown_system_becomes_custom() {
        assert_eq!(
            system_to_scheme("https://example.org/foo"),
            IdentifierScheme::Custom("https://example.org/foo".to_string())
        );
    }

    /// `case_type` round-trips through the `Task.code` coding, and an
    /// unknown code becomes `Custom`.
    #[test]
    fn case_type_code_round_trips() {
        for t in [
            CaseType::Benefit,
            CaseType::Legal,
            CaseType::SocialServices,
            CaseType::Tax,
            CaseType::Custom("licence-review".to_string()),
        ] {
            assert_eq!(code_to_case_type(&case_type_to_code(&t)), t, "{t:?}");
        }
        assert_eq!(
            code_to_case_type("unheard-of"),
            CaseType::Custom("unheard-of".to_string())
        );
    }

    /// The losslessly-reversible fields survive `DTO → FHIR → DTO`,
    /// including the agency-scoped case number and a subject reference.
    #[test]
    fn dto_fhir_round_trip_preserves_core_fields() {
        let mut case = Case::new("Housing benefit appeal — J. Smith");
        case.case_number = Some("HB-2026-01".to_string());
        case.agency_id = Some("organization:9a2f".to_string());
        case.agency_name = Some("Springfield Housing Authority".to_string());
        case.case_type = Some(CaseType::Housing);
        case.status = Some(CaseStatus::InProgress);
        case.priority = Some(Priority::High);
        case.subjects = vec!["person:0c4f".to_string()];
        case.identifiers = vec![CaseIdentifier {
            scheme: IdentifierScheme::Docket,
            value: "CV-2024-001234".to_string(),
        }];

        let fhir = to_fhir_task(&case, "pid-123", None);
        assert_eq!(fhir.resource_type, "Task");
        assert_eq!(fhir.intent, "order");
        assert_eq!(fhir.id.as_deref(), Some("pid-123"));

        let back = from_fhir_task(&fhir).expect("valid resource");
        assert_eq!(back.title, case.title);
        assert_eq!(back.case_number, case.case_number);
        assert_eq!(back.agency_id, case.agency_id);
        assert_eq!(back.agency_name, case.agency_name);
        assert_eq!(back.case_type, case.case_type);
        assert_eq!(back.status, case.status);
        assert_eq!(back.priority, case.priority);
        assert_eq!(back.subjects, case.subjects);
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].scheme, IdentifierScheme::Docket);
        assert_eq!(back.identifiers[0].value, "CV-2024-001234");
    }

    /// A resource with no `description` is rejected (maps to a `400`).
    #[test]
    fn missing_description_is_rejected() {
        let fhir = FhirTask::new();
        assert!(from_fhir_task(&fhir).is_err());
    }

    /// The documented status collision: `Closed` and `Resolved` both emit
    /// `completed`, which resolves back to `Closed`.
    #[test]
    fn closed_and_resolved_collide_on_completed() {
        assert_eq!(case_status_to_fhir(&CaseStatus::Closed), "completed");
        assert_eq!(case_status_to_fhir(&CaseStatus::Resolved), "completed");
        assert_eq!(fhir_status_to_case("completed"), Some(CaseStatus::Closed));
    }
}
