//! The [`Organization`] model — a clinic, hospital, or other body.
//!
//! Organizations appear as the `managing_organization` of a person and
//! as the `assigner` of identifiers. They reuse the shared
//! [`crate::models::Address`], [`crate::models::ContactPoint`], and
//! [`crate::models::Identifier`] value objects, and can nest via
//! `part_of` to model a parent/child hierarchy.
//!
//! # Examples
//!
//! ```
//! use person_service::models::Organization;
//!
//! let org = Organization::new("General Hospital".to_string());
//! assert!(org.active);
//! assert!(org.addresses.is_empty());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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
    /// Create a new active organization from a name.
    ///
    /// Generates a fresh UUID, stamps timestamps, marks it `active`, and
    /// leaves every collection empty and `part_of` unset.
    #[must_use]
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
