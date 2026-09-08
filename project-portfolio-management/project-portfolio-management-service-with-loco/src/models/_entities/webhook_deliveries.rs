//! `SeaORM` Entity — `webhook_deliveries`. The delivery log for the
//! outbound webhook sink: one row per target URL's final delivery
//! outcome for one outbox event.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `webhook_deliveries` table.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One recorded webhook delivery outcome.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_deliveries")]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Auto-increment pk.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The envelope's dedup id, so this row is traceable to its
    /// `event_outbox` row.
    pub event_id: Uuid,
    /// The entity name (e.g. `plan`).
    pub entity: String,
    /// The change kind: `created` / `updated` / `deleted` / `merged`.
    pub kind: String,
    /// The configured target URL this row is about.
    pub url: String,
    /// How many HTTP attempts this delivery took.
    pub attempts: i32,
    /// `delivered` or `failed` (the final outcome only).
    pub status: String,
    /// The last HTTP status received, when a response came back at all.
    pub status_code: Option<i32>,
    /// The last error message, when the final attempt failed.
    pub error: Option<String>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
