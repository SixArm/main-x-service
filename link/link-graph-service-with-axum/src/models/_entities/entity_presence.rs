//! `SeaORM` Entity — `entity_presence`. The existence oracle
//! (spec §10.2): one row per known `EntityRef`, `alive` toggled by the
//! source service's `created` / `deleted` events.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted presence row. The primary key column is `ref` (a SQL /
/// Rust-keyword clash), mapped from the `entity_ref` field.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "entity_presence")]
pub struct Model {
    /// The `EntityRef` URN (column `ref`).
    #[sea_orm(primary_key, auto_increment = false, column_name = "ref")]
    pub entity_ref: String,
    /// `true` when last seen `created`; `false` when `deleted`.
    pub alive: bool,
    /// The last envelope `seq` observed for this ref (ordering).
    pub last_seq: i64,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
