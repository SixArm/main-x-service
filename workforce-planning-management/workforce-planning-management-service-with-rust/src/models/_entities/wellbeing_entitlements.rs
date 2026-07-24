//! `SeaORM` Entity — `wellbeing_entitlements`. One configurable health-entitlement rule (WPM-R25): non-clinical predicates only (age band, departments, job titles) per WPM-D17.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "wellbeing_entitlements")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub name: String,
    pub description: String,
    pub info_url: Option<String>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    pub departments: Json,
    pub job_titles: Json,
    pub doses: i32,
    pub active_from: Option<Date>,
    pub active_until: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
