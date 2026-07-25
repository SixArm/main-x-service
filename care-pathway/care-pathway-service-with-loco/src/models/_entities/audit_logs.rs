//! `SeaORM` Entity — `audit_logs`. One row per CRUD action on a
//! care pathway (who / what / when + a snapshot).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `audit_logs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted audit-log row: one CRUD/merge action on a care pathway.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    /// Row creation timestamp (when the action was recorded).
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp (audit rows are append-only in practice).
    pub updated_at: DateTimeWithTimeZone,
    /// Internal auto-increment primary key.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The care-pathway `pid` this entry concerns.
    pub entity_pid: Uuid,
    /// The action verb: `created` / `updated` / `deleted` / `merged` / …
    pub action: String,
    /// The acting user's `sub` (token `pid`), or `None` when unauthenticated.
    pub actor: Option<String>,
    /// Post-change payload snapshot (JSON), or `None` for deletes.
    pub snapshot: Option<Json>,
    /// Hash of the preceding chain row; `None` for the genesis row and for
    /// rows written before the chain existed (verification reports those as
    /// `unchained`, not as a break).
    pub prev_hash: Option<String>,
    /// This row's content hash — the link every successor binds to. See
    /// [`crate::compliance::audit_chain`].
    pub hash: Option<String>,
    /// Request/processing context (purpose-of-use, residency, lawful basis,
    /// disclosure recipient), as JSON.
    pub context: Option<Json>,
    /// Whether this access was an outward **disclosure** rather than an
    /// internal access — the HIPAA §164.528 accounting distinction.
    pub disclosure: bool,
    /// When the row's content was destroyed under GDPR Art. 17. A redacted
    /// row keeps its stored `hash` and `prev_hash`, so the chain stays
    /// verifiable and still proves the event occurred.
    pub redacted_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
