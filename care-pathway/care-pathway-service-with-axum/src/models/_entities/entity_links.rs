//! `SeaORM` Entity — `entity_links`. The **write side** of cross-service
//! entity linking (`agents/share/cross-service-linking.md` §4.1): one
//! **outbound** edge this service originates from a care-pathway
//! *instance* (v1: the `continues_as` journey edge).
//!
//! Separate from the within-entity `relationships` on the `CarePathway`
//! payload — a cross-service edge is **never** a matcher signal (the
//! partition rule, §7). A journey continuing into an inpatient stay says
//! nothing about whether two pathway *templates* are the same document.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `entity_links` table.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One persisted outbound cross-service edge.
///
/// `Eq` is intentionally not derived: `confidence` is an `f64`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "entity_links")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    /// The edge id (also the `linked`/`unlinked` event's `edge_id`).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// The originating **pathway instance** pid — a journey belongs to an
    /// enrolment, not to the template.
    pub from_pid: Uuid,
    /// The edge kind token (§9 registry), e.g. `continues_as`.
    pub kind: String,
    /// The far record's `EntityRef` URN, e.g. `patient_flow_stay:<uuid>`.
    pub to_ref: String,
    /// Optional role label.
    pub role: Option<String>,
    /// Optional confidence: `1.0` operator-asserted, `<1` suggested.
    pub confidence: Option<f64>,
    /// How the edge arose: `operator` | `import` | `matcher_suggested`.
    pub provenance: String,
    /// When the continuation began.
    pub valid_from: Option<Date>,
    /// When it ended.
    pub valid_to: Option<Date>,
    /// Soft-delete timestamp (a withdrawn edge); `None` while live.
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
