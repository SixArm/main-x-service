//! `task schedule_sweep` — apply every due publish/unpublish schedule
//! (CMS-R12).
//!
//! The CLI surface for the same function the `POST /api/schedules/sweep`
//! endpoint calls, so a system scheduler (cron, a Kubernetes `CronJob`,
//! loco's own scheduler) can drive it without an HTTP round trip. The
//! sweep is idempotent, so running it more often than necessary is
//! harmless and running it twice at once is safe: each variant is
//! locked and its due field cleared in the same transaction.

use loco_rs::prelude::*;

/// The sweep task.
pub struct ScheduleSweep;

#[async_trait]
impl Task for ScheduleSweep {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "schedule_sweep".to_string(),
            detail: "Apply due scheduled publishes and unpublishes".to_string(),
        }
    }

    /// Run one sweep, logging what it applied and what it skipped.
    ///
    /// # Errors
    ///
    /// When a query or write fails.
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let outcomes = crate::controllers::workflow::run_sweep(&ctx.db, None).await?;
        let applied = outcomes.iter().filter(|o| o.outcome != "skipped").count();
        let skipped = outcomes.len() - applied;
        for outcome in outcomes.iter().filter(|o| o.outcome == "skipped") {
            // A skipped schedule is the case an operator needs to hear
            // about: their page did not go live, and this says why.
            tracing::warn!(
                variant = %outcome.variant_pid,
                detail = outcome.detail.as_deref().unwrap_or(""),
                "scheduled transition skipped"
            );
        }
        tracing::info!(applied, skipped, "schedule sweep complete");
        Ok(())
    }
}
