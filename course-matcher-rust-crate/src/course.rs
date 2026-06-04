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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseIdentifier {
    pub scheme: IdentifierScheme,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierScheme {
    LmsCourseId,
    CourseCode,
    PlatformSlug,
    Oer,
    Doi,
    Lom,
    Wikidata,
    Isced,
    Ror,
    Uri,
    Uuid,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EducationalLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    PrimaryEducation,
    SecondaryEducation,
    HigherEducation,
    Undergraduate,
    Graduate,
    Postgraduate,
    Vocational,
    ProfessionalDevelopment,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningResourceType {
    Lecture,
    Tutorial,
    Workshop,
    Assignment,
    Reading,
    Video,
    Audio,
    Exam,
    Simulation,
    Project,
    Discussion,
    Custom(String),
}
