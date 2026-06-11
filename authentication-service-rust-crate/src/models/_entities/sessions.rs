//! `SeaORM` Entity — `sessions`. Tracks issued access tokens by `jid`
//! (the JWT `jti`) so the auth service can revoke them on signout.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    /// JWT id (`jti`) of the issued token.
    #[sea_orm(unique)]
    pub jid: String,
    /// Owning user's `pid`.
    pub user_pid: Uuid,
    /// Token expiry.
    pub expires_at: DateTimeWithTimeZone,
    /// Set when the session is revoked via signout.
    pub revoked_at: Option<DateTimeWithTimeZone>,
    /// Best-effort user agent captured at issuance.
    pub user_agent: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
