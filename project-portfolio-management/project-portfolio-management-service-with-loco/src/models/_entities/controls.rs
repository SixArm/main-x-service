//! `SeaORM` Entity — `controls`. The Controlling-process register.
//! See entity spec §5.9.8 / FR-38.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "controls")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub name: String,
    /// `feedforward` | `concurrent` | `feedback` — decides what a
    /// failing reading may do.
    pub timing: String,
    pub metric: String,
    pub target_value: i64,
    pub comparator: String,
    pub tolerance: Option<i64>,
    pub unit: Option<String>,
    pub currency: Option<String>,
    pub source_kind: String,
    pub source_ref: Option<String>,
    /// `None` means no cadence is declared, so a reading can never be
    /// *overdue*.
    pub cadence_days: Option<i64>,
    pub owner_ref: Option<String>,
    pub enabled: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
