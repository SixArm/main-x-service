//! `SeaORM` Entity — `phase_transitions`. The **append-only** project
//! phase log, from which per-phase durations are derived. See entity
//! spec §5.9.4 / FR-30.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "phase_transitions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    /// `None` marks the plan's first phase.
    pub from_phase: Option<String>,
    pub to_phase: String,
    pub occurred_at: DateTimeWithTimeZone,
    pub actor: Option<String>,
    /// Required for a backward move: re-planning is normal, an
    /// unexplained regression is not.
    pub reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
