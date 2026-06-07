//! Organization model definition.
//!
//! An [`Organization`] is a standalone record for a venue operator,
//! promoter, presenter, clinic, employer, etc. It carries richer
//! metadata (identifiers, telecom, addresses, hierarchy) than the
//! lightweight [`Party`](crate::models::Party) reference embedded in an
//! event's organizer / performer lists.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use super::{Address, ContactPoint};

/// An organization (venue operator, promoter, presenter,
/// clinic, employer, …). Used both as an `organizer` /
/// `performer` party for an event and as a standalone record
/// when callers want richer organization metadata than a `Party`
/// reference carries.
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
    /// Create a new active organization with the given name. All
    /// collection fields start empty; `id` is generated (UUID v4) and
    /// timestamps are set to now.
    ///
    /// # Examples
    ///
    /// ```
    /// use event_service::models::Organization;
    ///
    /// let org = Organization::new("Cal Performances".into());
    /// assert!(org.active);
    /// assert!(org.telecom.is_empty());
    /// ```
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
