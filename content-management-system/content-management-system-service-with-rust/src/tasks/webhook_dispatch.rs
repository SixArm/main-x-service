//! `task webhook_dispatch` — deliver due outbound webhooks (CMS-R23).
//!
//! The CLI surface for the same function `POST /api/webhooks/dispatch`
//! calls, so a system scheduler can drive it without an HTTP round
//! trip. Reruns are safe: a delivered or abandoned event is never
//! re-sent, and a failed one waits out its backoff.
//!
//! Under the default in-memory event transport there is no durable
//! record to deliver from, so the task **says so and does nothing**
//! rather than delivering a subset that disappears on restart.

use loco_rs::prelude::*;

/// The dispatch task.
pub struct WebhookDispatch;

#[async_trait]
impl Task for WebhookDispatch {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "webhook_dispatch".to_string(),
            detail: "Deliver due outbound webhooks from the event outbox".to_string(),
        }
    }

    /// Run one dispatch pass.
    ///
    /// # Errors
    ///
    /// When a query or write fails. A failed *delivery* is recorded,
    /// not returned: one broken receiver must not stop the others.
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        if !crate::streaming::transport().is_outbox() {
            tracing::warn!(
                "webhook dispatch needs CMS_EVENT_TRANSPORT=outbox; with the in-memory \
                 transport there is no durable event record to deliver from, so nothing \
                 was sent"
            );
            return Ok(());
        }
        let outcomes = crate::controllers::webhooks::run_dispatch(&ctx.db).await?;
        let delivered = outcomes.iter().filter(|o| o.state == "delivered").count();
        let abandoned = outcomes.iter().filter(|o| o.state == "abandoned").count();
        for outcome in outcomes.iter().filter(|o| o.state != "delivered") {
            // A receiver that is not getting its events is what an
            // operator needs to hear about.
            tracing::warn!(
                webhook = %outcome.webhook_pid,
                event = %outcome.event_id,
                state = outcome.state,
                status = outcome.status,
                "webhook delivery did not succeed"
            );
        }
        tracing::info!(
            attempted = outcomes.len(),
            delivered,
            abandoned,
            "webhook dispatch complete"
        );
        Ok(())
    }
}
