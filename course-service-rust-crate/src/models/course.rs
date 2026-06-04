//! `Course` — central domain entity.
//!
//! Aligned with [schema.org/Course](https://schema.org/Course) and its
//! parent classes (`LearningResource`, `CreativeWork`, `Thing`). The
//! `CourseInstance` collection on this struct is the canonical
//! representation of `hasCourseInstance`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CourseIdentifier, CourseInstance, EducationalCredential, Syllabus};

/// A course — the *template* / abstract description. Specific offerings
/// live in the `instances` collection as `CourseInstance` records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    /// Server-generated UUID. Defaults to a fresh v4 on missing input.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,

    // ─── Thing properties ─────────────────────────────────────────

    /// Required title (schema.org/name).
    pub name: String,
    /// Aliases (schema.org/alternateName).
    #[serde(default)]
    pub alternate_names: Vec<String>,
    /// Long-form description (schema.org/description).
    #[serde(default)]
    pub description: Option<String>,
    /// Short distinguishing description.
    #[serde(default)]
    pub disambiguating_description: Option<String>,
    /// Canonical URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Image URLs.
    #[serde(default)]
    pub image: Vec<String>,
    /// Cross-system identity URLs (Wikidata, OER repos, …).
    #[serde(default)]
    pub same_as: Vec<String>,
    /// Tags / keywords (schema.org/keywords).
    #[serde(default)]
    pub keywords: Vec<String>,
    /// External system identifiers — DOI, OER ID, Coursera-id, etc.
    #[serde(default)]
    pub identifiers: Vec<CourseIdentifier>,
    /// More specific schema.org subtype URL (e.g. for a certificate course).
    #[serde(default)]
    pub additional_type: Option<String>,
    /// Active flag — soft-delete uses this.
    #[serde(default = "default_true")]
    pub active: bool,

    // ─── CreativeWork properties ──────────────────────────────────

    /// schema.org/about — subjects covered.
    #[serde(default)]
    pub about: Vec<String>,
    /// schema.org/audience — intended audience (e.g. "undergraduate").
    #[serde(default)]
    pub audience: Option<String>,
    /// schema.org/inLanguage — BCP-47 language codes.
    #[serde(default)]
    pub in_language: Vec<String>,
    /// schema.org/license — applicable license URL or text.
    #[serde(default)]
    pub license: Option<String>,
    /// schema.org/typicalAgeRange — e.g. "18-22".
    #[serde(default)]
    pub typical_age_range: Option<String>,
    /// schema.org/timeRequired — ISO 8601 duration (e.g. "PT45H").
    #[serde(default)]
    pub time_required: Option<String>,
    /// schema.org/version — content version string.
    #[serde(default)]
    pub version: Option<String>,
    /// schema.org/isAccessibleForFree.
    #[serde(default)]
    pub is_accessible_for_free: Option<bool>,

    // ─── LearningResource properties ──────────────────────────────

    /// schema.org/teaches — competencies taught.
    #[serde(default)]
    pub teaches: Vec<String>,
    /// schema.org/assesses — competencies assessed.
    #[serde(default)]
    pub assesses: Vec<String>,
    /// schema.org/competencyRequired — prerequisite competencies.
    #[serde(default)]
    pub competency_required: Vec<String>,
    /// schema.org/educationalLevel.
    #[serde(default)]
    pub educational_level: Option<EducationalLevel>,
    /// schema.org/educationalUse.
    #[serde(default)]
    pub educational_use: Option<String>,
    /// schema.org/learningResourceType.
    #[serde(default)]
    pub learning_resource_type: Option<LearningResourceType>,
    /// schema.org/interactivityType.
    #[serde(default)]
    pub interactivity_type: Option<InteractivityType>,

    // ─── Course-specific properties ───────────────────────────────

    /// Provider's identifier (e.g. "CS101"). schema.org/courseCode.
    #[serde(default)]
    pub course_code: Option<String>,
    /// schema.org/numberOfCredits.
    #[serde(default)]
    pub number_of_credits: Option<u32>,
    /// schema.org/coursePrerequisites — free-text or URLs.
    #[serde(default)]
    pub course_prerequisites: Vec<String>,
    /// schema.org/availableLanguage — BCP-47 codes.
    #[serde(default)]
    pub available_language: Vec<String>,
    /// schema.org/financialAidEligible — aid program names.
    #[serde(default)]
    pub financial_aid_eligible: Vec<String>,
    /// schema.org/educationalCredentialAwarded.
    #[serde(default)]
    pub educational_credential_awarded: Option<EducationalCredential>,
    /// schema.org/occupationalCredentialAwarded.
    #[serde(default)]
    pub occupational_credential_awarded: Option<EducationalCredential>,
    /// schema.org/totalHistoricalEnrollment.
    #[serde(default)]
    pub total_historical_enrollment: Option<u64>,
    /// schema.org/syllabusSections — full table of contents.
    #[serde(default)]
    pub syllabus_sections: Vec<Syllabus>,
    /// schema.org/hasCourseInstance — specific offerings.
    #[serde(default)]
    pub instances: Vec<CourseInstance>,

    // ─── Lifecycle ────────────────────────────────────────────────

    /// Course lifecycle state (Draft / Published / Archived / Retired).
    #[serde(default)]
    pub status: CourseStatus,
    /// Cross-references to other courses (replaces, supersedes, …).
    #[serde(default)]
    pub links: Vec<CourseLink>,
    /// Provider — the issuing organisation. Optional foreign key into a
    /// future provider directory or the inline `Provider` struct.
    #[serde(default)]
    pub provider_id: Option<Uuid>,

    /// Soft-delete timestamp; `None` means active.
    #[serde(default)]
    pub deleted_at: Option<DateTime<Utc>>,

    /// Created server-side on insert.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Updated server-side on insert and update.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl Course {
    /// Construct a minimal valid Course with just the required name.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            alternate_names: vec![],
            description: None,
            disambiguating_description: None,
            url: None,
            image: vec![],
            same_as: vec![],
            keywords: vec![],
            identifiers: vec![],
            additional_type: None,
            active: true,
            about: vec![],
            audience: None,
            in_language: vec![],
            license: None,
            typical_age_range: None,
            time_required: None,
            version: None,
            is_accessible_for_free: None,
            teaches: vec![],
            assesses: vec![],
            competency_required: vec![],
            educational_level: None,
            educational_use: None,
            learning_resource_type: None,
            interactivity_type: None,
            course_code: None,
            number_of_credits: None,
            course_prerequisites: vec![],
            available_language: vec![],
            financial_aid_eligible: vec![],
            educational_credential_awarded: None,
            occupational_credential_awarded: None,
            total_historical_enrollment: None,
            syllabus_sections: vec![],
            instances: vec![],
            status: CourseStatus::default(),
            links: vec![],
            provider_id: None,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Course lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CourseStatus {
    /// Draft — not yet visible to learners.
    Draft,
    /// Published — can be offered.
    #[default]
    Published,
    /// Archived — historical record, no new instances.
    Archived,
    /// Retired — replaced by a successor; see `links`.
    Retired,
}

/// schema.org/educationalLevel — common buckets. `Custom` keeps the
/// surface extensible without forking the enum.
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

/// schema.org/learningResourceType — coarse buckets.
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

/// schema.org/interactivityType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractivityType {
    /// Hands-on (problem-solving, simulation, role-play).
    Active,
    /// Read / view / listen.
    Expositive,
    /// Both active and expositive.
    Mixed,
}

/// schema.org-style cross-references between Course records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseLink {
    pub other_course_id: Uuid,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkType {
    /// This course replaces `other`.
    Replaces,
    /// `other` replaces this course.
    ReplacedBy,
    /// Loose cross-reference ("see also").
    Seealso,
    /// `other` is a prerequisite for this course.
    Prerequisite,
    /// `other` is a successor / next-in-sequence.
    Successor,
}
