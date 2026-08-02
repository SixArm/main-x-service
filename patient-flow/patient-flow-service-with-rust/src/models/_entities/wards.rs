//! `SeaORM` Entity — `wards`. A ward within a site; `kind` is
//! `inpatient` / `assessment` / `virtual` (spec `domain-model.md`).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "wards")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub name: String,
    pub code: String,
    pub kind: String,
    pub specialty: Option<String>,
    pub open: bool,
    pub escalation: bool,
    pub closed_to_admissions: bool,
    pub place_ref: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
