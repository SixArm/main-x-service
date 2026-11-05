//! `SeaORM` Entity — `entity_links`. The **write side** of cross-service
//! entity linking (`agents/share/cross-service-linking.md` §4.1): one
//! **outbound** edge this service originates from a case (v1: the
//! `subject_of` case → person edge). Separate from the within-entity
//! `relationships` on the `Case` payload — these are never a matcher
//! signal (the partition rule, §7).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `entity_links` table.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One persisted outbound cross-service edge.
///
/// `Eq` is intentionally not derived: `confidence` is an `f64` (not `Eq`).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "entity_links")]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp (set when upserted / soft-deleted).
    pub updated_at: DateTimeWithTimeZone,
    /// The edge id (also the `linked`/`unlinked` event's `edge_id`).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// This service's originating case record (`pid`).
    pub from_pid: Uuid,
    /// The edge kind token (§9 registry), e.g. `subject_of`.
    pub kind: String,
    /// The far record's `EntityRef` URN, e.g. `person:<uuid>`.
    pub to_ref: String,
    /// Optional role label (unused by `subject_of`).
    pub role: Option<String>,
    /// Optional confidence: `1.0` operator-asserted, `<1` suggested.
    pub confidence: Option<f64>,
    /// How the edge arose: `operator` | `import` | `matcher_suggested`.
    pub provenance: String,
    /// Affiliation start (nullable — `subject_of` is temporal).
    pub valid_from: Option<Date>,
    /// Affiliation end ("former subject of" once past).
    pub valid_to: Option<Date>,
    /// Soft-delete timestamp (a withdrawn edge); `None` while live.
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
