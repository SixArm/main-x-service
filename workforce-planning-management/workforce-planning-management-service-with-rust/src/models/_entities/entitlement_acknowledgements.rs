//! `SeaORM` Entity — `entitlement_acknowledgements`. One employee's response to a wellbeing prompt (WPM-R25): an HR workflow fact (`booked | done | declined | dismissed`), never a vaccination status (WPM-D17).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "entitlement_acknowledgements")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub entitlement_pid: Uuid,
    pub employee_pid: Uuid,
    pub response: String,
    pub responded_on: Date,
    pub reminded_on: Option<Date>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
