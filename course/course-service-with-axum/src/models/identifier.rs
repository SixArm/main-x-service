//! External identifiers attached to a Course.
//!
//! schema.org models `identifier` as `PropertyValue | Text | URL`. We
//! use the `PropertyValue` shape so callers can attach an arbitrary
//! number of identifiers from different schemes (LMS course IDs, DOI,
//! ISBN of the textbook, etc.) and the matcher can short-circuit on
//! deterministic schemes.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One external identifier attached to a `Course`, modelled on
/// schema.org/PropertyValue.
///
/// A `Course` carries a `Vec<CourseIdentifier>`; each entry pairs a
/// scheme ([`property_id`](Self::property_id)) with a scheme-specific
/// [`value`](Self::value). Schemes flagged by
/// [`IdentifierType::is_deterministic`] let the matcher short-circuit
/// scoring to 1.0 on an exact value match.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CourseIdentifier {
    /// The identifier scheme (DOI, LMS course-id, Wikidata, …).
    pub property_id: IdentifierType,
    /// The scheme-specific identifier value (e.g. the DOI string).
    pub value: String,
    /// Optional human-readable label.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional URL that resolves to the identifier's authority.
    #[serde(default)]
    pub url: Option<String>,
}

/// The scheme of a [`CourseIdentifier`].
///
/// `Custom` keeps the surface extensible without forking the enum for
/// every niche registry. See [`is_deterministic`](Self::is_deterministic)
/// for which schemes the matcher treats as globally unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum IdentifierType {
    /// LMS course-id (Canvas, Moodle, Blackboard, …).
    LmsCourseId,
    /// Provider's catalog code (e.g. "CS101"). May not be globally unique.
    CourseCode,
    /// Coursera / edX / Udemy / `FutureLearn` / openSAP course slug.
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
    /// Whether this scheme's values are unique-by-construction across
    /// providers. A match on a deterministic scheme short-circuits
    /// scoring to 1.0. Course code is NOT deterministic (CS101 exists
    /// at many universities).
    ///
    /// # Examples
    ///
    /// ```
    /// use course_service::models::identifier::IdentifierType;
    ///
    /// // Globally-unique schemes short-circuit the matcher.
    /// assert!(IdentifierType::Doi.is_deterministic());
    /// assert!(IdentifierType::Wikidata.is_deterministic());
    ///
    /// // Provider-scoped codes do not.
    /// assert!(!IdentifierType::CourseCode.is_deterministic());
    /// assert!(!IdentifierType::LmsCourseId.is_deterministic());
    /// ```
    #[must_use]
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
