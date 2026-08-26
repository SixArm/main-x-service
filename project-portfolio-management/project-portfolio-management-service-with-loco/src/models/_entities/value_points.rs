//! `SeaORM` Entity — `value_points`. An observed value, carrying **how** it was arrived at — a
//! measured figure and an asserted one are different evidence.
//! See entity spec §5.9.6.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "value_points")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub benefit_pid: Option<Uuid>,
    pub observed_at: DateTimeWithTimeZone,
    pub value: i64,
    pub currency: Option<String>,
    pub is_first_measurable: bool,
    pub method: String,
    pub evidence_ref: Option<String>,
    pub actor: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
