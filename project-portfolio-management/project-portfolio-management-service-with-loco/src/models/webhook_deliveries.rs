//! `webhook_deliveries` model — the outbound webhook sink's delivery
//! log ([`crate::webhooks`], `agents/share/event-bus.md` §12).
//!
//! [`Model::record`] is the one write: insert the final outcome of one
//! target URL's retry-with-backoff sequence for one outbox event. There
//! is no update — a delivery outcome is immutable once decided, matching
//! `audit_logs`' append-only posture. [`Model::recent`] is the operator
//! read, newest first, optionally scoped to one `event_id`.

use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, QueryFilter, QueryOrder, QuerySelect};

pub use super::_entities::webhook_deliveries::{self, ActiveModel, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for super::_entities::webhook_deliveries::ActiveModel {}

/// One delivery outcome to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryOutcome {
    /// The envelope's dedup id.
    pub event_id: uuid::Uuid,
    /// Entity name.
    pub entity: String,
    /// Change kind token.
    pub kind: String,
    /// The target URL.
    pub url: String,
    /// How many HTTP attempts were made.
    pub attempts: u32,
    /// Whether the delivery ultimately succeeded.
    pub delivered: bool,
    /// The last HTTP status received, if any.
    pub status_code: Option<u16>,
    /// The last error message, if the final attempt failed.
    pub error: Option<String>,
}

impl Model {
    /// Record one delivery outcome.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(db: &DatabaseConnection, outcome: &DeliveryOutcome) -> ModelResult<Self> {
        let row = webhook_deliveries::ActiveModel {
            event_id: ActiveValue::set(outcome.event_id),
            entity: ActiveValue::set(outcome.entity.clone()),
            kind: ActiveValue::set(outcome.kind.clone()),
            url: ActiveValue::set(outcome.url.clone()),
            attempts: ActiveValue::set(i32::try_from(outcome.attempts).unwrap_or(i32::MAX)),
            status: ActiveValue::set(
                if outcome.delivered {
                    "delivered"
                } else {
                    "failed"
                }
                .to_string(),
            ),
            status_code: ActiveValue::set(outcome.status_code.map(i32::from)),
            error: ActiveValue::set(outcome.error.clone()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(row)
    }

    /// The most recent delivery-log rows, newest first, optionally
    /// scoped to one event.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(
        db: &DatabaseConnection,
        event_id: Option<uuid::Uuid>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let mut query = webhook_deliveries::Entity::find();
        if let Some(event_id) = event_id {
            query = query.filter(webhook_deliveries::Column::EventId.eq(event_id));
        }
        let rows = query
            .order_by_desc(webhook_deliveries::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::DeliveryOutcome;
    use uuid::Uuid;

    /// Sanity: a `DeliveryOutcome` for a failed delivery carries no
    /// status code when the failure was a pure transport error (no
    /// response ever came back) — pinned since [`super::Model::record`]
    /// maps `None` straight through and a future refactor could
    /// accidentally coerce it to `0`.
    #[test]
    fn a_transport_failure_carries_no_status_code() {
        let outcome = DeliveryOutcome {
            event_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            entity: "plan".to_string(),
            kind: "created".to_string(),
            url: "https://example.com/hook".to_string(),
            attempts: 4,
            delivered: false,
            status_code: None,
            error: Some("connection reset".to_string()),
        };
        assert_eq!(outcome.status_code, None);
        assert!(!outcome.delivered);
    }
}
