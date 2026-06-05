//! schema.org/EducationalOccupationalCredential.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EducationalCredential {
    pub name: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CredentialCategory {
    Certificate,
    Diploma,
    Degree,
    Badge,
    Microcredential,
    License,
    Custom(String),
}
