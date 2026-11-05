//! `SeaORM` Entity — `consumer_offsets`. Per-topic bus position plus the
//! freshness watermark backing `as_of` (spec §10.3).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted per-topic offset + freshness watermark row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "consumer_offsets")]
pub struct Model {
    /// The bus topic (`mxi.<entity>.events`).
    #[sea_orm(primary_key, auto_increment = false)]
    pub topic: String,
    /// The last committed bus offset for this topic.
    pub offset_val: i64,
    /// The `occurred_at` of the last consumed event on this topic.
    pub last_occurred_at: DateTimeWithTimeZone,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
