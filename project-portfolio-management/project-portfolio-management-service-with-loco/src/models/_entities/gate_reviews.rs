//! `SeaORM` Entity — `gate_reviews`. One phase-gate decision on a
//! plan (PPM-3); an approved review advances the item's `stage`.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "gate_reviews")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub gate: String,
    pub decision: String,
    pub conditions: Option<String>,
    pub approver_ref: Option<String>,
    pub decided_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
