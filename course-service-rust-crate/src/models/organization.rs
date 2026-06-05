//! Provider organisation (schema.org/Organization specialised).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Provider {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub alternate_names: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub same_as: Vec<String>,
    #[serde(default)]
    pub kind: Option<ProviderKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProviderKind {
    University,
    College,
    School,
    OnlinePlatform,
    Bootcamp,
    CertificationBody,
    GovernmentAgency,
    Other(String),
}
