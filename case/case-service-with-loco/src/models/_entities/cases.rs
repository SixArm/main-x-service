//! `SeaORM` Entity — `cases`. The full `case_matcher::Case` payload is
//! stored in `data` (JSONB); `pid` and `title` are denormalised for
//! lookup and listing.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted case row.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "cases")]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp.
    pub updated_at: DateTimeWithTimeZone,
    /// Internal auto-increment primary key.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Public, externally exposed identifier (UUID v4).
    #[sea_orm(unique)]
    pub pid: Uuid,
    /// Denormalised case title, for lookup and listing.
    pub title: String,
    /// The full `case_matcher::Case` payload (JSONB).
    pub data: Json,
    /// Whether the row is active (cleared on soft-delete).
    pub active: bool,
    /// Soft-delete timestamp; `None` while active.
    pub deleted_at: Option<DateTimeWithTimeZone>,
    /// SHA-256 over the record's content and lifecycle state, for
    /// out-of-band tamper detection (`crate::compliance::record_integrity`).
    /// `None` on rows written before the column existed; never
    /// back-filled, because a back-fill would certify whatever the current
    /// content happens to be — the claim the hash exists to test.
    pub content_hash: Option<String>,
    /// BLAKE3 digest over the same pre-image as `content_hash`. `None` on
    /// rows written before the second algorithm was adopted.
    pub content_hash_blake3: Option<String>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
