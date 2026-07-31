//! `SeaORM` Entity — `assets`. One stored file, content-addressed by
//! SHA-256, with the metadata an editor needs and the alt text a
//! publish gate requires (CMS-R6–R8).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assets")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Option<Uuid>,
    pub kind: String,
    pub mime: String,
    pub byte_size: i64,
    pub checksum_sha256: String,
    pub storage_ref: String,
    pub original_filename: Option<String>,
    pub title: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub credit: Option<String>,
    pub licence: Option<String>,
    pub tags: Json,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_ms: Option<i32>,
    pub uploaded_by_ref: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
