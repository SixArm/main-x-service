//! `SeaORM` Entity — `edges`. The bidirectional, queryable graph
//! (spec §10.1). Refs are stored as their `EntityRef` URN strings; a
//! symmetric edge is stored once with the canonical (smaller) ref as
//! `from_ref`.

// SeaORM-generated entity: the field-level shape is documented by the
// migration SQL, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted graph edge. Note: no `Eq`/`Hash` derive because
/// `confidence` is an `Option<f64>`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "edges")]
pub struct Model {
    /// The edge id — equals the source `linked` event's `edge_id`.
    #[sea_orm(primary_key, auto_increment = false)]
    pub edge_id: Uuid,
    /// Canonical "from" endpoint (`EntityRef` URN).
    pub from_ref: String,
    /// "to" endpoint (`EntityRef` URN).
    pub to_ref: String,
    /// Edge kind wire token (closed registry).
    pub kind: String,
    /// `false` for symmetric kinds.
    pub directed: bool,
    /// Optional role (e.g. job title for `employed_by`).
    pub role: Option<String>,
    /// Confidence in `[0,1]`; `1.0` operator-asserted.
    pub confidence: Option<f64>,
    /// Provenance wire token (`operator` | `import` | `matcher_suggested`).
    pub provenance: String,
    /// Affiliation start (nullable).
    pub valid_from: Option<Date>,
    /// Affiliation end (nullable).
    pub valid_to: Option<Date>,
    /// Integrity-lifecycle status wire token.
    pub status: String,
    /// When the `linked` event was consumed.
    pub observed_at: DateTimeWithTimeZone,
    /// The envelope `event_id` that produced this row (dedup provenance).
    pub source_event_id: Uuid,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
