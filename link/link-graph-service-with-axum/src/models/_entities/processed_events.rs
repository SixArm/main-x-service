//! `SeaORM` Entity — `processed_events`. Bus-consumer idempotency under
//! at-least-once delivery (spec §10.3; BUS-2).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A recorded envelope `event_id` that has already been applied to the
/// read-model.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "processed_events")]
pub struct Model {
    /// The envelope `event_id` — the idempotency key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_id: Uuid,
    /// When this event was recorded as processed.
    pub processed_at: DateTimeWithTimeZone,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
