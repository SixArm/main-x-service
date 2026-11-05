//! `SeaORM` Entity — `audit_logs`. One row per CRUD action on a
//! case (who / what / when + a snapshot).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `audit_logs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    pub entity_pid: Uuid,
    pub action: String,
    pub actor: Option<String>,
    pub snapshot: Option<Json>,
    /// Hash of the preceding chain row; `None` for the genesis row and for
    /// rows written before the chain existed.
    pub prev_hash: Option<String>,
    /// SHA-3 digest of the preceding chain row.
    pub prev_hash_sha3: Option<String>,
    /// This row's content hash — the link every successor binds to.
    pub hash: Option<String>,
    /// This row's SHA-3 digest — the third parallel chain's link.
    pub hash_sha3: Option<String>,
    /// HMAC over this row's pre-image, as `"<key id>:<hex>"`.
    pub mac: Option<String>,
    /// Request/processing context (purpose-of-use, disclosure recipient).
    pub context: Option<Json>,
    /// Whether this access was an outward **disclosure** rather than an
    /// internal access — the HIPAA §164.528 accounting distinction.
    pub disclosure: bool,
    /// When the row's content was destroyed under GDPR Art. 17.
    pub redacted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
