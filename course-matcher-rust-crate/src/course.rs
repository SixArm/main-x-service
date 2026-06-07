//! Domain model — a slim, library-friendly subset of
//! `schema.org/Course`. The full service-side `Course` carries many
//! more properties; the matcher only models what the algorithm uses.

use serde::{Deserialize, Serialize};

/// Pairwise input to the matcher.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Course {
    /// Required title.
    pub name: String,
    /// schema.org/alternateName — also tried when scoring `name`.
    #[serde(default)]
    pub alternate_names: Vec<String>,
    /// schema.org/courseCode (e.g. "CS101"). Provider-scoped — only
    /// meaningful when both records share `provider_id`.
    #[serde(default)]
    pub course_code: Option<String>,
    /// Provider / issuing organisation identifier (opaque to the
    /// matcher). When both records share this value the
    /// `provider_score` is `1.0`.
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Fallback when `provider_id` is unset — Jaro-Winkler on this
    /// value contributes to the `provider_score`.
    #[serde(default)]
    pub provider_name: Option<String>,
    /// schema.org/educationalLevel.
    #[serde(default)]
    pub educational_level: Option<EducationalLevel>,
    /// schema.org/learningResourceType.
    #[serde(default)]
    pub learning_resource_type: Option<LearningResourceType>,
    /// schema.org/keywords — lowercased + trimmed before scoring.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// schema.org/teaches — competencies taught.
    #[serde(default)]
    pub teaches: Vec<String>,
    /// External identifiers — DOI, Wikidata, LMS course id, etc.
    #[serde(default)]
    pub identifiers: Vec<CourseIdentifier>,
    /// schema.org/sameAs — cross-system identity URLs (Wikidata page,
    /// OER repo entry, etc.). Used by the deterministic short-circuit.
    #[serde(default)]
    pub same_as: Vec<String>,
    /// schema.org/inLanguage — BCP-47 codes.
    #[serde(default)]
    pub in_language: Vec<String>,
}

impl Course {
    /// Construct a Course with just the required name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// An external identifier for a course: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseIdentifier {
    /// The scheme under which `value` is published.
    pub scheme: IdentifierScheme,
    /// The identifier value within `scheme`.
    pub value: String,
}

/// The scheme under which an identifier's `value` is published.
///
/// Schemes marked **deterministic** (DOI / Wikidata / LOM / OER /
/// URI / UUID) are globally unique by construction — a match on
/// these pins the final score to `1.0` via the R-0 short-circuit.
/// Schemes marked **provider-scoped** (LMS course-id, course-code,
/// platform-slug, ISCED, ROR) only make sense in the context of
/// their issuing organisation and are intentionally NOT
/// deterministic: `CS101` in Canvas at one school is not the same
/// row as `CS101` in Canvas at another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierScheme {
    /// Learning Management System course-id (Canvas / Moodle /
    /// Blackboard / etc). Provider-scoped. Example: `canvas-12345`.
    LmsCourseId,
    /// Provider's catalog code (e.g. `CS101`, `MATH 220`).
    /// Provider-scoped — short-circuits only via R-1
    /// (`provider_id + course_code`).
    CourseCode,
    /// MOOC / online-learning platform slug. Provider-scoped.
    /// Example: `coursera:learn-to-program`, `edx:MITx/6.00.1x`.
    PlatformSlug,
    /// Open Education Resource identifier (OERCommons, MERLOT, …).
    /// **Deterministic.** Example: `oercommons:60132`.
    Oer,
    /// Digital Object Identifier. **Deterministic.**
    /// Example: `10.1234/intro-cs`.
    Doi,
    /// IEEE Learning Object Metadata identifier. **Deterministic.**
    /// Example: `lom:OEM-2025-CS-101`.
    Lom,
    /// Wikidata entity id. **Deterministic.** Example: `Q12345`.
    Wikidata,
    /// UNESCO International Standard Classification of Education
    /// programme code. Provider-scoped (classifies the field, not
    /// the offering). Example: `0613` (Software & applications dev).
    Isced,
    /// Research Organization Registry id for the issuing provider.
    /// Provider-scoped. Example: `ror-021nxhr62`.
    Ror,
    /// Generic URI / URN. **Deterministic.**
    /// Example: `urn:isbn:978-0-13-468599-1`.
    Uri,
    /// Bare UUID. **Deterministic.**
    /// Example: `550e8400-e29b-41d4-a716-446655440000`.
    Uuid,
    /// Free-form custom scheme with a caller-supplied label.
    /// Provider-scoped. Example: `Custom("KhanCourse")`.
    Custom(String),
}

impl IdentifierScheme {
    /// Schemes whose values are unique by construction across
    /// providers. A match on these pins the final score to `1.0`.
    pub fn is_deterministic(&self) -> bool {
        matches!(
            self,
            IdentifierScheme::Doi
                | IdentifierScheme::Wikidata
                | IdentifierScheme::Lom
                | IdentifierScheme::Oer
                | IdentifierScheme::Uri
                | IdentifierScheme::Uuid
        )
    }
}

/// schema.org/educationalLevel — the intended difficulty or stage of study.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EducationalLevel {
    /// Introductory level, no prior knowledge assumed.
    Beginner,
    /// Builds on beginner foundations.
    Intermediate,
    /// Assumes substantial prior knowledge.
    Advanced,
    /// Mastery level.
    Expert,
    /// Primary / elementary schooling.
    PrimaryEducation,
    /// Secondary / high-school schooling.
    SecondaryEducation,
    /// Tertiary / higher education in general.
    HigherEducation,
    /// Undergraduate degree level.
    Undergraduate,
    /// Graduate (master's) level.
    Graduate,
    /// Postgraduate (doctoral) level.
    Postgraduate,
    /// Vocational / trade training.
    Vocational,
    /// Continuing professional development.
    ProfessionalDevelopment,
    /// Free-form custom level with a caller-supplied label.
    Custom(String),
}

/// schema.org/learningResourceType — the form a course offering takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningResourceType {
    /// Instructor-led lecture.
    Lecture,
    /// Guided tutorial.
    Tutorial,
    /// Hands-on workshop.
    Workshop,
    /// Graded assignment.
    Assignment,
    /// Reading material.
    Reading,
    /// Video content.
    Video,
    /// Audio content.
    Audio,
    /// Assessment / exam.
    Exam,
    /// Interactive simulation.
    Simulation,
    /// Project work.
    Project,
    /// Discussion / seminar.
    Discussion,
    /// Free-form custom type with a caller-supplied label.
    Custom(String),
}
