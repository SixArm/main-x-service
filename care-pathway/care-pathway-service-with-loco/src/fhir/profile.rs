//! FHIR **profile** and **terminology** conformance validation.
//!
//! ONC §170.315(g)(10) does not ask whether a resource is well-formed
//! FHIR; it asks whether it conforms to a declared **profile** — the
//! must-support elements, the cardinalities, and above all the
//! **terminology bindings** that say which value set an element's code
//! must come from. That last part is the substance: `"code": "banana"` is
//! perfectly well-formed FHIR and clinically meaningless, and only a
//! binding check rejects it.
//!
//! ## The profile this service claims
//!
//! [`PROFILE_URL`] — a **family-local** `StructureDefinition` canonical,
//! not a US Core one. `PlanDefinition` has no US Core profile, and the
//! family serves FHIR **R5** while certification targets **R4**; claiming
//! `http://hl7.org/fhir/us/core/...` would be a false conformance
//! statement. What is implemented is the *machinery* US Core conformance
//! needs, against a profile this service can actually honour.
//!
//! ## Errors versus warnings
//!
//! - **error** — the resource violates the profile: a missing
//!   must-support element, a broken cardinality, or a code that is not
//!   valid within a system the profile **binds**. These fail `$validate`
//!   and are returned as `OperationOutcome` issues.
//! - **warning** — the resource uses a code system the profile does not
//!   bind (a `Custom` coding system, a foreign identifier namespace).
//!   Unbound is not invalid: the family deliberately preserves foreign
//!   namespaces rather than dropping them ([`crate::fhir::system_to_scheme`]),
//!   so flagging without failing is the correct strength. This mirrors
//!   FHIR's own `example` / `preferred` / `extensible` / `required`
//!   binding strengths, of which only `required` is an error.

use care_pathway_matcher::{CarePathway, CodeSystem};

use crate::fhir::code_system_to_uri;
use crate::fhir::resources::{FhirIssue, FhirPlanDefinition};

/// The `StructureDefinition` canonical this service claims conformance to,
/// stamped into `meta.profile` on every rendered resource.
pub const PROFILE_URL: &str =
    "urn:mxi:carepathway:StructureDefinition/mxi-care-pathway-plandefinition";

/// `PlanDefinition.status` is **required**-bound to the FHIR
/// `publication-status` value set.
pub const PUBLICATION_STATUS: [&str; 4] = ["draft", "active", "retired", "unknown"];

/// Whether this profile **binds** a condition-code system. A code in a
/// bound system must be structurally valid for it; a code in any other
/// system is reported as unbound (a warning), never silently accepted as
/// conformant.
#[must_use]
pub fn is_bound_code_system(system: &CodeSystem) -> bool {
    matches!(
        system,
        CodeSystem::Icd10 | CodeSystem::Icd11 | CodeSystem::Snomed
    )
}

/// Build an `error`-severity issue.
fn error(code: &str, diagnostics: impl Into<String>) -> FhirIssue {
    FhirIssue {
        severity: "error".to_string(),
        code: code.to_string(),
        diagnostics: Some(diagnostics.into()),
    }
}

/// Build a `warning`-severity issue.
fn warning(code: &str, diagnostics: impl Into<String>) -> FhirIssue {
    FhirIssue {
        severity: "warning".to_string(),
        code: code.to_string(),
        diagnostics: Some(diagnostics.into()),
    }
}

/// Validate a wire `PlanDefinition` against [`PROFILE_URL`]: must-support
/// elements, cardinalities, and the `status` binding.
///
/// Structural only — it inspects the FHIR resource as received, before
/// conversion, so a violation is reported against the FHIR element path
/// the client sent rather than against an internal field name.
#[must_use]
pub fn validate_profile(resource: &FhirPlanDefinition) -> Vec<FhirIssue> {
    let mut issues = Vec::new();

    if resource.resource_type != "PlanDefinition" {
        issues.push(error(
            "structure",
            format!(
                "PlanDefinition.resourceType: expected \"PlanDefinition\", found {:?}",
                resource.resource_type
            ),
        ));
    }

    // title 1..1 — the profile's only must-support scalar with no default.
    match resource.title.as_deref().map(str::trim) {
        None | Some("") => issues.push(error(
            "required",
            "PlanDefinition.title: minimum cardinality 1 is not met (must-support)",
        )),
        Some(_) => {}
    }

    // status 1..1, required binding to publication-status.
    match resource.status.as_deref().map(str::trim) {
        None | Some("") => issues.push(error(
            "required",
            "PlanDefinition.status: minimum cardinality 1 is not met (must-support)",
        )),
        Some(status) if !PUBLICATION_STATUS.contains(&status) => issues.push(error(
            "code-invalid",
            format!(
                "PlanDefinition.status: {status:?} is not in the required binding \
                 http://hl7.org/fhir/ValueSet/publication-status ({})",
                PUBLICATION_STATUS.join(" | ")
            ),
        )),
        Some(_) => {}
    }

    // identifier: system and value are both required when an identifier is
    // present — a value with no namespace is not an identifier, it is a
    // string, and treating it as one is how false matches start.
    for (i, identifier) in resource.identifier.iter().enumerate() {
        if identifier
            .system
            .as_deref()
            .is_none_or(|v| v.trim().is_empty())
        {
            issues.push(error(
                "required",
                format!(
                    "PlanDefinition.identifier[{i}].system: required when identifier is present"
                ),
            ));
        }
        if identifier
            .value
            .as_deref()
            .is_none_or(|v| v.trim().is_empty())
        {
            issues.push(error(
                "required",
                format!(
                    "PlanDefinition.identifier[{i}].value: required when identifier is present"
                ),
            ));
        }
    }

    // useContext: code.code and at least one value coding are required.
    for (i, context) in resource.use_context.iter().enumerate() {
        if context
            .code
            .code
            .as_deref()
            .is_none_or(|v| v.trim().is_empty())
        {
            issues.push(error(
                "required",
                format!("PlanDefinition.useContext[{i}].code.code: minimum cardinality 1"),
            ));
        }
        let value = &context.value_codeable_concept;
        if value.coding.is_empty() && value.text.as_deref().is_none_or(|v| v.trim().is_empty()) {
            issues.push(error(
                "required",
                format!(
                    "PlanDefinition.useContext[{i}].valueCodeableConcept: \
                     requires at least one coding or text"
                ),
            ));
        }
    }

    // action.title 1..1 — an action with no title conveys nothing.
    for (i, action) in resource.action.iter().enumerate() {
        if action.title.as_deref().is_none_or(|v| v.trim().is_empty()) {
            issues.push(error(
                "required",
                format!("PlanDefinition.action[{i}].title: minimum cardinality 1"),
            ));
        }
    }

    // relatedArtifact.url 1..1 when present.
    for (i, artifact) in resource.related_artifact.iter().enumerate() {
        if artifact.url.as_deref().is_none_or(|v| v.trim().is_empty()) {
            issues.push(error(
                "required",
                format!("PlanDefinition.relatedArtifact[{i}].url: minimum cardinality 1"),
            ));
        }
    }

    issues
}

/// Validate the terminology of a converted pathway: each condition code
/// against the value set its system binds.
///
/// A code in a **bound** system that is not valid for that system is an
/// `error` — this is the check that separates "well-formed FHIR" from
/// "clinically meaningful". A code in an unbound system is a `warning`.
#[must_use]
pub fn validate_terminology(pathway: &CarePathway) -> Vec<FhirIssue> {
    let mut issues = Vec::new();
    for (i, code) in pathway.condition_codes.iter().enumerate() {
        let uri = code_system_to_uri(&code.system);
        if !is_bound_code_system(&code.system) {
            issues.push(warning(
                "code-invalid",
                format!(
                    "PlanDefinition.useContext[{i}].valueCodeableConcept.coding.system: \
                     {uri} is not bound by {PROFILE_URL}; the code is preserved but not validated"
                ),
            ));
            continue;
        }
        if let Some(problem) = crate::validation::condition_code_issue(code) {
            issues.push(error(
                "code-invalid",
                format!(
                    "PlanDefinition.useContext[{i}].valueCodeableConcept.coding.code: \
                     {problem} (system {uri})"
                ),
            ));
        }
    }
    issues
}

/// Run the full conformance pass a `$validate` invocation performs:
/// profile structure, then terminology, then the service's own payload
/// validators (input caps, identifier shapes, BCP-47 language tags).
///
/// The service validators are included deliberately: a resource that
/// passes FHIR conformance but would be rejected on `POST` is not
/// "valid" from the caller's point of view, and reporting that at
/// `$validate` time is the whole reason the operation exists.
#[must_use]
pub fn validate_all(resource: &FhirPlanDefinition, pathway: &CarePathway) -> Vec<FhirIssue> {
    let mut issues = validate_profile(resource);
    issues.extend(validate_terminology(pathway));
    issues.extend(
        crate::validation::problems(pathway)
            .into_iter()
            .map(|p| error("processing", p)),
    );
    issues
}

/// Whether a set of issues contains anything that should fail validation.
/// Warnings (unbound systems) do not.
#[must_use]
pub fn has_errors(issues: &[FhirIssue]) -> bool {
    issues
        .iter()
        .any(|i| i.severity == "error" || i.severity == "fatal")
}

#[cfg(test)]
mod tests {
    use super::*;
    use care_pathway_matcher::ConditionCode;

    use crate::fhir::resources::{
        FhirCodeableConcept, FhirCoding, FhirIdentifier, FhirPlanAction, FhirRelatedArtifact,
        FhirUsageContext,
    };
    use crate::fhir::to_fhir_plan_definition;

    /// A resource rendered by this service conforms to its own profile —
    /// the round-trip pin that keeps the renderer and the profile honest
    /// about each other.
    #[test]
    fn rendered_resources_conform_to_the_profile() {
        let pathway = CarePathway {
            condition_codes: vec![ConditionCode {
                system: CodeSystem::Icd10,
                code: "I63.9".to_string(),
            }],
            interventions: vec!["Thrombolysis".to_string()],
            ..CarePathway::new("Acute Stroke Care Pathway")
        };
        let resource = to_fhir_plan_definition(&pathway, "pid-1", true, None);
        let issues = validate_all(&resource, &pathway);
        assert!(!has_errors(&issues), "{issues:?}");
    }

    /// A soft-deleted record renders as `retired`, which is inside the
    /// required binding.
    #[test]
    fn retired_status_is_within_the_binding() {
        let pathway = CarePathway::new("Retired Pathway");
        let resource = to_fhir_plan_definition(&pathway, "pid-1", false, None);
        assert_eq!(resource.status.as_deref(), Some("retired"));
        assert!(!has_errors(&validate_profile(&resource)));
    }

    /// A missing title breaks the must-support cardinality.
    #[test]
    fn missing_title_is_an_error() {
        let mut resource = FhirPlanDefinition::new();
        resource.status = Some("active".to_string());
        let issues = validate_profile(&resource);
        assert!(has_errors(&issues));
        assert!(
            issues
                .iter()
                .any(|i| i.diagnostics.as_deref().unwrap_or("").contains("title")),
            "{issues:?}"
        );
    }

    /// A status outside the required binding is a `code-invalid` error —
    /// well-formed JSON, non-conformant FHIR.
    #[test]
    fn status_outside_the_binding_is_an_error() {
        let mut resource = FhirPlanDefinition::new();
        resource.title = Some("X".to_string());
        resource.status = Some("published".to_string());
        let issues = validate_profile(&resource);
        assert!(has_errors(&issues));
        assert!(
            issues.iter().any(|i| i.code == "code-invalid"),
            "{issues:?}"
        );
    }

    /// The wrong `resourceType` is caught even though Serde accepted it.
    #[test]
    fn wrong_resource_type_is_an_error() {
        let mut resource = FhirPlanDefinition::new();
        resource.resource_type = "Patient".to_string();
        resource.title = Some("X".to_string());
        resource.status = Some("active".to_string());
        assert!(has_errors(&validate_profile(&resource)));
    }

    /// An identifier with no system is an error: a bare value is not an
    /// identifier, and treating it as one is how false matches begin.
    #[test]
    fn identifier_without_a_system_is_an_error() {
        let mut resource = FhirPlanDefinition::new();
        resource.title = Some("X".to_string());
        resource.status = Some("active".to_string());
        resource.identifier = vec![FhirIdentifier {
            system: None,
            value: Some("12345".to_string()),
        }];
        let issues = validate_profile(&resource);
        assert!(has_errors(&issues));
        assert!(
            issues
                .iter()
                .any(|i| i.diagnostics.as_deref().unwrap_or("").contains("system")),
            "{issues:?}"
        );
    }

    /// Empty `action.title`, empty `useContext` values, and empty artifact
    /// URLs each break their cardinality.
    #[test]
    fn empty_nested_elements_break_cardinality() {
        let mut resource = FhirPlanDefinition::new();
        resource.title = Some("X".to_string());
        resource.status = Some("active".to_string());
        resource.action = vec![FhirPlanAction {
            title: Some("   ".to_string()),
        }];
        resource.use_context = vec![FhirUsageContext {
            code: FhirCoding {
                system: None,
                code: None,
                display: None,
            },
            value_codeable_concept: FhirCodeableConcept {
                coding: Vec::new(),
                text: None,
            },
        }];
        resource.related_artifact = vec![FhirRelatedArtifact {
            artifact_type: Some("documentation".to_string()),
            url: None,
        }];
        let issues = validate_profile(&resource);
        assert!(issues.len() >= 4, "{issues:?}");
        assert!(has_errors(&issues));
    }

    /// **The terminology check that matters**: a structurally invalid code
    /// in a bound system is an error, not a shrug.
    #[test]
    fn invalid_code_in_a_bound_system_is_an_error() {
        for (system, code) in [
            (CodeSystem::Icd10, "banana"),
            (CodeSystem::Icd11, "!!!"),
            (CodeSystem::Snomed, "12"),
        ] {
            let pathway = CarePathway {
                condition_codes: vec![ConditionCode {
                    system,
                    code: code.to_string(),
                }],
                ..CarePathway::new("X")
            };
            let issues = validate_terminology(&pathway);
            assert!(has_errors(&issues), "{code} in a bound system must fail");
            assert_eq!(issues[0].code, "code-invalid");
        }
    }

    /// A valid code in each bound system passes.
    #[test]
    fn valid_codes_in_bound_systems_pass() {
        let pathway = CarePathway {
            condition_codes: vec![
                ConditionCode {
                    system: CodeSystem::Icd10,
                    code: "I63.9".to_string(),
                },
                ConditionCode {
                    system: CodeSystem::Snomed,
                    code: "422504002".to_string(),
                },
            ],
            ..CarePathway::new("X")
        };
        let issues = validate_terminology(&pathway);
        assert!(!has_errors(&issues), "{issues:?}");
    }

    /// An **unbound** system warns rather than failing: the family
    /// deliberately preserves foreign namespaces, so rejecting them would
    /// contradict the conversion contract.
    #[test]
    fn unbound_system_warns_but_does_not_fail() {
        let pathway = CarePathway {
            condition_codes: vec![ConditionCode {
                system: CodeSystem::Custom("local-registry".to_string()),
                code: "anything at all".to_string(),
            }],
            ..CarePathway::new("X")
        };
        let issues = validate_terminology(&pathway);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, "warning");
        assert!(!has_errors(&issues));
    }

    /// `validate_all` also surfaces the service's own payload rules, so a
    /// resource that would be rejected on `POST` is not reported as valid.
    #[test]
    fn validate_all_includes_service_validators() {
        let pathway = CarePathway {
            in_language: vec!["not a bcp47 tag!".to_string()],
            ..CarePathway::new("X")
        };
        let resource = to_fhir_plan_definition(&pathway, "pid-1", true, None);
        assert!(!has_errors(&validate_profile(&resource)), "profile is fine");
        assert!(
            has_errors(&validate_all(&resource, &pathway)),
            "but the service would reject it"
        );
    }

    /// The claimed profile is family-local. Claiming a US Core canonical
    /// would be a false conformance statement (see the module docs), so
    /// pin it.
    #[test]
    fn claimed_profile_is_not_a_us_core_canonical() {
        assert!(PROFILE_URL.starts_with("urn:mxi:"));
        assert!(!PROFILE_URL.contains("hl7.org/fhir/us/core"));
    }
}
