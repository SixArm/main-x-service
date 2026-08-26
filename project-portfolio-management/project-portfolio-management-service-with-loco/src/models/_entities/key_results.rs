//! `SeaORM` Entity — `key_results`. The measurable half of an OKR.
//! See entity spec §5.9.2 / FR-27.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "key_results")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub objective_pid: Uuid,
    pub title: String,
    pub metric: String,
    pub direction: String,
    /// The baseline, captured once. **Never updated** — progress
    /// measured from a moving baseline is not progress.
    pub start_value: i64,
    pub target_value: i64,
    pub current_value: i64,
    pub tolerance: Option<i64>,
    pub unit: Option<String>,
    pub currency: Option<String>,
    pub owner_ref: Option<String>,
    pub due_date: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
