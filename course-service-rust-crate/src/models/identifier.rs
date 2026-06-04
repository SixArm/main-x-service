//! External identifiers attached to a Course.
//!
//! schema.org models `identifier` as `PropertyValue | Text | URL`. We
//! use the `PropertyValue` shape so callers can attach an arbitrary
//! number of identifiers from different schemes (LMS course IDs, DOI,
//! ISBN of the textbook, etc.) and the matcher can short-circuit on
//! deterministic schemes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseIdentifier {
    pub property_id: IdentifierType,
    pub value: String,
    /// Optional human-readable label.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional URL that resolves to the identifier's authority.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierType {
    /// LMS course-id (Canvas, Moodle, Blackboard, …).
    LmsCourseId,
    /// Provider's catalog code (e.g. "CS101"). May not be globally unique.
    CourseCode,
    /// Coursera / edX / Udemy / FutureLearn / openSAP course slug.
    PlatformSlug,
    /// Open Education Resource (OER) identifier.
    Oer,
    /// DOI.
    Doi,
    /// IEEE LOM ID.
    Lom,
    /// Wikidata Q-id (e.g. Q12345).
    Wikidata,
    /// ISCED programme code.
    Isced,
    /// ROR ID for the issuing provider (organisation-scoped).
    Ror,
    /// URI / URN.
    Uri,
    /// UUID.
    Uuid,
    /// Free-form custom scheme.
    Custom(String),
}

impl IdentifierType {
    /// Identifier schemes whose values are unique-by-construction
    /// across providers. A match on these short-circuits scoring to
    /// 1.0. Course code is NOT deterministic (CS101 exists at many
    /// universities).
    pub fn is_deterministic(&self) -> bool {
        matches!(
            self,
            IdentifierType::Doi
                | IdentifierType::Wikidata
                | IdentifierType::Lom
                | IdentifierType::Uri
                | IdentifierType::Uuid
                | IdentifierType::Oer
        )
    }
}
