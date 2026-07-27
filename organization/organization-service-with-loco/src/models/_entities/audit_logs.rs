//! `SeaORM` Entity — `audit_logs`. One row per CRUD action on an
//! organization (who / what / when + a snapshot).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `audit_logs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One audit-trail row (`SeaORM`-generated). Mirrors the `audit_logs`
/// table from `m20220101_000002_audit_logs`; written best-effort on
/// every CRUD action so a failed audit never fails the request.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    /// Row insert time (UTC) — effectively the "when" of the action.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-modification time (UTC); audit rows are append-only so
    /// this normally equals `created_at`.
    pub updated_at: DateTimeWithTimeZone,
    /// Auto-increment surrogate primary key; also the recency order key.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The organization `pid` this entry concerns (the "what").
    pub entity_pid: Uuid,
    /// The action verb: `created` / `updated` / `deleted` / `merged` /
    /// `merged_into`.
    pub action: String,
    /// The actor that caused the change — the bearer token `sub` (user
    /// `pid`) when one was presented, else `None` (the "who").
    pub actor: Option<String>,
    /// Optional JSON snapshot of the record's payload at action time
    /// (`None` for deletes, where there is nothing to snapshot).
    pub snapshot: Option<Json>,
    /// HMAC-SHA256 over this audit row's pre-image, as
    /// `"<scheme>.<key id>:<hex>"`.
    ///
    /// Detects an audit row whose *content* was edited. It does not
    /// detect a row deleted wholesale — that needs the hash chain this
    /// service does not yet have (see `crate::compliance`).
    pub mac: Option<String>,
}

/// `SeaORM` relation enum for `audit_logs`. Audit rows reference an
/// organization only by `entity_pid` value (no FK), so no relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
