//! Service `Course` → matcher `Course` adapter.
//!
//! The matcher carries a slim subset of `schema.org/Course` — only the
//! fields the algorithm consumes. We project the richer service domain
//! model down to that shape. The provider FK is stringified so the
//! matcher's `same provider_id + course_code` short-circuit fires
//! when both service records reference the same provider row.
//!
//! Routing rules (pinned by `tests/duplicate_detection.rs` in T-11):
//! - `service::Course.name` → `matcher::Course.name`
//! - `service::Course.alternate_names` → `matcher::Course.alternate_names`
//! - `service::Course.course_code` → `matcher::Course.course_code`
//! - `service::Course.provider_id` (Uuid) → `matcher::Course.provider_id` (String)
//! - `service::Course.educational_level` → `matcher::Course.educational_level`
//! - `service::Course.learning_resource_type` → `matcher::Course.learning_resource_type`
//! - `service::Course.keywords` → `matcher::Course.keywords`
//! - `service::Course.teaches` → `matcher::Course.teaches`
//! - `service::Course.identifiers` → `matcher::Course.identifiers` (scheme + value)
//! - `service::Course.same_as` → `matcher::Course.same_as`
//! - `service::Course.in_language` → `matcher::Course.in_language`

use course_matcher as cm;

use crate::models::{
    Course as ServiceCourse, CourseIdentifier as ServiceIdent,
    EducationalLevel as ServiceLevel, IdentifierType, LearningResourceType as ServiceLRT,
};

/// Project the rich service [`Course`](ServiceCourse) down to the slim
/// `course_matcher::Course` the algorithm consumes. The provider FK
/// (`Uuid`) is stringified so the matcher's
/// `same provider_id + course_code` short-circuit can fire.
pub fn to_matcher_course(c: &ServiceCourse) -> cm::Course {
    cm::Course {
        name: c.name.clone(),
        alternate_names: c.alternate_names.clone(),
        course_code: c.course_code.clone(),
        provider_id: c.provider_id.map(|u| u.to_string()),
        provider_name: None,
        educational_level: c.educational_level.as_ref().map(level_to_matcher),
        learning_resource_type: c.learning_resource_type.as_ref().map(lrt_to_matcher),
        keywords: c.keywords.clone(),
        teaches: c.teaches.clone(),
        identifiers: c.identifiers.iter().map(ident_to_matcher).collect(),
        same_as: c.same_as.clone(),
        in_language: c.in_language.clone(),
    }
}

/// Map a service [`CourseIdentifier`](ServiceIdent) to the matcher's
/// `CourseIdentifier` (scheme + value pair).
fn ident_to_matcher(i: &ServiceIdent) -> cm::CourseIdentifier {
    cm::CourseIdentifier {
        scheme: scheme_to_matcher(&i.property_id),
        value: i.value.clone(),
    }
}

/// Map the service [`IdentifierType`] enum one-to-one to the matcher's
/// `IdentifierScheme`.
fn scheme_to_matcher(t: &IdentifierType) -> cm::IdentifierScheme {
    match t {
        IdentifierType::LmsCourseId => cm::IdentifierScheme::LmsCourseId,
        IdentifierType::CourseCode => cm::IdentifierScheme::CourseCode,
        IdentifierType::PlatformSlug => cm::IdentifierScheme::PlatformSlug,
        IdentifierType::Oer => cm::IdentifierScheme::Oer,
        IdentifierType::Doi => cm::IdentifierScheme::Doi,
        IdentifierType::Lom => cm::IdentifierScheme::Lom,
        IdentifierType::Wikidata => cm::IdentifierScheme::Wikidata,
        IdentifierType::Isced => cm::IdentifierScheme::Isced,
        IdentifierType::Ror => cm::IdentifierScheme::Ror,
        IdentifierType::Uri => cm::IdentifierScheme::Uri,
        IdentifierType::Uuid => cm::IdentifierScheme::Uuid,
        IdentifierType::Custom(s) => cm::IdentifierScheme::Custom(s.clone()),
    }
}

/// Map the service [`EducationalLevel`](ServiceLevel) enum one-to-one to
/// the matcher's `EducationalLevel`.
fn level_to_matcher(l: &ServiceLevel) -> cm::EducationalLevel {
    match l {
        ServiceLevel::Beginner => cm::EducationalLevel::Beginner,
        ServiceLevel::Intermediate => cm::EducationalLevel::Intermediate,
        ServiceLevel::Advanced => cm::EducationalLevel::Advanced,
        ServiceLevel::Expert => cm::EducationalLevel::Expert,
        ServiceLevel::PrimaryEducation => cm::EducationalLevel::PrimaryEducation,
        ServiceLevel::SecondaryEducation => cm::EducationalLevel::SecondaryEducation,
        ServiceLevel::HigherEducation => cm::EducationalLevel::HigherEducation,
        ServiceLevel::Undergraduate => cm::EducationalLevel::Undergraduate,
        ServiceLevel::Graduate => cm::EducationalLevel::Graduate,
        ServiceLevel::Postgraduate => cm::EducationalLevel::Postgraduate,
        ServiceLevel::Vocational => cm::EducationalLevel::Vocational,
        ServiceLevel::ProfessionalDevelopment => cm::EducationalLevel::ProfessionalDevelopment,
        ServiceLevel::Custom(s) => cm::EducationalLevel::Custom(s.clone()),
    }
}

/// Map the service [`LearningResourceType`](ServiceLRT) enum one-to-one
/// to the matcher's `LearningResourceType`.
fn lrt_to_matcher(l: &ServiceLRT) -> cm::LearningResourceType {
    match l {
        ServiceLRT::Lecture => cm::LearningResourceType::Lecture,
        ServiceLRT::Tutorial => cm::LearningResourceType::Tutorial,
        ServiceLRT::Workshop => cm::LearningResourceType::Workshop,
        ServiceLRT::Assignment => cm::LearningResourceType::Assignment,
        ServiceLRT::Reading => cm::LearningResourceType::Reading,
        ServiceLRT::Video => cm::LearningResourceType::Video,
        ServiceLRT::Audio => cm::LearningResourceType::Audio,
        ServiceLRT::Exam => cm::LearningResourceType::Exam,
        ServiceLRT::Simulation => cm::LearningResourceType::Simulation,
        ServiceLRT::Project => cm::LearningResourceType::Project,
        ServiceLRT::Discussion => cm::LearningResourceType::Discussion,
        ServiceLRT::Custom(s) => cm::LearningResourceType::Custom(s.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// A `Uuid` provider FK is stringified into the matcher's `provider_id`.
    #[test]
    fn provider_id_uuid_routes_to_matcher_string() {
        let mut c = ServiceCourse::new("Linear Algebra");
        let pid = Uuid::new_v4();
        c.provider_id = Some(pid);
        let m = to_matcher_course(&c);
        assert_eq!(m.provider_id.as_deref(), Some(pid.to_string().as_str()));
    }

    /// Identifier schemes (including `Custom`) map across one-to-one.
    #[test]
    fn identifier_scheme_routes_one_to_one() {
        let mut c = ServiceCourse::new("CS101");
        c.identifiers = vec![
            ServiceIdent {
                property_id: IdentifierType::Doi,
                value: "10.1234/abc".into(),
                name: None,
                url: None,
            },
            ServiceIdent {
                property_id: IdentifierType::Custom("KhanCourse".into()),
                value: "kc-42".into(),
                name: None,
                url: None,
            },
        ];
        let m = to_matcher_course(&c);
        assert!(matches!(m.identifiers[0].scheme, cm::IdentifierScheme::Doi));
        assert!(matches!(
            m.identifiers[1].scheme,
            cm::IdentifierScheme::Custom(ref s) if s == "KhanCourse"
        ));
    }

    /// Educational level maps across one-to-one.
    #[test]
    fn educational_level_routes_one_to_one() {
        let mut c = ServiceCourse::new("Intro");
        c.educational_level = Some(ServiceLevel::Undergraduate);
        let m = to_matcher_course(&c);
        assert!(matches!(
            m.educational_level,
            Some(cm::EducationalLevel::Undergraduate)
        ));
    }
}
