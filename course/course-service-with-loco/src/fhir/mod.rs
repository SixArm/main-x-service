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

use crate::models::{Course, CourseIdentifier, CourseStatus, EducationalLevel, IdentifierType};
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
/// Extension URL carrying the course `description`.
pub const EXT_DESCRIPTION: &str = "urn:mxi:course:description";
/// Extension URL carrying one `about` subject (repeatable).
pub const EXT_ABOUT: &str = "urn:mxi:course:about";
/// Extension URL carrying the course's canonical `url`.
pub const EXT_URL: &str = "urn:mxi:course:url";
/// Extension URL carrying one `same_as` reference URL (repeatable).
pub const EXT_SAME_AS: &str = "urn:mxi:course:same-as";
/// Extension URL carrying one `assesses` competency (repeatable).
pub const EXT_ASSESSES: &str = "urn:mxi:course:assesses";
/// Extension URL carrying one `competency_required` entry (repeatable).
pub const EXT_COMPETENCY_REQUIRED: &str = "urn:mxi:course:competency-required";
/// Extension URL carrying `number_of_credits` (`valueUnsignedInt`).
pub const EXT_NUMBER_OF_CREDITS: &str = "urn:mxi:course:number-of-credits";
/// Extension URL carrying the lifecycle `status` (its lowercase serde tag).
pub const EXT_STATUS: &str = "urn:mxi:course:status";
/// Extension URL carrying the `active` flag (`valueBoolean`).
pub const EXT_ACTIVE: &str = "urn:mxi:course:active";
/// Extension URL carrying the `provider_id` UUID.
pub const EXT_PROVIDER_ID: &str = "urn:mxi:course:provider-id";

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

/// Render a [`CourseStatus`] as its lowercase serde tag (`"published"`, …).
fn status_to_string(status: CourseStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_default()
}

/// Parse a [`CourseStatus`] from its serde tag. Unlike the level, the status
/// enum is closed, so an unknown value is an error rather than a fallback.
fn status_from_string(s: &str) -> Result<CourseStatus, String> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|_| {
        format!(
            "Invalid course status {s:?} (extension {EXT_STATUS}): expected one of \
             draft, published, archived, retired"
        )
    })
}

/// The `valueString`s of every extension with `url`, in document order.
fn ext_strings(fhir: &FhirBasic, url: &str) -> Vec<String> {
    fhir.extension
        .iter()
        .filter(|e| e.url == url)
        .filter_map(|e| e.value_string.clone())
        .collect()
}

/// The first extension with `url`, if any.
fn ext_first<'a>(fhir: &'a FhirBasic, url: &str) -> Option<&'a FhirExtension> {
    fhir.extension.iter().find(|e| e.url == url)
}

/// The `valueString` of the first extension with `url`, if any.
fn ext_string(fhir: &FhirBasic, url: &str) -> Option<String> {
    ext_first(fhir, url).and_then(|e| e.value_string.clone())
}

/// Render a stored [`Course`] as the **non-standard** FHIR [`FhirBasic`].
///
/// `id`/`meta.lastUpdated` come from the record; `meta.profile` advertises
/// the non-standard course profile. `identifier` carries `course_code` (as a
/// `schema.org/courseCode` token) then each [`CourseIdentifier`]. The name,
/// `educational_level`, `description`, `url`, `number_of_credits`
/// (`valueUnsignedInt`), `status`, `active` (`valueBoolean`), `provider_id`,
/// and the repeatable `keywords`, `teaches`, `about`, `same_as`, `assesses`,
/// and `competency_required` ride in `urn:mxi:course:*` extensions (one
/// extension per value for the repeatable ones). `status` and `active` are
/// always emitted.
///
/// **Fidelity gaps** (structured, no flat extension, not emitted):
/// credentials, syllabus sections, and the `instances` sub-resource.
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

    let mut extension = vec![FhirExtension::string(EXT_NAME, course.name.clone())];
    if let Some(ref level) = course.educational_level {
        extension.push(FhirExtension::string(EXT_LEVEL, level_to_string(level)));
    }
    let repeated = [
        (EXT_KEYWORD, &course.keywords),
        (EXT_TEACHES, &course.teaches),
        (EXT_ABOUT, &course.about),
        (EXT_SAME_AS, &course.same_as),
        (EXT_ASSESSES, &course.assesses),
        (EXT_COMPETENCY_REQUIRED, &course.competency_required),
    ];
    for (url, values) in repeated {
        extension.extend(values.iter().map(|v| FhirExtension::string(url, v.clone())));
    }
    if let Some(ref description) = course.description {
        extension.push(FhirExtension::string(EXT_DESCRIPTION, description.clone()));
    }
    if let Some(ref url) = course.url {
        extension.push(FhirExtension::string(EXT_URL, url.clone()));
    }
    if let Some(credits) = course.number_of_credits {
        extension.push(FhirExtension::unsigned_int(EXT_NUMBER_OF_CREDITS, credits));
    }
    // `status` and `active` are always emitted: both have non-`None`
    // defaults, so omitting them would make "absent" ambiguous on the way
    // back in.
    extension.push(FhirExtension::string(
        EXT_STATUS,
        status_to_string(course.status),
    ));
    extension.push(FhirExtension::boolean(EXT_ACTIVE, course.active));
    if let Some(provider_id) = course.provider_id {
        extension.push(FhirExtension::string(
            EXT_PROVIDER_ID,
            provider_id.to_string(),
        ));
    }
    basic.extension = extension;

    // Fidelity gaps (agents/share/fhir.md §2: every drop of fidelity is a
    // documented, TODO-marked gap, never silent). These are the structured
    // `Course` fields with no flat `urn:mxi:course:*` extension yet — each
    // needs a nested (complex) extension or a contained resource, so each is
    // dropped on the way out. Add one as each becomes a real requirement.
    // TODO(fhir): course.credentials is not emitted.
    // TODO(fhir): course's syllabus sections are not emitted.
    // TODO(fhir): the `instances` sub-resource is not emitted.

    basic
}

/// Parse an inbound [`FhirBasic`] into a stored [`Course`].
///
/// The course **name** (extension `urn:mxi:course:name`) is required — a
/// resource without one is a `400`. Every other extension [`to_fhir_basic`]
/// emits is read back into its field; `identifier` tokens become
/// `course_code` (for the `schema.org/courseCode` system) or
/// [`CourseIdentifier`]s (system → scheme). An absent `status` / `active`
/// extension leaves the [`Course::new`] default (`published` / `true`), so a
/// minimal hand-written resource still creates a live course.
///
/// **Fidelity gaps**: only the fields [`to_fhir_basic`] emits are recovered;
/// every other `Course` field defaults (see [`to_fhir_basic`]'s gap list).
/// The resource `id` is **not** applied here — the create handler mints a
/// fresh `pid` and the update handler sets it from the path.
///
/// # Errors
///
/// Returns a diagnostic string (mapped to a `400` by the handler) when the
/// resource carries no non-empty `urn:mxi:course:name` extension, or when a
/// typed extension is present but malformed: a `status` that is not a known
/// lifecycle tag, a `provider-id` that is not a UUID, or a `number-of-credits`
/// / `active` extension missing its `valueUnsignedInt` / `valueBoolean`.
/// Malformed values are rejected rather than dropped, so a client never gets
/// a `201` for data the server silently discarded.
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
    course.keywords = ext_strings(fhir, EXT_KEYWORD);
    course.teaches = ext_strings(fhir, EXT_TEACHES);
    course.about = ext_strings(fhir, EXT_ABOUT);
    course.same_as = ext_strings(fhir, EXT_SAME_AS);
    course.assesses = ext_strings(fhir, EXT_ASSESSES);
    course.competency_required = ext_strings(fhir, EXT_COMPETENCY_REQUIRED);
    course.description = ext_string(fhir, EXT_DESCRIPTION);
    course.url = ext_string(fhir, EXT_URL);

    if let Some(ext) = ext_first(fhir, EXT_NUMBER_OF_CREDITS) {
        let credits = ext.value_unsigned_int.ok_or_else(|| {
            format!("Extension {EXT_NUMBER_OF_CREDITS} requires a valueUnsignedInt")
        })?;
        course.number_of_credits = Some(credits);
    }
    if let Some(ext) = ext_first(fhir, EXT_STATUS) {
        let status = ext
            .value_string
            .as_deref()
            .ok_or_else(|| format!("Extension {EXT_STATUS} requires a valueString"))?;
        course.status = status_from_string(status)?;
    }
    if let Some(ext) = ext_first(fhir, EXT_ACTIVE) {
        course.active = ext
            .value_boolean
            .ok_or_else(|| format!("Extension {EXT_ACTIVE} requires a valueBoolean"))?;
    }
    if let Some(ext) = ext_first(fhir, EXT_PROVIDER_ID) {
        let raw = ext.value_string.as_deref().unwrap_or_default();
        let provider_id = uuid::Uuid::parse_str(raw).map_err(|_| {
            format!("Invalid provider id {raw:?} (extension {EXT_PROVIDER_ID}): expected a UUID")
        })?;
        course.provider_id = Some(provider_id);
    }

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

    // Fidelity gaps (mirrors to_fhir_basic's TODO list — nothing recovers
    // these fields here because to_fhir_basic never emitted them, so they
    // default on every round-trip through FHIR):
    // TODO(fhir): course.credentials is not recovered.
    // TODO(fhir): course's syllabus sections are not recovered.
    // TODO(fhir): the `instances` sub-resource is not recovered.

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

    /// Every field that rides in a flat extension survives
    /// `DTO → Basic → DTO` — including the ten that were fidelity gaps
    /// before T-31 (`description` … `provider_id`), with non-default `status`
    /// and `active` so a silent fall-back to the default would be caught.
    #[test]
    fn extension_fields_round_trip() {
        let mut course = Course::new("Data Structures");
        course.description = Some("Lists, trees, and graphs.".to_string());
        course.about = vec!["computer science".to_string(), "algorithms".to_string()];
        course.url = Some("https://example.edu/courses/cs201".to_string());
        course.same_as = vec!["https://www.wikidata.org/wiki/Q175263".to_string()];
        course.assesses = vec!["tree traversal".to_string()];
        course.competency_required = vec!["CS101".to_string(), "discrete math".to_string()];
        course.number_of_credits = Some(4);
        course.status = CourseStatus::Retired;
        course.active = false;
        course.provider_id = Some(uuid::Uuid::new_v4());

        let back = from_fhir_basic(&to_fhir_basic(&course)).expect("valid resource");
        assert_eq!(back.description, course.description);
        assert_eq!(back.about, course.about);
        assert_eq!(back.url, course.url);
        assert_eq!(back.same_as, course.same_as);
        assert_eq!(back.assesses, course.assesses);
        assert_eq!(back.competency_required, course.competency_required);
        assert_eq!(back.number_of_credits, course.number_of_credits);
        assert_eq!(back.status, course.status);
        assert_eq!(back.active, course.active);
        assert_eq!(back.provider_id, course.provider_id);
    }

    /// Typed values serialize under their FHIR `value[x]` names, and only
    /// the one that is set.
    #[test]
    fn typed_extensions_serialize_as_fhir_value_x() {
        let mut course = Course::new("Statistics");
        course.number_of_credits = Some(3);
        let json = serde_json::to_value(to_fhir_basic(&course)).expect("serializes");
        let ext = json["extension"].as_array().expect("extension array");
        let find = |url: &str| {
            ext.iter()
                .find(|e| e["url"] == url)
                .unwrap_or_else(|| panic!("{url} emitted"))
                .clone()
        };
        let credits = find(EXT_NUMBER_OF_CREDITS);
        assert_eq!(credits["valueUnsignedInt"], 3);
        assert!(credits.get("valueString").is_none());
        assert_eq!(find(EXT_ACTIVE)["valueBoolean"], true);
        assert_eq!(find(EXT_STATUS)["valueString"], "published");
    }

    /// A minimal resource (name only) keeps the `Course::new` defaults for
    /// the always-emitted `status` / `active`, and leaves the rest empty.
    #[test]
    fn absent_extensions_keep_defaults() {
        let mut basic = FhirBasic::new();
        basic.extension = vec![FhirExtension::string(EXT_NAME, "Ethics")];
        let course = from_fhir_basic(&basic).expect("valid resource");
        assert_eq!(course.status, CourseStatus::Published);
        assert!(course.active);
        assert_eq!(course.description, None);
        assert_eq!(course.number_of_credits, None);
        assert_eq!(course.provider_id, None);
        assert!(course.about.is_empty());
    }

    /// Each malformed typed extension is a `400`-mapped error, never a
    /// silent drop.
    #[test]
    fn malformed_typed_extensions_are_rejected() {
        let bad = [
            FhirExtension::string(EXT_STATUS, "pending"),
            FhirExtension::string(EXT_PROVIDER_ID, "not-a-uuid"),
            FhirExtension::string(EXT_NUMBER_OF_CREDITS, "3"),
            FhirExtension::string(EXT_ACTIVE, "false"),
            FhirExtension::boolean(EXT_STATUS, true),
        ];
        for ext in bad {
            let mut basic = FhirBasic::new();
            basic.extension = vec![FhirExtension::string(EXT_NAME, "Ethics"), ext.clone()];
            let err = from_fhir_basic(&basic).expect_err("malformed extension rejected");
            assert!(err.contains(&ext.url), "{err:?} names {}", ext.url);
        }
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
