//! `SeaORM` Entity — `preview_tokens`. The stored form is a **hash**;
//! the raw token exists only in the response that issued it (CMS-R22).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "preview_tokens")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    /// SHA-256 of the token. Never the token itself.
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub site_pid: Uuid,
    pub variant_pid: Uuid,
    pub revision_pid: Uuid,
    pub issued_by: Option<String>,
    pub expires_at: DateTimeWithTimeZone,
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub used_count: i32,
    pub last_used_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
