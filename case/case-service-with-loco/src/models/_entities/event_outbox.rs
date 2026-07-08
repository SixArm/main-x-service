//! `SeaORM` Entity — `event_outbox`. The transactional-outbox hand-off
//! buffer for the durable event bus (Phase 2; one row per published
//! event, written in the entity mutation's transaction and drained by a
//! relay worker).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `event_outbox` table.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One persisted outbox row: a canonical envelope awaiting relay to the
/// durable bus.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "event_outbox")]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp (set when marked published).
    pub updated_at: DateTimeWithTimeZone,
    /// Auto-increment pk; also the global relay order.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Envelope id — the consumer dedup key.
    #[sea_orm(unique)]
    pub event_id: Uuid,
    /// The entity name (e.g. `case`).
    pub entity: String,
    /// The record pid — the bus partition key.
    pub entity_pid: Uuid,
    /// The change kind: `created` / `updated` / `deleted` / `merged`.
    pub kind: String,
    /// When the change occurred (stamped at enqueue).
    pub occurred_at: DateTimeWithTimeZone,
    /// The acting user's pid, or `None` when unauthenticated.
    pub actor: Option<String>,
    /// The envelope schema version.
    pub schema_version: i32,
    /// The full canonical envelope as JSONB.
    pub payload: Json,
    /// `None` until the relay ships the row to the bus.
    pub published_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
