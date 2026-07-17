//! `SeaORM` Entity — `beds`. One physical (or virtual-slot) bed with
//! its live state-machine columns (spec `bed-management.md`).

#![allow(missing_docs)]
#![allow(clippy::struct_excessive_bools)] // bed attribute columns are independent flags

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "beds")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub bay_pid: Uuid,
    pub number: String,
    pub state: String,
    pub state_since: DateTimeWithTimeZone,
    pub closure_reason: Option<String>,
    pub deep_clean_required: bool,
    pub isolation_capable: bool,
    pub oxygen: bool,
    pub bariatric: bool,
    #[sea_orm(column_name = "virtual")]
    pub is_virtual: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
