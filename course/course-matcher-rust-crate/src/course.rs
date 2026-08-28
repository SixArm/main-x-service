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
    /// Typed references to other courses (by opaque id) in the
    /// consuming registry — e.g. "this course is similar to course
    /// X". A **supporting** signal only: never identifying on its
    /// own, and the matcher never resolves the reference (it has no
    /// registry) — it only compares the two courses' relationship
    /// **sets** via typed-set Jaccard. Default empty. See
    /// [`RelationshipRef`] / [`RelationKind`]; spec §5.1 / §6.1 /
    /// §23 T-11.
    #[serde(default)]
    pub relationships: Vec<RelationshipRef>,
    /// Operator-applied free-text labels (e.g. `"online"`,
    /// `"self-paced"`). Stored verbatim; compared case-insensitively
    /// at match time, consistent with the crate's normalise-at-
    /// match-time convention for names / keywords. A **supporting**
    /// signal only: never identifying on its own. Distinct from
    /// [`Course::keywords`] (descriptive subject terms) but scored
    /// the same way. Default empty. See spec §5.2 / §6 / §13a /
    /// §23 T-12.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Course {
    /// Construct a `Course` with just the required name; every other
    /// field defaults to empty / `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::Course;
    ///
    /// let course = Course::new("Introduction to Computer Science");
    /// assert_eq!(course.name, "Introduction to Computer Science");
    /// assert!(course.identifiers.is_empty());
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// The kind of typed relationship one [`Course`] asserts toward
/// another, carried on a [`RelationshipRef`].
///
/// Mirrors the consuming service's own `Course` relationship
/// vocabulary. `SimilarTo` is symmetric; `HigherLevelThan` /
/// `LowerLevelThan` are inverses of each other — but the matcher does
/// **not** resolve or cross-check that inverse relationship; it only
/// compares the raw `(relation, course_id)` pairs each side asserts
/// (spec §5.1 / §6.1). The enum is expected to grow as consuming
/// services add relationship kinds; a new variant is purely additive.
///
/// # Examples
///
/// ```
/// use course_matcher::RelationKind;
///
/// let k = RelationKind::SimilarTo;
/// assert_eq!(k, RelationKind::SimilarTo);
/// assert_ne!(k, RelationKind::HigherLevelThan);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    /// This course is similar to the referenced course (symmetric).
    SimilarTo,
    /// This course is at a higher educational level than the referenced course.
    HigherLevelThan,
    /// This course is at a lower educational level than the referenced course.
    LowerLevelThan,
}

/// A typed reference from one [`Course`] to another, by opaque id in
/// the consuming registry (e.g. "this course is similar to course
/// `X`").
///
/// `RelationshipRef` is a **supporting** matching signal, not an
/// identifying one: the matcher never resolves `course_id` against a
/// registry (it has none) — it only compares the two courses'
/// relationship **sets** via typed-set Jaccard over `(relation,
/// course_id)` pairs (spec §5.1 / §6.1 / §23 T-11).
///
/// Construct via [`RelationshipRef::new`], which trims `course_id` and
/// rejects an empty result, so two records carrying different
/// incidental whitespace around the same id compare equal.
///
/// # Examples
///
/// ```
/// use course_matcher::{RelationKind, RelationshipRef};
///
/// let r = RelationshipRef::new(RelationKind::SimilarTo, " course-42 ").unwrap();
/// assert_eq!(r.course_id, "course-42");
/// assert_eq!(r.relation, RelationKind::SimilarTo);
///
/// assert!(RelationshipRef::new(RelationKind::HigherLevelThan, "   ").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipRef {
    /// The kind of relationship this side asserts. See [`RelationKind`].
    pub relation: RelationKind,
    /// Opaque id of the related course in the consuming registry.
    /// Whitespace-trimmed and non-empty — see [`RelationshipRef::new`].
    pub course_id: String,
}

impl RelationshipRef {
    /// Construct a relationship reference, trimming `course_id` and
    /// rejecting an empty result.
    ///
    /// Returns `None` when `course_id` is empty after trimming.
    ///
    /// ```
    /// use course_matcher::{RelationKind, RelationshipRef};
    /// let r = RelationshipRef::new(RelationKind::LowerLevelThan, "course-7").unwrap();
    /// assert_eq!(r.course_id, "course-7");
    /// assert!(RelationshipRef::new(RelationKind::LowerLevelThan, "").is_none());
    /// ```
    #[must_use]
    pub fn new(relation: RelationKind, course_id: impl AsRef<str>) -> Option<Self> {
        let course_id = course_id.as_ref().trim();
        if course_id.is_empty() {
            return None;
        }
        Some(Self {
            relation,
            course_id: course_id.to_string(),
        })
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
    /// Open Education Resource identifier (`OERCommons`, MERLOT, …).
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
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::IdentifierScheme;
    ///
    /// assert!(IdentifierScheme::Doi.is_deterministic());
    /// assert!(!IdentifierScheme::CourseCode.is_deterministic());
    /// ```
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_name_and_defaults_everything_else() {
        let c = Course::new("Introduction to Computer Science");
        assert_eq!(c.name, "Introduction to Computer Science");
        assert!(c.alternate_names.is_empty());
        assert!(c.course_code.is_none());
        assert!(c.provider_id.is_none());
        assert!(c.provider_name.is_none());
        assert!(c.educational_level.is_none());
        assert!(c.learning_resource_type.is_none());
        assert!(c.keywords.is_empty());
        assert!(c.teaches.is_empty());
        assert!(c.identifiers.is_empty());
        assert!(c.same_as.is_empty());
        assert!(c.in_language.is_empty());
        assert!(c.relationships.is_empty());
        assert!(c.tags.is_empty());
    }

    #[test]
    fn default_course_has_empty_name() {
        let c = Course::default();
        assert_eq!(c.name, "");
        assert!(c.identifiers.is_empty());
    }

    #[test]
    fn deterministic_schemes_are_exactly_the_globally_unique_ones() {
        // The six deterministic schemes (per spec §15) short-circuit to 1.0.
        for scheme in [
            IdentifierScheme::Doi,
            IdentifierScheme::Wikidata,
            IdentifierScheme::Lom,
            IdentifierScheme::Oer,
            IdentifierScheme::Uri,
            IdentifierScheme::Uuid,
        ] {
            assert!(
                scheme.is_deterministic(),
                "{scheme:?} should be deterministic"
            );
        }
        // The provider-scoped schemes must NOT short-circuit — CS101
        // exists at every university.
        for scheme in [
            IdentifierScheme::LmsCourseId,
            IdentifierScheme::CourseCode,
            IdentifierScheme::PlatformSlug,
            IdentifierScheme::Isced,
            IdentifierScheme::Ror,
            IdentifierScheme::Custom("KhanCourse".into()),
        ] {
            assert!(
                !scheme.is_deterministic(),
                "{scheme:?} must not be deterministic"
            );
        }
    }

    #[test]
    fn course_serde_round_trips() {
        let mut c = Course::new("Café Programming Élève");
        c.alternate_names = vec!["Programmation".into()];
        c.course_code = Some("CS101".into());
        c.provider_id = Some("ror-021nxhr62".into());
        c.educational_level = Some(EducationalLevel::Undergraduate);
        c.learning_resource_type = Some(LearningResourceType::Lecture);
        c.keywords = vec!["programming".into()];
        c.teaches = vec!["recursion".into()];
        c.identifiers = vec![CourseIdentifier {
            scheme: IdentifierScheme::Doi,
            value: "10.1234/abc".into(),
        }];
        c.same_as = vec!["https://www.wikidata.org/wiki/Q123".into()];
        c.in_language = vec!["en".into()];
        c.relationships = vec![RelationshipRef::new(RelationKind::SimilarTo, "course-2").unwrap()];
        c.tags = vec!["online".into()];

        let json = serde_json::to_string(&c).expect("serialize");
        let back: Course = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.name, c.name);
        assert_eq!(back.alternate_names, c.alternate_names);
        assert_eq!(back.course_code, c.course_code);
        assert_eq!(back.provider_id, c.provider_id);
        assert_eq!(back.educational_level, c.educational_level);
        assert_eq!(back.learning_resource_type, c.learning_resource_type);
        assert_eq!(back.keywords, c.keywords);
        assert_eq!(back.teaches, c.teaches);
        assert_eq!(back.same_as, c.same_as);
        assert_eq!(back.in_language, c.in_language);
        assert_eq!(back.relationships, c.relationships);
        assert_eq!(back.tags, c.tags);
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].scheme, IdentifierScheme::Doi);
        assert_eq!(back.identifiers[0].value, "10.1234/abc");
    }

    #[test]
    fn relationship_ref_new_trims_and_rejects_empty() {
        let r = RelationshipRef::new(RelationKind::SimilarTo, "  course-42  ").unwrap();
        assert_eq!(r.course_id, "course-42");
        assert_eq!(r.relation, RelationKind::SimilarTo);
        assert!(RelationshipRef::new(RelationKind::HigherLevelThan, "").is_none());
        assert!(RelationshipRef::new(RelationKind::LowerLevelThan, "   ").is_none());
    }

    #[test]
    fn relation_kind_round_trips_through_json() {
        for k in [
            RelationKind::SimilarTo,
            RelationKind::HigherLevelThan,
            RelationKind::LowerLevelThan,
        ] {
            let json = serde_json::to_string(&k).expect("serialize");
            let back: RelationKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, k);
        }
    }

    #[test]
    fn custom_scheme_round_trips_its_label() {
        let scheme = IdentifierScheme::Custom("KhanCourse".into());
        let json = serde_json::to_string(&scheme).expect("serialize");
        let back: IdentifierScheme = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, scheme);
    }

    #[test]
    fn course_deserialises_from_name_only_json() {
        // Every non-name field is #[serde(default)] — a bare name must parse.
        let c: Course = serde_json::from_str(r#"{"name":"Lone Course"}"#).expect("deserialize");
        assert_eq!(c.name, "Lone Course");
        assert!(c.keywords.is_empty());
        assert!(c.identifiers.is_empty());
        assert!(c.relationships.is_empty());
        assert!(c.tags.is_empty());
    }
}
