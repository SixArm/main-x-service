//! Organization model definition

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use super::{Address, ContactPoint};

/// Organization (clinic, hospital, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Organization {
    /// Unique organization identifier
    pub id: Uuid,

    /// Organization identifiers
    pub identifiers: Vec<super::Identifier>,

    /// Active status
    pub active: bool,

    /// Organization type (Hospital, Clinic, etc.)
    pub org_type: Vec<String>,

    /// Organization name
    pub name: String,

    /// Alias names
    pub alias: Vec<String>,

    /// Telecom contacts
    pub telecom: Vec<ContactPoint>,

    /// Addresses
    pub addresses: Vec<Address>,

    /// Part of (parent organization)
    pub part_of: Option<Uuid>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Create a new organization
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            identifiers: Vec::new(),
            active: true,
            org_type: Vec::new(),
            name,
            alias: Vec::new(),
            telecom: Vec::new(),
            addresses: Vec::new(),
            part_of: None,
            created_at: now,
            updated_at: now,
        }
    }
}
