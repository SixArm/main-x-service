//! `SeaORM` Entity — `sites`. One delivery namespace: its locales,
//! fallback chains, and the `visibility` that decides whether its
//! published delivery reads are on the anonymous allow-list (CMS-R1).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sites")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub key: String,
    pub name: String,
    pub owner_ref: Option<String>,
    pub default_locale: String,
    pub locales: Json,
    pub fallback_chains: Json,
    pub strict_locales: Json,
    pub visibility: String,
    pub base_url: Option<String>,
    pub robots_default: String,
    pub require_distinct_approver: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
