//! schema.org/Syllabus — a section of a Course's content.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Syllabus {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Heading / section title.
    pub name: String,
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
