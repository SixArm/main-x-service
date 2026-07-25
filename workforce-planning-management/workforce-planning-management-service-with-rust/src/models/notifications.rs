//! `notifications` model — push and list in-app notifications
//! (WPM-R31 / WPM-D23). Reference-only by contract: a kind, a neutral
//! body, and pids/names — callers must never pass scores, comments,
//! or any masked-tier value into `body`/`data`.

use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

pub use super::_entities::notifications::{self, ActiveModel, Entity, Model};

impl Model {
    /// Push one notification to an employee. Generic over
    /// [`ConnectionTrait`] so it can ride the handler's transaction.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn push<C: ConnectionTrait>(
        db: &C,
        employee_pid: Uuid,
        kind: &str,
        body: &str,
        data: serde_json::Value,
    ) -> ModelResult<Self> {
        let entry = notifications::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            employee_pid: ActiveValue::set(employee_pid),
            kind: ActiveValue::set(kind.to_string()),
            body: ActiveValue::set(body.to_string()),
            data: ActiveValue::set(data),
            read_at: ActiveValue::set(None),
            ..Default::default()
        };
        Ok(entry.insert(db).await?)
    }
}
