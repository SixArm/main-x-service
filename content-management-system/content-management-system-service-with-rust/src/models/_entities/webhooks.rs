//! `SeaORM` Entity — `webhooks`. The only extension mechanism
//! (CMS-D12): a declared outbound subscription.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhooks")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub name: String,
    pub url: String,
    pub event_kinds: Json,
    /// The shared secret. **Never serialized**: it is returned once, by
    /// the registration response, and by nothing else.
    #[serde(skip_serializing)]
    pub secret: String,
    pub active: bool,
    pub last_delivered_at: Option<DateTimeWithTimeZone>,
    pub consecutive_failures: i32,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
