//! `SeaORM` Entity — `budget_baselines`. The **append-only**, versioned
//! phased budget baseline (T-28b). See
//! `migration/src/m20260918_000001_budget_baselines.rs` for the schema
//! rationale.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_baselines")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub version: i32,
    pub currency: String,
    pub approved_at: DateTimeWithTimeZone,
    /// Required for `version > 1`: a re-baseline with no reason is
    /// indistinguishable from a mistake.
    pub reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
