//! `audit_log` model — record reads/writes touching governed
//! `subject_of` (case↔person) edges (spec §10.4 / design §10). The case
//! service audits access to the underlying record; the aggregator audits
//! access to the link, so the two trails are consistent.

use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ConnectionTrait};
use uuid::Uuid;

pub use super::_entities::audit_log::{self, ActiveModel, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for ActiveModel {}

/// The request context stamped on a governance-audit row: the caller
/// identity and best-effort network attribution. Borrowed, so a handler
/// can build it once per request and audit several edges.
#[derive(Debug, Default, Clone, Copy)]
pub struct AuditContext<'a> {
    /// The caller's verified bearer `sub`, if any.
    pub actor: Option<&'a str>,
    /// The caller IP (best-effort; `None` in v1).
    pub user_ip: Option<&'a str>,
    /// The caller `User-Agent` header, if present.
    pub user_agent: Option<&'a str>,
}

impl Model {
    /// Record one governance-audit row for `action` on the edge
    /// `from_ref --edge_kind--> to_ref`, stamping the caller context and
    /// the current time. A fresh UUID keys the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record<C: ConnectionTrait>(
        db: &C,
        ctx: &AuditContext<'_>,
        action: &str,
        edge_kind: &str,
        from_ref: &str,
        to_ref: &str,
    ) -> ModelResult<()> {
        let am = audit_log::ActiveModel {
            id: ActiveValue::set(Uuid::new_v4()),
            actor: ActiveValue::set(ctx.actor.map(ToString::to_string)),
            action: ActiveValue::set(action.to_string()),
            edge_kind: ActiveValue::set(Some(edge_kind.to_string())),
            from_ref: ActiveValue::set(Some(from_ref.to_string())),
            to_ref: ActiveValue::set(Some(to_ref.to_string())),
            occurred_at: ActiveValue::set(Utc::now().fixed_offset()),
            user_ip: ActiveValue::set(ctx.user_ip.map(ToString::to_string)),
            user_agent: ActiveValue::set(ctx.user_agent.map(ToString::to_string)),
        };
        Entity::insert(am).exec(db).await?;
        Ok(())
    }
}
