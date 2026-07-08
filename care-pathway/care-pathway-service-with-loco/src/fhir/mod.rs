//! HL7 FHIR R5 interop for the `PlanDefinition` resource.
//!
//! Adopts the family FHIR contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md), `medium`
//! fidelity) for the care-pathway service: a clinical pathway *template*
//! maps to a FHIR `PlanDefinition`. It provides the bidirectional mapping
//! between the stored `care_pathway_matcher::CarePathway` DTO and the
//! wire-level [`resources::FhirPlanDefinition`]:
//! [`to_fhir_plan_definition`] for outbound responses and
//! [`from_fhir_plan_definition`] for inbound requests. Resource shapes
//! live in [`resources`], search-parameter parsing in [`search`], and the
//! mounted Axum endpoints in [`crate::controllers::fhir`].
//!
//! Conversions are **lossy where the DTO has no FHIR home** — documented
//! inline and gathered in [`from_fhir_plan_definition`]'s doc — never
//! silent.

/// FHIR resource + envelope wire types (`PlanDefinition`,
/// `OperationOutcome`, `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use care_pathway_matcher::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, PathwayIdentifier,
};
use resources::{
    FhirCodeableConcept, FhirCoding, FhirIdentifier, FhirMeta, FhirPlanAction, FhirPlanDefinition,
    FhirRelatedArtifact, FhirUsageContext,
};

/// FHIR `identifier.system` for the pathway's top-level `pathway_code`.
const PATHWAY_CODE_SYSTEM: &str = "urn:mxi:carepathway:pathway-code";
/// FHIR `identifier.system` for the pathway's `provider_id`.
const PROVIDER_ID_SYSTEM: &str = "urn:mxi:carepathway:provider-id";
/// FHIR `identifier.system` for the pathway's `provider_name`.
const PROVIDER_NAME_SYSTEM: &str = "urn:mxi:carepathway:provider-name";
/// FHIR `identifier.system` for a stored alternate name.
const ALTERNATE_NAME_SYSTEM: &str = "urn:mxi:carepathway:alternate-name";
/// FHIR `identifier.system` for a BCP-47 language tag.
const LANGUAGE_SYSTEM: &str = "urn:mxi:carepathway:language";
/// `code.code` marking a target-condition `useContext`.
const USAGE_FOCUS: &str = "focus";
/// `code.code` marking a care-setting `useContext`.
const USAGE_SETTING: &str = "setting";
/// `code.code` marking a keyword-topic `useContext`.
const USAGE_TOPIC: &str = "topic";
/// The standard usage-context-type code system.
const USAGE_CONTEXT_TYPE_SYSTEM: &str =
    "http://terminology.hl7.org/CodeSystem/usage-context-type";
/// Family namespace for the non-standard `setting` / `topic` context codes.
const MXI_USAGE_CONTEXT_SYSTEM: &str = "urn:mxi:carepathway:usage-context";
/// Family namespace for care-setting tokens.
const CARE_SETTING_SYSTEM: &str = "urn:mxi:carepathway:care-setting";
/// The `PlanDefinition.type` classification (clinical protocol) system.
const PLAN_DEFINITION_TYPE_SYSTEM: &str =
    "http://terminology.hl7.org/CodeSystem/plan-definition-type";

/// Map an identifier `scheme` to the FHIR `identifier.system` URI. The
/// well-known registries use their canonical namespace; the rest use a
/// family `urn:mxi:carepathway:*` namespace. [`system_to_scheme`] is the
/// exact inverse, so a scheme round-trips through FHIR unchanged.
#[must_use]
pub fn scheme_to_system(scheme: &IdentifierScheme) -> String {
    match scheme {
        IdentifierScheme::Doi => "https://doi.org".to_string(),
        IdentifierScheme::Wikidata => "https://www.wikidata.org/entity".to_string(),
        IdentifierScheme::GuidelineId => "urn:mxi:carepathway:guideline".to_string(),
        IdentifierScheme::Uri => "urn:mxi:carepathway:uri".to_string(),
        IdentifierScheme::Uuid => "urn:mxi:carepathway:uuid".to_string(),
        IdentifierScheme::PathwayCode => "urn:mxi:carepathway:scheme-pathwaycode".to_string(),
        IdentifierScheme::LocalId => "urn:mxi:carepathway:localid".to_string(),
        IdentifierScheme::Custom(label) => format!("urn:mxi:carepathway:custom:{label}"),
    }
}

/// Map a FHIR `identifier.system` URI back to an identifier `scheme` — the
/// inverse of [`scheme_to_system`]. An unrecognised system is preserved as
/// `Custom(system)` so an inbound identifier from a foreign namespace is
/// never dropped.
#[must_use]
pub fn system_to_scheme(system: &str) -> IdentifierScheme {
    match system {
        "https://doi.org" => IdentifierScheme::Doi,
        "https://www.wikidata.org/entity" => IdentifierScheme::Wikidata,
        "urn:mxi:carepathway:guideline" => IdentifierScheme::GuidelineId,
        "urn:mxi:carepathway:uri" => IdentifierScheme::Uri,
        "urn:mxi:carepathway:uuid" => IdentifierScheme::Uuid,
        "urn:mxi:carepathway:scheme-pathwaycode" => IdentifierScheme::PathwayCode,
        "urn:mxi:carepathway:localid" => IdentifierScheme::LocalId,
        other => other.strip_prefix("urn:mxi:carepathway:custom:").map_or_else(
            || IdentifierScheme::Custom(other.to_string()),
            |label| IdentifierScheme::Custom(label.to_string()),
        ),
    }
}

/// Map a target-condition [`CodeSystem`] to its FHIR code-system URI.
/// [`uri_to_code_system`] is the exact inverse.
#[must_use]
pub fn code_system_to_uri(system: &CodeSystem) -> String {
    match system {
        CodeSystem::Icd10 => "http://hl7.org/fhir/sid/icd-10".to_string(),
        CodeSystem::Icd11 => "http://id.who.int/icd/release/11/mms".to_string(),
        CodeSystem::Snomed => "http://snomed.info/sct".to_string(),
        CodeSystem::Custom(label) => format!("urn:mxi:carepathway:condition:{label}"),
    }
}

/// Map a FHIR code-system URI back to a [`CodeSystem`] — the inverse of
/// [`code_system_to_uri`]. An unrecognised system is preserved as
/// `Custom(system)`.
#[must_use]
pub fn uri_to_code_system(uri: &str) -> CodeSystem {
    match uri {
        "http://hl7.org/fhir/sid/icd-10" => CodeSystem::Icd10,
        "http://id.who.int/icd/release/11/mms" => CodeSystem::Icd11,
        "http://snomed.info/sct" => CodeSystem::Snomed,
        other => other.strip_prefix("urn:mxi:carepathway:condition:").map_or_else(
            || CodeSystem::Custom(other.to_string()),
            |label| CodeSystem::Custom(label.to_string()),
        ),
    }
}

/// Map a [`CareSetting`] to its family token. [`token_to_care_setting`] is
/// the exact inverse, so a setting round-trips unchanged.
#[must_use]
pub fn care_setting_to_token(setting: &CareSetting) -> String {
    match setting {
        CareSetting::Inpatient => "inpatient".to_string(),
        CareSetting::Outpatient => "outpatient".to_string(),
        CareSetting::PrimaryCare => "primary-care".to_string(),
        CareSetting::EmergencyDepartment => "emergency-department".to_string(),
        CareSetting::Community => "community".to_string(),
        CareSetting::HomeCare => "home-care".to_string(),
        CareSetting::Rehabilitation => "rehabilitation".to_string(),
        CareSetting::MentalHealth => "mental-health".to_string(),
        CareSetting::Palliative => "palliative".to_string(),
        CareSetting::Custom(label) => format!("custom:{label}"),
    }
}

/// Map a family care-setting token back to a [`CareSetting`] — the inverse
/// of [`care_setting_to_token`]. An unrecognised token is preserved as
/// `Custom(token)`.
#[must_use]
pub fn token_to_care_setting(token: &str) -> CareSetting {
    match token {
        "inpatient" => CareSetting::Inpatient,
        "outpatient" => CareSetting::Outpatient,
        "primary-care" => CareSetting::PrimaryCare,
        "emergency-department" => CareSetting::EmergencyDepartment,
        "community" => CareSetting::Community,
        "home-care" => CareSetting::HomeCare,
        "rehabilitation" => CareSetting::Rehabilitation,
        "mental-health" => CareSetting::MentalHealth,
        "palliative" => CareSetting::Palliative,
        other => other.strip_prefix("custom:").map_or_else(
            || CareSetting::Custom(other.to_string()),
            |label| CareSetting::Custom(label.to_string()),
        ),
    }
}

/// The FHIR `status` string for a record's `active` flag: `active` for a
/// live row, `retired` for a soft-deleted one.
#[must_use]
pub fn status_for(active: bool) -> &'static str {
    if active { "active" } else { "retired" }
}

/// Build a single-coding `useContext` entry from a context-type code and a
/// value coding.
fn use_context(type_system: &str, type_code: &str, value: FhirCodeableConcept) -> FhirUsageContext {
    FhirUsageContext {
        code: FhirCoding {
            system: Some(type_system.to_string()),
            code: Some(type_code.to_string()),
            display: None,
        },
        value_codeable_concept: value,
    }
}

/// Build the `identifier` array: scalar-field family tokens
/// (`pathway_code` / `provider_id` / `provider_name` / `alternate_names` /
/// `in_language`) followed by the external `identifiers` (scheme → system).
fn plan_identifiers(pathway: &CarePathway) -> Vec<FhirIdentifier> {
    let mut identifier = Vec::new();
    let mut push_id = |system: &str, value: &str| {
        identifier.push(FhirIdentifier {
            system: Some(system.to_string()),
            value: Some(value.to_string()),
        });
    };
    if let Some(ref code) = pathway.pathway_code {
        push_id(PATHWAY_CODE_SYSTEM, code);
    }
    if let Some(ref provider) = pathway.provider_id {
        push_id(PROVIDER_ID_SYSTEM, provider);
    }
    if let Some(ref provider) = pathway.provider_name {
        push_id(PROVIDER_NAME_SYSTEM, provider);
    }
    for alt in &pathway.alternate_names {
        push_id(ALTERNATE_NAME_SYSTEM, alt);
    }
    for lang in &pathway.in_language {
        push_id(LANGUAGE_SYSTEM, lang);
    }
    for id in &pathway.identifiers {
        identifier.push(FhirIdentifier {
            system: Some(scheme_to_system(&id.scheme)),
            value: Some(id.value.clone()),
        });
    }
    identifier
}

/// Build the `useContext` array: target conditions (`focus`), care setting
/// (`setting`), and keyword topics (`topic`).
fn plan_use_contexts(pathway: &CarePathway) -> Vec<FhirUsageContext> {
    let mut out = Vec::new();
    for cc in &pathway.condition_codes {
        out.push(use_context(
            USAGE_CONTEXT_TYPE_SYSTEM,
            USAGE_FOCUS,
            FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: Some(code_system_to_uri(&cc.system)),
                    code: Some(cc.code.clone()),
                    display: None,
                }],
                text: None,
            },
        ));
    }
    if let Some(ref setting) = pathway.care_setting {
        out.push(use_context(
            MXI_USAGE_CONTEXT_SYSTEM,
            USAGE_SETTING,
            FhirCodeableConcept {
                coding: vec![FhirCoding {
                    system: Some(CARE_SETTING_SYSTEM.to_string()),
                    code: Some(care_setting_to_token(setting)),
                    display: None,
                }],
                text: None,
            },
        ));
    }
    for keyword in &pathway.keywords {
        out.push(use_context(
            MXI_USAGE_CONTEXT_SYSTEM,
            USAGE_TOPIC,
            FhirCodeableConcept {
                coding: Vec::new(),
                text: Some(keyword.clone()),
            },
        ));
    }
    out
}

/// Render a stored [`CarePathway`] as a FHIR R5 [`FhirPlanDefinition`].
///
/// `id` is the record's public `pid`; `status` comes from `active`
/// (`active` ⇒ live, `retired` ⇒ soft-deleted) and `last_updated` from the
/// row. `name` → `title`; `pathway_code` / `provider_id` / `provider_name`
/// / `alternate_names` / `in_language` → `identifier` (family systems);
/// `identifiers` → `identifier` (scheme → system); `condition_codes` /
/// `care_setting` / `keywords` → `useContext`; `interventions` → `action`;
/// `same_as` → `relatedArtifact`; `type` is a constant clinical-protocol
/// coding.
#[must_use]
pub fn to_fhir_plan_definition(
    pathway: &CarePathway,
    id: &str,
    active: bool,
    last_updated: Option<String>,
) -> FhirPlanDefinition {
    let mut fhir = FhirPlanDefinition::new();
    fhir.id = Some(id.to_string());
    fhir.status = Some(status_for(active).to_string());
    fhir.meta = last_updated.map(|lu| FhirMeta {
        version_id: None,
        last_updated: Some(lu),
    });
    fhir.title = Some(pathway.name.clone());
    fhir.kind = Some(FhirCodeableConcept {
        coding: vec![FhirCoding {
            system: Some(PLAN_DEFINITION_TYPE_SYSTEM.to_string()),
            code: Some("clinical-protocol".to_string()),
            display: Some("Clinical Protocol".to_string()),
        }],
        text: None,
    });

    fhir.identifier = plan_identifiers(pathway);
    fhir.use_context = plan_use_contexts(pathway);

    fhir.action = pathway
        .interventions
        .iter()
        .map(|i| FhirPlanAction {
            title: Some(i.clone()),
        })
        .collect();

    fhir.related_artifact = pathway
        .same_as
        .iter()
        .map(|url| FhirRelatedArtifact {
            artifact_type: Some("documentation".to_string()),
            url: Some(url.clone()),
        })
        .collect();

    fhir
}

/// Parse an inbound [`FhirPlanDefinition`] into a stored [`CarePathway`].
///
/// `title` is required (a `PlanDefinition` with no `title` is a `400`) and
/// becomes `name`. `identifier` entries are demultiplexed by `system` back
/// into `pathway_code` / `provider_id` / `provider_name` /
/// `alternate_names` / `in_language`, with the remainder becoming
/// `identifiers` (system → scheme). `useContext` entries are demultiplexed
/// by their `code.code`: `focus` → `condition_codes`, `setting` →
/// `care_setting`, `topic` → `keywords`. `action[].title` →
/// `interventions`; `relatedArtifact[].url` → `same_as`.
///
/// **Fidelity gaps** (no round-trip): the FHIR `status` and the constant
/// `type` classification are not carried back into the DTO (the DTO has no
/// status field — the record's `active` flag is the source of truth); an
/// inbound `useContext` `focus`/`setting` with an empty coding is skipped.
///
/// # Errors
///
/// Returns the missing-`title` diagnostic string when the resource has no
/// non-empty `title`.
pub fn from_fhir_plan_definition(fhir: &FhirPlanDefinition) -> Result<CarePathway, String> {
    let title = fhir
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "PlanDefinition.title is required".to_string())?;

    let mut pathway = CarePathway::new(title);

    for id in &fhir.identifier {
        let Some(value) = id.value.clone() else {
            continue;
        };
        match id.system.as_deref() {
            Some(PATHWAY_CODE_SYSTEM) => pathway.pathway_code = Some(value),
            Some(PROVIDER_ID_SYSTEM) => pathway.provider_id = Some(value),
            Some(PROVIDER_NAME_SYSTEM) => pathway.provider_name = Some(value),
            Some(ALTERNATE_NAME_SYSTEM) => pathway.alternate_names.push(value),
            Some(LANGUAGE_SYSTEM) => pathway.in_language.push(value),
            system => {
                let scheme = system
                    .map_or(IdentifierScheme::Custom(String::new()), system_to_scheme);
                pathway.identifiers.push(PathwayIdentifier { scheme, value });
            }
        }
    }

    for uc in &fhir.use_context {
        match uc.code.code.as_deref() {
            Some(USAGE_FOCUS) => {
                if let Some(coding) = uc.value_codeable_concept.coding.first()
                    && let Some(code) = coding.code.clone()
                {
                    let system = coding
                        .system
                        .as_deref()
                        .map_or(CodeSystem::Custom(String::new()), uri_to_code_system);
                    pathway.condition_codes.push(ConditionCode { system, code });
                }
            }
            Some(USAGE_SETTING) => {
                if let Some(coding) = uc.value_codeable_concept.coding.first()
                    && let Some(ref code) = coding.code
                {
                    pathway.care_setting = Some(token_to_care_setting(code));
                }
            }
            Some(USAGE_TOPIC) => {
                if let Some(text) = uc.value_codeable_concept.text.clone() {
                    pathway.keywords.push(text);
                }
            }
            _ => {}
        }
    }

    pathway.interventions = fhir
        .action
        .iter()
        .filter_map(|a| a.title.clone())
        .collect();

    pathway.same_as = fhir
        .related_artifact
        .iter()
        .filter_map(|a| a.url.clone())
        .collect();

    Ok(pathway)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every identifier scheme round-trips through the FHIR `system` URI
    /// unchanged (`scheme → system → scheme` is the identity).
    #[test]
    fn scheme_system_round_trips() {
        let schemes = [
            IdentifierScheme::Doi,
            IdentifierScheme::Wikidata,
            IdentifierScheme::GuidelineId,
            IdentifierScheme::Uri,
            IdentifierScheme::Uuid,
            IdentifierScheme::PathwayCode,
            IdentifierScheme::LocalId,
            IdentifierScheme::Custom("nice-internal".to_string()),
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

    /// Every condition code-system round-trips through its FHIR URI.
    #[test]
    fn code_system_round_trips() {
        let systems = [
            CodeSystem::Icd10,
            CodeSystem::Icd11,
            CodeSystem::Snomed,
            CodeSystem::Custom("local".to_string()),
        ];
        for system in systems {
            let uri = code_system_to_uri(&system);
            assert_eq!(uri_to_code_system(&uri), system, "round-trip {system:?}");
        }
    }

    /// Every care setting round-trips through its family token.
    #[test]
    fn care_setting_round_trips() {
        let settings = [
            CareSetting::Inpatient,
            CareSetting::Outpatient,
            CareSetting::PrimaryCare,
            CareSetting::EmergencyDepartment,
            CareSetting::Community,
            CareSetting::HomeCare,
            CareSetting::Rehabilitation,
            CareSetting::MentalHealth,
            CareSetting::Palliative,
            CareSetting::Custom("ward-b".to_string()),
        ];
        for setting in settings {
            let token = care_setting_to_token(&setting);
            assert_eq!(token_to_care_setting(&token), setting, "round-trip {setting:?}");
        }
    }

    /// The reversible fields survive `DTO → FHIR → DTO`.
    #[test]
    fn dto_fhir_round_trip_preserves_core_fields() {
        let pathway = CarePathway {
            name: "Acute Stroke Care Pathway".to_string(),
            alternate_names: vec!["Stroke Pathway".to_string(), "ASCP".to_string()],
            pathway_code: Some("STRK-001".to_string()),
            provider_id: Some("org-42".to_string()),
            provider_name: Some("St. Mary's".to_string()),
            care_setting: Some(CareSetting::EmergencyDepartment),
            condition_codes: vec![
                ConditionCode {
                    system: CodeSystem::Icd10,
                    code: "I63".to_string(),
                },
                ConditionCode {
                    system: CodeSystem::Snomed,
                    code: "422504002".to_string(),
                },
            ],
            interventions: vec!["Thrombolysis".to_string(), "CT scan".to_string()],
            keywords: vec!["stroke".to_string(), "acute".to_string()],
            identifiers: vec![PathwayIdentifier {
                scheme: IdentifierScheme::GuidelineId,
                value: "NG128".to_string(),
            }],
            same_as: vec!["https://www.nice.org.uk/guidance/ng128".to_string()],
            in_language: vec!["en".to_string()],
        };

        let fhir = to_fhir_plan_definition(&pathway, "pid-123", true, None);
        assert_eq!(fhir.resource_type, "PlanDefinition");
        assert_eq!(fhir.id.as_deref(), Some("pid-123"));
        assert_eq!(fhir.status.as_deref(), Some("active"));

        let back = from_fhir_plan_definition(&fhir).expect("valid resource");
        assert_eq!(back.name, pathway.name);
        assert_eq!(back.alternate_names, pathway.alternate_names);
        assert_eq!(back.pathway_code, pathway.pathway_code);
        assert_eq!(back.provider_id, pathway.provider_id);
        assert_eq!(back.provider_name, pathway.provider_name);
        assert_eq!(back.care_setting, pathway.care_setting);
        assert_eq!(back.condition_codes.len(), 2);
        assert_eq!(back.condition_codes[0].system, CodeSystem::Icd10);
        assert_eq!(back.condition_codes[0].code, "I63");
        assert_eq!(back.condition_codes[1].system, CodeSystem::Snomed);
        assert_eq!(back.interventions, pathway.interventions);
        assert_eq!(back.keywords, pathway.keywords);
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].scheme, IdentifierScheme::GuidelineId);
        assert_eq!(back.identifiers[0].value, "NG128");
        assert_eq!(back.same_as, pathway.same_as);
        assert_eq!(back.in_language, pathway.in_language);
    }

    /// A soft-deleted record renders `status: "retired"`.
    #[test]
    fn soft_deleted_renders_retired() {
        let pathway = CarePathway::new("Retired Pathway");
        let fhir = to_fhir_plan_definition(&pathway, "pid-1", false, None);
        assert_eq!(fhir.status.as_deref(), Some("retired"));
    }

    /// A resource with no `title` is rejected (maps to a `400`).
    #[test]
    fn missing_title_is_rejected() {
        let fhir = FhirPlanDefinition::new();
        assert!(from_fhir_plan_definition(&fhir).is_err());
    }
}
