//! `SeaORM` Entity — `succession_plans`. One critical role's succession plan (WPM-R12).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "succession_plans")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub role_title: String,
    pub department: String,
    pub criticality: i32,
    pub incumbent_pid: Option<Uuid>,
    /// How likely the incumbent is to leave: `low` | `medium` | `high`.
    pub risk_of_loss: Option<String>,
    /// When the role is expected to fall vacant, when that is known.
    pub vacancy_expected_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
