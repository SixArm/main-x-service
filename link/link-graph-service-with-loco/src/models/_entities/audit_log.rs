//! `SeaORM` Entity — `audit_log`. One row per read/write touching a
//! governed `subject_of` (case↔person) edge (spec §10.4 / design §10).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted governance-audit row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_log")]
pub struct Model {
    /// Row id (UUID v4, generated at record time).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// The caller's bearer `sub`, or `None` for a bus-driven write.
    pub actor: Option<String>,
    /// `read_edge` | `read_single_view` | `apply_linked`.
    pub action: String,
    /// The governed edge's kind token.
    pub edge_kind: Option<String>,
    /// The governed edge's `from` endpoint URN.
    pub from_ref: Option<String>,
    /// The governed edge's `to` endpoint URN.
    pub to_ref: Option<String>,
    /// When the audited action occurred.
    pub occurred_at: DateTimeWithTimeZone,
    /// Caller IP (best-effort; `None` in v1).
    pub user_ip: Option<String>,
    /// Caller `User-Agent` (best-effort).
    pub user_agent: Option<String>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
