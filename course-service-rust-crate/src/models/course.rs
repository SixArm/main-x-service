//! `Course` — central domain entity.
//!
//! Aligned with [schema.org/Course](https://schema.org/Course) and its
//! parent classes (`LearningResource`, `CreativeWork`, `Thing`). The
//! `CourseInstance` collection on this struct is the canonical
//! representation of `hasCourseInstance`.
//!
//! A [`Course`] is the abstract *template* ("CS101 — Intro to Computer
//! Science"); each concrete offering ("CS101, Fall 2026, Prof. Smith")
//! is a [`CourseInstance`]
//! in the [`instances`](Course::instances) collection. The struct is a
//! plain serde value object: construction never validates, so callers
//! can deserialize raw input first and run `crate::validation` second.
//!
//! # Examples
//!
//! ```
//! use course_service::models::course::{Course, CourseStatus};
//!
//! let mut c = Course::new("Introduction to Computer Science");
//! c.course_code = Some("CS101".into());
//! c.keywords = vec!["programming".into(), "algorithms".into()];
//!
//! assert_eq!(c.name, "Introduction to Computer Science");
//! // A fresh course is active and Published by default.
//! assert!(c.active);
//! assert_eq!(c.status, CourseStatus::Published);
//! ```

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{CourseIdentifier, CourseInstance, EducationalCredential, Syllabus};

/// A course — the *template* / abstract description. Specific offerings
/// live in the `instances` collection as `CourseInstance` records.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    pub deleted_at: Option<Timestamp>,

    /// Created server-side on insert.
    #[serde(default = "Timestamp::now")]
    pub created_at: Timestamp,
    /// Updated server-side on insert and update.
    #[serde(default = "Timestamp::now")]
    pub updated_at: Timestamp,
}

/// serde `#[serde(default = ...)]` helper for the [`Course::active`]
/// flag, which defaults to `true` (a freshly deserialized course is
/// active unless the input explicitly says otherwise).
fn default_true() -> bool {
    true
}

impl Course {
    /// Construct a minimal valid Course with just the required name.
    ///
    /// Generates a fresh v4 [`id`](Course::id), stamps
    /// [`created_at`](Course::created_at) /
    /// [`updated_at`](Course::updated_at) to the current instant, sets
    /// [`active`](Course::active) to `true`, and leaves every optional
    /// field empty / `None`. This is the canonical builder used by
    /// tests and handlers; set further fields directly afterwards.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_service::models::course::Course;
    ///
    /// let c = Course::new("Linear Algebra");
    /// assert_eq!(c.name, "Linear Algebra");
    /// assert!(c.identifiers.is_empty());
    /// assert!(c.deleted_at.is_none());
    /// // created_at and updated_at are set to the same instant.
    /// assert_eq!(c.created_at, c.updated_at);
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        let now = Timestamp::now();
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
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
///
/// Maps one-to-one onto `course_matcher::EducationalLevel` via
/// `crate::matching::adapter`, so the level participates in the
/// matcher's educational-level component score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum EducationalLevel {
    /// Entry level; no prior knowledge assumed.
    Beginner,
    /// Some prior knowledge assumed.
    Intermediate,
    /// Substantial prior knowledge assumed.
    Advanced,
    /// Mastery level.
    Expert,
    /// Primary / elementary schooling.
    PrimaryEducation,
    /// Secondary / high-school schooling.
    SecondaryEducation,
    /// Tertiary education, unspecified degree level.
    HigherEducation,
    /// Bachelor-level study.
    Undergraduate,
    /// Master-level study.
    Graduate,
    /// Doctoral / post-master study.
    Postgraduate,
    /// Trade / vocational training.
    Vocational,
    /// Continuing professional development.
    ProfessionalDevelopment,
    /// Any level not covered above, carried verbatim.
    Custom(String),
}

/// schema.org/learningResourceType — coarse buckets.
///
/// Maps one-to-one onto `course_matcher::LearningResourceType` via
/// `crate::matching::adapter`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum LearningResourceType {
    /// A lecture (instructor presents to learners).
    Lecture,
    /// A guided, step-by-step tutorial.
    Tutorial,
    /// A hands-on workshop.
    Workshop,
    /// A graded or ungraded assignment.
    Assignment,
    /// A reading (text resource).
    Reading,
    /// A video resource.
    Video,
    /// An audio resource.
    Audio,
    /// An exam / assessment.
    Exam,
    /// An interactive simulation.
    Simulation,
    /// A project (often capstone / portfolio).
    Project,
    /// A discussion / seminar.
    Discussion,
    /// Any type not covered above, carried verbatim.
    Custom(String),
}

/// schema.org/interactivityType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum InteractivityType {
    /// Hands-on (problem-solving, simulation, role-play).
    Active,
    /// Read / view / listen.
    Expositive,
    /// Both active and expositive.
    Mixed,
}

/// schema.org-style cross-reference between two Course records.
///
/// Stored in its own `course_links` child table. The merge workflow
/// adds a [`LinkType::Replaces`] link from the surviving course to the
/// folded duplicate so the audit chain stays navigable.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CourseLink {
    /// The id of the course on the other end of the relationship.
    pub other_course_id: Uuid,
    /// How this course relates to [`other_course_id`](Self::other_course_id).
    pub link_type: LinkType,
}

/// The kind of relationship a [`CourseLink`] expresses, from the
/// perspective of the course that owns the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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
