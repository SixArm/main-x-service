//! schema.org/Syllabus — a section of a Course's content.
//!
//! A `Syllabus` node is one entry in a course's table of contents. The
//! [`sub_sections`](Syllabus::sub_sections) field makes the type
//! recursive, so a course outline is a tree of sections.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// One section of a course outline; recursive via
/// [`sub_sections`](Self::sub_sections).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Syllabus {
    /// Server-generated UUID. Defaults to a fresh v4 on missing input.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Heading / section title.
    pub name: String,
    /// Free-text description of the section.
    #[serde(default)]
    pub description: Option<String>,
    /// Position in the table of contents.
    #[serde(default)]
    pub position: Option<u32>,
    /// schema.org/teaches — competencies covered by this section.
    #[serde(default)]
    pub teaches: Vec<String>,
    /// ISO 8601 duration (e.g. "PT2H").
    #[serde(default)]
    pub time_required: Option<String>,
    /// Optional reading list / supplementary resources (URLs).
    #[serde(default)]
    pub resources: Vec<String>,
    /// Nested sub-sections.
    #[serde(default)]
    pub sub_sections: Vec<Syllabus>,
}
