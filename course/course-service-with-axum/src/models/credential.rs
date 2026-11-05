//! schema.org/EducationalOccupationalCredential.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A credential a course can award — schema.org/EducationalOccupationalCredential.
///
/// Attached to a [`Course`](crate::models::course::Course) via
/// `educational_credential_awarded` / `occupational_credential_awarded`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EducationalCredential {
    /// Credential title (e.g. "Certificate of Completion").
    pub name: String,
    /// Coarse credential class.
    #[serde(default)]
    pub category: Option<CredentialCategory>,
    /// e.g. ISCED level.
    #[serde(default)]
    pub educational_level: Option<String>,
    /// Issuing competent authority (free text or org URL).
    #[serde(default)]
    pub recognized_by: Option<String>,
    /// Credential URL / Open Badge / Verifiable Credential ID.
    #[serde(default)]
    pub url: Option<String>,
}

/// Coarse classification of an [`EducationalCredential`]. `Custom`
/// keeps the surface extensible without forking the enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CredentialCategory {
    /// A certificate.
    Certificate,
    /// A diploma.
    Diploma,
    /// An academic degree.
    Degree,
    /// A digital badge (e.g. Open Badge).
    Badge,
    /// A microcredential / nanodegree.
    Microcredential,
    /// A professional license.
    License,
    /// Any category not covered above, carried verbatim.
    Custom(String),
}
