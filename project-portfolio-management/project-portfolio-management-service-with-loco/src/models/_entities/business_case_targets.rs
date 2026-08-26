//! `SeaORM` Entity — `business_case_targets`. The charter or gate-approved target. `approved_at` is the
//! Time-to-Value clock start and is never updated.
//! See entity spec §5.9.6.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "business_case_targets")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub metric: String,
    pub baseline_value: i64,
    pub target_value: i64,
    pub unit: Option<String>,
    pub currency: Option<String>,
    pub promised_by: Option<Date>,
    pub source: String,
    pub approved_at: DateTimeWithTimeZone,
    pub approved_by_ref: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
