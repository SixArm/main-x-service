//! `SeaORM` Entity — `saved_views`. A per-user saved filter/sort/
//! column preset for a front-end route (T-28l). Keyed by the token
//! `sub` and nothing else identity-shaped.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "saved_views")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub sub: String,
    pub route: String,
    pub name: String,
    pub filter: Json,
    pub sort: Json,
    pub columns: Json,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
