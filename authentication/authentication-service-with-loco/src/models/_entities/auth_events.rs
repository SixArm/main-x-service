//! `SeaORM` Entity — `auth_events`. One row per authentication event
//! (signup / magic-link request / redemption / signout / me) for the
//! security + compliance audit trail. Never stores tokens or secrets.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `auth_events` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "auth_events")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    pub event: String,
    pub email: Option<String>,
    pub user_pid: Option<Uuid>,
    pub detail: Option<String>,
    /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
    ///
    /// Unkeyed, so anyone holding the database can recompute it — what it
    /// catches is careless or unaware modification. Written
    /// unconditionally, unlike the MAC, which needs a key: with no key
    /// configured these two digests are the row's only integrity.
    pub hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub hash_sha3: Option<String>,
    /// HMAC-SHA256 over this row's pre-image, as
    /// `"<scheme>.<key id>:<hex>"`.
    ///
    /// Detects a row whose content was altered — notably an
    /// `attributes_assigned` row rewritten to hide a privilege grant. It
    /// does **not** detect a row deleted wholesale (see
    /// `crate::compliance`).
    pub mac: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
