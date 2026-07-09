//! HL7 FHIR R5 interop for the Course entity as a **non-standard** `Basic`.
//!
//! **There is no standard FHIR R5 resource for an educational course.** This
//! module implements a deliberately **non-standard, best-effort** mapping
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md) §3): the stored
//! [`crate::models::Course`] is wrapped as a FHIR `Basic` resource whose
//! `code` is the local coding `{urn:mxi:resource | course}` and whose
//! course-specific data rides in `identifier` tokens and documented
//! `urn:mxi:course:*` extensions. It is a courtesy shape for FHIR-shaped
//! tooling, **not** interoperable with a standards-only client.
//!
//! [`to_fhir_basic`] renders a stored course for outbound responses;
//! [`from_fhir_basic`] parses an inbound resource. Resource shapes live in
//! [`resources`], search-parameter parsing in [`search`], and the mounted
//! Axum endpoints in [`crate::api::rest::fhir`].
//!
//! Conversions are **lossy where the model has no FHIR home** — documented
//! inline and in [`from_fhir_basic`]'s doc — never silent.

/// FHIR resource + envelope wire types (`Basic`, `OperationOutcome`, `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use crate::models::{Course, CourseIdentifier, EducationalLevel, IdentifierType};
use resources::{FhirBasic, FhirExtension, FhirIdentifier, FhirMeta};

/// `identifier.system` URI for the course's scalar `course_code`
/// (schema.org/courseCode). Kept distinct from the `IdentifierType::CourseCode`
/// system so the two round-trip to different homes.
pub const SYS_COURSE_CODE: &str = "https://schema.org/courseCode";

/// Extension URL carrying the course `name` (required on inbound).
pub const EXT_NAME: &str = "urn:mxi:course:name";
/// Extension URL carrying the course `educational_level`.
pub const EXT_LEVEL: &str = "urn:mxi:course:educational-level";
/// Extension URL carrying one course `keyword` (repeatable).
pub const EXT_KEYWORD: &str = "urn:mxi:course:keyword";
/// Extension URL carrying one `teaches` competency (repeatable).
pub const EXT_TEACHES: &str = "urn:mxi:course:teaches";

/// Map a [`CourseIdentifier`] scheme to its FHIR `identifier.system` URI.
/// Well-known registries use their canonical namespace; the rest use a
/// family `urn:mxi:course:*` namespace. [`system_to_scheme`] is the exact
/// inverse, so a scheme round-trips through FHIR unchanged.
#[must_use]
pub fn scheme_to_system(scheme: &IdentifierType) -> String {
    match scheme {
        IdentifierType::Doi => "https://doi.org".to_string(),
        IdentifierType::Wikidata => "https://www.wikidata.org/entity".to_string(),
        IdentifierType::Ror => "https://ror.org".to_string(),
        IdentifierType::Oer => "urn:mxi:course:oer".to_string(),
        IdentifierType::Lom => "urn:mxi:course:lom".to_string(),
        IdentifierType::Isced => "urn:mxi:course:isced".to_string(),
        IdentifierType::Uri => "urn:mxi:course:uri".to_string(),
        IdentifierType::Uuid => "urn:mxi:course:uuid".to_string(),
        IdentifierType::LmsCourseId => "urn:mxi:course:lms".to_string(),
        IdentifierType::CourseCode => "urn:mxi:course:course-code".to_string(),
        IdentifierType::PlatformSlug => "urn:mxi:course:platform-slug".to_string(),
        IdentifierType::Custom(label) => format!("urn:mxi:course:custom:{label}"),
    }
}

/// Map a FHIR `identifier.system` URI back to a [`CourseIdentifier`] scheme —
/// the inverse of [`scheme_to_system`]. An unrecognised system is preserved
/// as `Custom(system)` so an inbound identifier from a foreign namespace is
/// never dropped.
#[must_use]
pub fn system_to_scheme(system: &str) -> IdentifierType {
    match system {
        "https://doi.org" => IdentifierType::Doi,
        "https://www.wikidata.org/entity" => IdentifierType::Wikidata,
        "https://ror.org" => IdentifierType::Ror,
        "urn:mxi:course:oer" => IdentifierType::Oer,
        "urn:mxi:course:lom" => IdentifierType::Lom,
        "urn:mxi:course:isced" => IdentifierType::Isced,
        "urn:mxi:course:uri" => IdentifierType::Uri,
        "urn:mxi:course:uuid" => IdentifierType::Uuid,
        "urn:mxi:course:lms" => IdentifierType::LmsCourseId,
        "urn:mxi:course:course-code" => IdentifierType::CourseCode,
        "urn:mxi:course:platform-slug" => IdentifierType::PlatformSlug,
        other => other.strip_prefix("urn:mxi:course:custom:").map_or_else(
            || IdentifierType::Custom(other.to_string()),
            |label| IdentifierType::Custom(label.to_string()),
        ),
    }
}

/// Render an [`EducationalLevel`] as the plain string carried in the level
/// extension (the serde tag for known variants; the label for `Custom`).
fn level_to_string(level: &EducationalLevel) -> String {
    match level {
        EducationalLevel::Custom(label) => label.clone(),
        other => serde_json::to_value(other)
            .ok()
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_default(),
    }
}

/// Parse an [`EducationalLevel`] from the level extension string — a known
/// serde tag maps to its variant, anything else to `Custom(string)`.
fn level_from_string(s: &str) -> EducationalLevel {
    serde_json::from_value::<EducationalLevel>(serde_json::Value::String(s.to_string()))
        .unwrap_or_else(|_| EducationalLevel::Custom(s.to_string()))
}

/// Render a stored [`Course`] as the **non-standard** FHIR [`FhirBasic`].
///
/// `id`/`meta.lastUpdated` come from the record; `meta.profile` advertises
/// the non-standard course profile. `identifier` carries `course_code` (as a
/// `schema.org/courseCode` token) then each [`CourseIdentifier`]. The name,
/// `educational_level`, `keywords`, and `teaches` ride in `urn:mxi:course:*`
/// extensions.
///
/// **Fidelity gaps** (no `Basic` element, not emitted): `description`,
/// `about`, `url`, `same_as`, `assesses`, `competency_required`,
/// `number_of_credits`, `status`, `active`, `provider_id`, credentials,
/// syllabus sections, and the `instances` sub-resource.
#[must_use]
pub fn to_fhir_basic(course: &Course) -> FhirBasic {
    let mut basic = FhirBasic::new();
    basic.id = Some(course.id.to_string());
    basic.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(course.updated_at.to_rfc3339()),
        profile: vec![resources::PROFILE_URL.to_string()],
    });

    let mut identifier = Vec::new();
    if let Some(ref code) = course.course_code {
        identifier.push(FhirIdentifier {
            system: Some(SYS_COURSE_CODE.to_string()),
            value: Some(code.clone()),
        });
    }
    identifier.extend(course.identifiers.iter().map(|id| FhirIdentifier {
        system: Some(scheme_to_system(&id.property_id)),
        value: Some(id.value.clone()),
    }));
    basic.identifier = identifier;

    let mut extension = vec![FhirExtension {
        url: EXT_NAME.to_string(),
        value_string: Some(course.name.clone()),
    }];
    if let Some(ref level) = course.educational_level {
        extension.push(FhirExtension {
            url: EXT_LEVEL.to_string(),
            value_string: Some(level_to_string(level)),
        });
    }
    extension.extend(course.keywords.iter().map(|k| FhirExtension {
        url: EXT_KEYWORD.to_string(),
        value_string: Some(k.clone()),
    }));
    extension.extend(course.teaches.iter().map(|t| FhirExtension {
        url: EXT_TEACHES.to_string(),
        value_string: Some(t.clone()),
    }));
    basic.extension = extension;

    basic
}

/// Parse an inbound [`FhirBasic`] into a stored [`Course`].
///
/// The course **name** (extension `urn:mxi:course:name`) is required — a
/// resource without one is a `400`. `educational_level`, `keywords`, and
/// `teaches` come from their extensions; `identifier` tokens become
/// `course_code` (for the `schema.org/courseCode` system) or
/// [`CourseIdentifier`]s (system → scheme).
///
/// **Fidelity gaps**: only the fields [`to_fhir_basic`] emits are recovered;
/// every other `Course` field defaults (see [`to_fhir_basic`]'s gap list).
/// The resource `id` is **not** applied here — the create handler mints a
/// fresh `pid` and the update handler sets it from the path.
///
/// # Errors
///
/// Returns the missing-name diagnostic string when the resource carries no
/// non-empty `urn:mxi:course:name` extension.
pub fn from_fhir_basic(fhir: &FhirBasic) -> Result<Course, String> {
    let name = fhir
        .extension
        .iter()
        .find(|e| e.url == EXT_NAME)
        .and_then(|e| e.value_string.as_deref())
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| "Course name (extension urn:mxi:course:name) is required".to_string())?;

    let mut course = Course::new(name);

    if let Some(level) = fhir
        .extension
        .iter()
        .find(|e| e.url == EXT_LEVEL)
        .and_then(|e| e.value_string.as_deref())
    {
        course.educational_level = Some(level_from_string(level));
    }
    course.keywords = fhir
        .extension
        .iter()
        .filter(|e| e.url == EXT_KEYWORD)
        .filter_map(|e| e.value_string.clone())
        .collect();
    course.teaches = fhir
        .extension
        .iter()
        .filter(|e| e.url == EXT_TEACHES)
        .filter_map(|e| e.value_string.clone())
        .collect();

    for id in &fhir.identifier {
        let Some(value) = id.value.clone() else {
            continue;
        };
        let system = id.system.as_deref();
        if system == Some(SYS_COURSE_CODE) {
            course.course_code = Some(value);
        } else {
            let scheme = system.map_or(IdentifierType::Custom(String::new()), system_to_scheme);
            course.identifiers.push(CourseIdentifier {
                property_id: scheme,
                value,
                name: None,
                url: None,
            });
        }
    }

    Ok(course)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every identifier scheme round-trips through the FHIR `system` URI
    /// unchanged (`scheme → system → scheme` is the identity).
    #[test]
    fn scheme_system_round_trips() {
        let schemes = [
            IdentifierType::LmsCourseId,
            IdentifierType::CourseCode,
            IdentifierType::PlatformSlug,
            IdentifierType::Oer,
            IdentifierType::Doi,
            IdentifierType::Lom,
            IdentifierType::Wikidata,
            IdentifierType::Isced,
            IdentifierType::Ror,
            IdentifierType::Uri,
            IdentifierType::Uuid,
            IdentifierType::Custom("moodle-internal".to_string()),
        ];
        for scheme in schemes {
            let system = scheme_to_system(&scheme);
            assert_eq!(system_to_scheme(&system), scheme, "round-trip {scheme:?}");
        }
    }

    /// An unknown inbound `system` is preserved as `Custom(system)`, never
    /// dropped.
    #[test]
    fn unknown_system_becomes_custom() {
        assert_eq!(
            system_to_scheme("https://example.org/foo"),
            IdentifierType::Custom("https://example.org/foo".to_string())
        );
    }

    /// The losslessly-reversible fields survive `DTO → Basic → DTO`, and the
    /// output resource is a `Basic` carrying the `{urn:mxi:resource|course}`
    /// coding.
    #[test]
    fn dto_basic_round_trip_preserves_core_fields() {
        let mut course = Course::new("Introduction to Computer Science");
        course.course_code = Some("CS101".to_string());
        course.educational_level = Some(EducationalLevel::Undergraduate);
        course.keywords = vec!["programming".to_string(), "algorithms".to_string()];
        course.teaches = vec!["recursion".to_string()];
        course.identifiers = vec![CourseIdentifier {
            property_id: IdentifierType::Doi,
            value: "10.1234/abc".to_string(),
            name: None,
            url: None,
        }];

        let basic = to_fhir_basic(&course);
        assert_eq!(basic.resource_type, resources::RESOURCE_TYPE);
        assert_eq!(basic.id.as_deref(), Some(course.id.to_string().as_str()));
        assert_eq!(
            basic.code.coding[0].code.as_deref(),
            Some(resources::RESOURCE_CODE)
        );

        let back = from_fhir_basic(&basic).expect("valid resource");
        assert_eq!(back.name, course.name);
        assert_eq!(back.course_code, course.course_code);
        assert_eq!(back.educational_level, course.educational_level);
        assert_eq!(back.keywords, course.keywords);
        assert_eq!(back.teaches, course.teaches);
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].property_id, IdentifierType::Doi);
        assert_eq!(back.identifiers[0].value, course.identifiers[0].value);
    }

    /// A resource with no name extension is rejected (maps to a `400`).
    #[test]
    fn missing_name_is_rejected() {
        let basic = FhirBasic::new();
        assert!(from_fhir_basic(&basic).is_err());
    }

    /// A `Custom` educational level round-trips via its label.
    #[test]
    fn custom_level_round_trips() {
        let mut course = Course::new("K-12 Civics");
        course.educational_level = Some(EducationalLevel::Custom("K-12".to_string()));
        let basic = to_fhir_basic(&course);
        let back = from_fhir_basic(&basic).expect("valid resource");
        assert_eq!(
            back.educational_level,
            Some(EducationalLevel::Custom("K-12".to_string()))
        );
    }
}
