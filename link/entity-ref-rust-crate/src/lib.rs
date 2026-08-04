//! `entity-ref` — the one shared **contract** for cross-service entity
//! linking in the Main X Index family
//! (`agents/share/cross-service-linking.md` §3, §9).
//!
//! A record that lives in another service is named by an opaque **URN
//! string** `"<entity_type>:<uuid>"` (e.g. `person:0c4f1e2a-…`). This
//! crate owns:
//!
//! - [`EntityType`] — the globally-unique entity discriminator and its
//!   static `entity_type → owning service` map (a multi-entity service
//!   like `course` hosts both `course` and `courseinstance`, which is why
//!   the **type**, not the service, is the discriminator);
//! - [`EntityRef`] — the `{entity_type, id}` value type that parses,
//!   `Display`s, and (de)serialises as that single URN string, so the
//!   aggregator can index it as one `TEXT` column;
//! - [`EdgeKind`] — the closed **v1 edge-kind registry** (§9): each kind
//!   fixes its endpoint types, direction, temporality, inverse, and
//!   sensitivity, and can validate an endpoint pair.
//!
//! It is pure data with no behaviour beyond parsing/validation — no I/O,
//! no clock, no panics — and is deliberately **dependency-light**. The
//! original rollout plan (`agents/share/cross-service-linking.md` §2/§11)
//! framed this as copyable per project until a second non-aggregator
//! consumer justified a shared dependency; in practice it is embedded as
//! a real Cargo `path` dependency by eight crates (as of 2026-08-04):
//! the `link-graph-service-with-loco` aggregator, the three entity
//! services that originate edges (person, worker, case), and four
//! consumer apps (contact-relationship-management,
//! content-management-system, patient-flow,
//! workforce-planning-management) that validate/dereference cross-service
//! refs without originating edges. See the crate's `README.md` for the
//! full picture. This crate's own contract shipped as rollout **step 1**
//! ("land the contracts; no behaviour yet").

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The entity type of a linked record — globally unique across the
/// family. The wire token is the lowercase snake-case form
/// ([`EntityType::as_str`]); [`EntityType::service`] maps it to the
/// owning service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntityType {
    /// General person registry.
    Person,
    /// Workforce / professional registry.
    Worker,
    /// schema.org/Organization registry.
    Organization,
    /// Governmental case registry.
    Case,
    /// schema.org/Place registry.
    Place,
    /// schema.org/Thing registry.
    Thing,
    /// schema.org/Event registry.
    Event,
    /// schema.org/Course template registry.
    Course,
    /// A specific offering of a course (`course-service` sub-resource).
    CourseInstance,
    /// Clinical care-pathway registry.
    CarePathway,
}

impl EntityType {
    /// Every entity type, in a stable order (for iteration / tests).
    pub const ALL: [EntityType; 10] = [
        EntityType::Person,
        EntityType::Worker,
        EntityType::Organization,
        EntityType::Case,
        EntityType::Place,
        EntityType::Thing,
        EntityType::Event,
        EntityType::Course,
        EntityType::CourseInstance,
        EntityType::CarePathway,
    ];

    /// The lowercase wire token used in the URN (e.g. `"care_pathway"`,
    /// `"courseinstance"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            EntityType::Person => "person",
            EntityType::Worker => "worker",
            EntityType::Organization => "organization",
            EntityType::Case => "case",
            EntityType::Place => "place",
            EntityType::Thing => "thing",
            EntityType::Event => "event",
            EntityType::Course => "course",
            EntityType::CourseInstance => "courseinstance",
            EntityType::CarePathway => "care_pathway",
        }
    }

    /// The service that owns records of this type. Multiple types can map
    /// to one service (course + courseinstance → `course-service`), which
    /// is why the ref encodes the type, not the service.
    #[must_use]
    pub const fn service(self) -> &'static str {
        match self {
            EntityType::Person => "person-service",
            EntityType::Worker => "worker-service",
            EntityType::Organization => "organization-service",
            EntityType::Case => "case-service",
            EntityType::Place => "place-service",
            EntityType::Thing => "thing-service",
            EntityType::Event => "event-service",
            EntityType::Course | EntityType::CourseInstance => "course-service",
            EntityType::CarePathway => "care-pathway-service",
        }
    }

    /// Parse a wire token into an [`EntityType`], or `None` if unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == token)
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error parsing an [`EntityRef`] from its URN string form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseEntityRefError {
    /// Not exactly one `:` separating a non-empty type and id.
    #[error("malformed entity ref (expected `entity_type:uuid`): {0:?}")]
    Malformed(String),
    /// The type token is not a known [`EntityType`].
    #[error("unknown entity type: {0:?}")]
    UnknownType(String),
    /// The id is not a valid UUID.
    #[error("invalid uuid in entity ref: {0:?}")]
    BadId(String),
}

/// A reference to a record that lives in another service, identified by
/// its entity type and public UUID (`pid`). Serialises as the single URN
/// string `"<entity_type>:<uuid>"`.
///
/// ```
/// use entity_ref::{EntityRef, EntityType};
/// let r: EntityRef = "person:0c4f1e2a-0000-4000-8000-000000000000".parse().unwrap();
/// assert_eq!(r.entity_type, EntityType::Person);
/// assert_eq!(r.to_string(), "person:0c4f1e2a-0000-4000-8000-000000000000");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct EntityRef {
    /// The kind of record referenced.
    pub entity_type: EntityType,
    /// The record's public UUID (`pid`).
    pub id: Uuid,
}

impl EntityRef {
    /// Build a ref from its parts.
    #[must_use]
    pub const fn new(entity_type: EntityType, id: Uuid) -> Self {
        Self { entity_type, id }
    }

    /// The service that owns this reference's record.
    #[must_use]
    pub const fn service(&self) -> &'static str {
        self.entity_type.service()
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.entity_type.as_str(), self.id)
    }
}

impl FromStr for EntityRef {
    type Err = ParseEntityRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Exactly one ':' — split once and reject an empty type or a
        // remainder that itself contains ':' (UUIDs have none).
        let (type_token, id_token) = s
            .split_once(':')
            .filter(|(t, id)| !t.is_empty() && !id.is_empty() && !id.contains(':'))
            .ok_or_else(|| ParseEntityRefError::Malformed(s.to_string()))?;
        let entity_type = EntityType::from_token(type_token)
            .ok_or_else(|| ParseEntityRefError::UnknownType(type_token.to_string()))?;
        let id = Uuid::parse_str(id_token)
            .map_err(|_| ParseEntityRefError::BadId(id_token.to_string()))?;
        Ok(Self { entity_type, id })
    }
}

impl From<EntityRef> for String {
    fn from(r: EntityRef) -> Self {
        r.to_string()
    }
}

impl TryFrom<String> for EntityRef {
    type Error = ParseEntityRefError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// Sensitivity tier of an edge kind (§9, §10) — governs the authorisation
/// / audit / masking posture the aggregator must apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sensitivity {
    /// Affiliation / identity assertion (operator-asserted).
    Medium,
    /// Asserts a person is the subject of a government case (§10) —
    /// carries the case service's access-control / audit / masking rules.
    High,
}

/// The closed **v1 cross-service edge-kind registry**
/// (`cross-service-linking.md` §9). Each kind fixes its endpoint types,
/// direction, temporality, inverse label, and sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeKind {
    /// `person ↔ worker` — the same human across the two registries
    /// (symmetric; the federation backbone powering `single-view`).
    SameIdentity,
    /// `person → organization` — a person works at an organization.
    WorksAt,
    /// `person → organization` — a person is a member of an organization.
    MemberOf,
    /// `worker → organization` — a worker is employed by an organization
    /// (carries a `role`).
    EmployedBy,
    /// `case → person` — a case is about / has as its subject a person
    /// (**high** sensitivity — §10).
    SubjectOf,
}

impl EdgeKind {
    /// Every edge kind, in a stable order.
    pub const ALL: [EdgeKind; 5] = [
        EdgeKind::SameIdentity,
        EdgeKind::WorksAt,
        EdgeKind::MemberOf,
        EdgeKind::EmployedBy,
        EdgeKind::SubjectOf,
    ];

    /// The wire token for this kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            EdgeKind::SameIdentity => "same_identity",
            EdgeKind::WorksAt => "works_at",
            EdgeKind::MemberOf => "member_of",
            EdgeKind::EmployedBy => "employed_by",
            EdgeKind::SubjectOf => "subject_of",
        }
    }

    /// Parse a wire token into an [`EdgeKind`], or `None` if unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == token)
    }

    /// `true` for a symmetric kind (`same_identity`): direction is
    /// irrelevant and the aggregator canonicalises the pair.
    #[must_use]
    pub const fn is_symmetric(self) -> bool {
        matches!(self, EdgeKind::SameIdentity)
    }

    /// `true` if the edge is time-bounded (`valid_from`/`valid_to`
    /// meaningful — affiliations). `subject_of` is "sometimes"; modelled
    /// here as time-bounded-capable.
    #[must_use]
    pub const fn is_temporal(self) -> bool {
        matches!(
            self,
            EdgeKind::WorksAt | EdgeKind::MemberOf | EdgeKind::EmployedBy | EdgeKind::SubjectOf
        )
    }

    /// The inverse-direction label the aggregator stores for the far
    /// endpoint, or `None` for a symmetric kind (its own inverse).
    #[must_use]
    pub const fn inverse(self) -> Option<&'static str> {
        match self {
            EdgeKind::SameIdentity => None,
            EdgeKind::WorksAt | EdgeKind::MemberOf => Some("has_member"),
            EdgeKind::EmployedBy => Some("employs"),
            EdgeKind::SubjectOf => Some("is_subject_of"),
        }
    }

    /// This kind's sensitivity tier.
    #[must_use]
    pub const fn sensitivity(self) -> Sensitivity {
        match self {
            EdgeKind::SubjectOf => Sensitivity::High,
            _ => Sensitivity::Medium,
        }
    }

    /// Validate that `from`/`to` are the endpoint types this kind permits
    /// (§9). For the symmetric `same_identity`, either ordering of
    /// `{person, worker}` is accepted.
    #[must_use]
    pub fn permits(self, from: EntityType, to: EntityType) -> bool {
        use EntityType::{Case, Organization, Person, Worker};
        match self {
            EdgeKind::SameIdentity => {
                matches!((from, to), (Person, Worker) | (Worker, Person))
            }
            EdgeKind::WorksAt | EdgeKind::MemberOf => (from, to) == (Person, Organization),
            EdgeKind::EmployedBy => (from, to) == (Worker, Organization),
            EdgeKind::SubjectOf => (from, to) == (Case, Person),
        }
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_uuid() -> Uuid {
        Uuid::parse_str("0c4f1e2a-0000-4000-8000-000000000000").unwrap()
    }

    #[test]
    fn entity_ref_round_trips_through_its_urn() {
        let r = EntityRef::new(EntityType::Person, a_uuid());
        let urn = r.to_string();
        assert_eq!(urn, "person:0c4f1e2a-0000-4000-8000-000000000000");
        assert_eq!(urn.parse::<EntityRef>().unwrap(), r);
    }

    #[test]
    fn parses_every_entity_type_token() {
        for t in EntityType::ALL {
            let urn = format!("{}:{}", t.as_str(), a_uuid());
            assert_eq!(urn.parse::<EntityRef>().unwrap().entity_type, t);
        }
    }

    #[test]
    fn course_and_courseinstance_share_one_service() {
        assert_eq!(EntityType::Course.service(), "course-service");
        assert_eq!(EntityType::CourseInstance.service(), "course-service");
    }

    #[test]
    fn rejects_unknown_type_bad_uuid_and_malformed() {
        assert!(matches!(
            "widget:0c4f1e2a-0000-4000-8000-000000000000".parse::<EntityRef>(),
            Err(ParseEntityRefError::UnknownType(_))
        ));
        assert!(matches!(
            "person:not-a-uuid".parse::<EntityRef>(),
            Err(ParseEntityRefError::BadId(_))
        ));
        for bad in ["", "person", "person:", ":uuid", "a:b:c"] {
            assert!(
                matches!(
                    bad.parse::<EntityRef>(),
                    Err(ParseEntityRefError::Malformed(_) | ParseEntityRefError::BadId(_))
                ),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn serde_uses_the_urn_string_form() {
        let r = EntityRef::new(EntityType::Case, a_uuid());
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"case:0c4f1e2a-0000-4000-8000-000000000000\"");
        assert_eq!(serde_json::from_str::<EntityRef>(&json).unwrap(), r);
    }

    #[test]
    fn edge_kind_registry_endpoint_rules() {
        use EntityType::{Case, Organization, Person, Worker};
        // same_identity is symmetric over person/worker.
        assert!(EdgeKind::SameIdentity.permits(Person, Worker));
        assert!(EdgeKind::SameIdentity.permits(Worker, Person));
        assert!(!EdgeKind::SameIdentity.permits(Person, Person));
        // directed affiliations.
        assert!(EdgeKind::WorksAt.permits(Person, Organization));
        assert!(!EdgeKind::WorksAt.permits(Organization, Person));
        assert!(EdgeKind::EmployedBy.permits(Worker, Organization));
        assert!(EdgeKind::SubjectOf.permits(Case, Person));
        assert!(!EdgeKind::SubjectOf.permits(Person, Case));
    }

    #[test]
    fn edge_kind_metadata_matches_the_registry() {
        assert!(EdgeKind::SameIdentity.is_symmetric());
        assert!(EdgeKind::SameIdentity.inverse().is_none());
        assert!(!EdgeKind::SameIdentity.is_temporal());
        assert_eq!(EdgeKind::EmployedBy.inverse(), Some("employs"));
        assert!(EdgeKind::EmployedBy.is_temporal());
        // subject_of is the sole high-sensitivity kind.
        assert_eq!(EdgeKind::SubjectOf.sensitivity(), Sensitivity::High);
        for k in EdgeKind::ALL {
            if k != EdgeKind::SubjectOf {
                assert_eq!(k.sensitivity(), Sensitivity::Medium);
            }
        }
    }

    #[test]
    fn edge_kind_tokens_round_trip() {
        for k in EdgeKind::ALL {
            assert_eq!(EdgeKind::from_token(k.as_str()), Some(k));
        }
        assert_eq!(EdgeKind::from_token("nope"), None);
    }
}
