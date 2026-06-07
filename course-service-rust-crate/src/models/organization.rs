//! Provider organisation (schema.org/Organization specialised).
//!
//! A `Provider` is the institution that authors or offers a `Course`
//! (a university, online platform, bootcamp, …). Course code identifiers
//! are scoped to a provider, so the matcher uses provider identity to
//! disambiguate otherwise-colliding codes like "CS101".

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// An organisation that authors or offers courses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Provider {
    /// Server-generated provider UUID.
    pub id: Uuid,
    /// Canonical organisation name.
    pub name: String,
    /// Other names the provider is known by (acronyms, former names).
    #[serde(default)]
    pub alternate_names: Vec<String>,
    /// Provider's primary website URL.
    #[serde(default)]
    pub url: Option<String>,
    /// schema.org/sameAs — external authority URLs (Wikidata, ROR, …).
    #[serde(default)]
    pub same_as: Vec<String>,
    /// Coarse classification of the provider.
    #[serde(default)]
    pub kind: Option<ProviderKind>,
}

/// Coarse classification of a [`Provider`]. `Other` keeps the surface
/// extensible without forking the enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProviderKind {
    /// A degree-granting university.
    University,
    /// A college.
    College,
    /// A primary or secondary school.
    School,
    /// An online learning platform (Coursera, edX, Udemy, …).
    OnlinePlatform,
    /// An intensive vocational bootcamp.
    Bootcamp,
    /// A professional certification body.
    CertificationBody,
    /// A government agency.
    GovernmentAgency,
    /// Any provider kind not covered above, carried verbatim.
    Other(String),
}
