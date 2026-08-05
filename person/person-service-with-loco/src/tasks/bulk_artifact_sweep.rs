//! `bulk_artifact_sweep` task — physically delete the artifacts of bulk
//! jobs past their retention deadline (SEC-B4 follow-up).
//!
//! The `expires_at` deadline on a `bulk_jobs` row already stops the
//! status endpoint handing out an expired job's download/error-report
//! reference (`bulk::handlers::artifact_expired`). It does not delete
//! the underlying bytes from the [`crate::bulk::store::ArtifactStore`] —
//! they sit there indefinitely unless something physically removes
//! them. This task is that something: it walks
//! [`crate::bulk::sweep::sweep`] over the jobs past their deadline that
//! have not yet been swept, deletes each one's artifacts, and stamps
//! `artifact_deleted_at` so a swept row is never processed twice.
//!
//! ```text
//! # report how many jobs qualify, without deleting anything
//! cargo loco task bulk_artifact_sweep
//! cargo loco task bulk_artifact_sweep op:report
//!
//! # actually sweep (up to the row cap for this pass)
//! cargo loco task bulk_artifact_sweep op:apply
//! cargo loco task bulk_artifact_sweep op:apply limit:500
//! ```
//!
//! ## Scheduling
//!
//! This crate has no in-process periodic-timer convention — the
//! `BulkJobWorker` ([`crate::bulk::worker`]) is dispatched **per job**
//! via `perform_later` when a job is submitted, never on a clock, so
//! there is nothing already running on a schedule to piggyback on.
//! Rather than invent a new in-process scheduling primitive for this one
//! maintenance task, it is a loco `Task` — the same shape this crate
//! already uses for other operator-triggered maintenance
//! ([`crate::tasks::integrity_resign`]) — meant to be invoked by an
//! external scheduler (cron, a Kubernetes `CronJob`, a Podman `systemd`
//! timer unit, …) running `cargo loco task bulk_artifact_sweep
//! op:apply` on whatever cadence a deployment's retention policy wants
//! (daily is more than sufficient against the 7-day
//! [`crate::bulk::BULK_ARTIFACT_TTL_SECS`] window).

use loco_rs::prelude::*;

use crate::bulk::sweep::{self, SweepOutcome};

/// Default row cap per pass, matching the order of magnitude of
/// [`crate::tasks::integrity_resign::DEFAULT_LIMIT`] — generous for one
/// scheduled run without risking an unbounded single pass.
pub const DEFAULT_LIMIT: u64 = 10_000;

/// What the operator asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepCommand {
    /// `true` (`op:apply`) actually deletes; `false` (default /
    /// `op:report`) only reports how many jobs currently qualify.
    pub apply: bool,
    /// Row cap for this pass.
    pub limit: u64,
}

/// Parse the `key:value` argument style loco tasks use.
///
/// Defaults to a **report-only** pass (`apply: false`) because a task
/// that physically deletes data should never be the default of running
/// it with no arguments — the operator must ask for `op:apply`
/// explicitly, mirroring `integrity_resign`'s dry-run-by-default posture.
///
/// # Errors
///
/// When `op:` names something other than `report`/`apply`, or `limit:`
/// does not parse as a non-negative integer.
pub fn parse(vars: &std::collections::BTreeMap<String, String>) -> Result<SweepCommand, String> {
    let apply = match vars.get("op").map_or("report", String::as_str) {
        "report" | "dry_run" | "dry-run" => false,
        "apply" => true,
        other => return Err(format!("unknown op:{other} — expected report or apply")),
    };
    let limit = match vars.get("limit") {
        None => DEFAULT_LIMIT,
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| format!("limit:{s} is not a non-negative integer"))?,
    };
    Ok(SweepCommand { apply, limit })
}

/// The `bulk_artifact_sweep` CLI task.
pub struct BulkArtifactSweep;

#[async_trait]
impl Task for BulkArtifactSweep {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "bulk_artifact_sweep".to_string(),
            detail: "Physically delete artifacts of expired bulk jobs (SEC-B4 follow-up)"
                .to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let command = parse(&vars.cli).map_err(|e| Error::string(&e))?;

        if !command.apply {
            let now = time::OffsetDateTime::now_utc();
            let candidates =
                crate::db::bulk_jobs::list_artifact_sweep_candidates(&ctx.db, now, command.limit)
                    .await
                    .map_err(|e| Error::string(&e.to_string()))?;
            println!(
                "{} job(s) past their retention deadline have not yet been swept (re-run with \
                 op:apply to delete their artifacts)",
                candidates.len()
            );
            for job in &candidates {
                let deadline = job
                    .expires_at
                    .map_or_else(|| "?".to_string(), |t| t.to_string());
                println!("  {} ({}) expired {deadline}", job.id, job.kind);
            }
            return Ok(());
        }

        let store = crate::bulk::store::from_env()
            .await
            .map_err(|e| Error::string(&e.to_string()))?;
        let SweepOutcome {
            jobs_examined,
            jobs_swept,
            jobs_failed,
        } = sweep::sweep(&ctx.db, store.as_ref(), command.limit)
            .await
            .map_err(|e| Error::string(&e.to_string()))?;
        println!(
            "swept {jobs_swept}/{jobs_examined} job(s); {jobs_failed} will be retried on the \
             next pass"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SweepCommand, parse};

    fn vars(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// A physically-destructive task must not run destructively by
    /// default — no arguments means report-only.
    #[test]
    fn no_arguments_means_report_only() {
        assert_eq!(
            parse(&vars(&[])).expect("parses"),
            SweepCommand {
                apply: false,
                limit: super::DEFAULT_LIMIT
            }
        );
    }

    /// `op:apply` actually enables deletion; `op:report` (and its
    /// dry-run aliases) stay report-only; an unknown op is rejected.
    #[test]
    fn op_selects_apply_or_report() {
        assert_eq!(
            parse(&vars(&[("op", "apply")])).expect("parses"),
            SweepCommand {
                apply: true,
                limit: super::DEFAULT_LIMIT
            }
        );
        for report_alias in ["report", "dry_run", "dry-run"] {
            assert_eq!(
                parse(&vars(&[("op", report_alias)])).expect("parses"),
                SweepCommand {
                    apply: false,
                    limit: super::DEFAULT_LIMIT
                },
                "{report_alias} must stay report-only"
            );
        }
        assert!(parse(&vars(&[("op", "delete-everything")])).is_err());
    }

    /// `limit:` overrides the default row cap; a non-numeric value is
    /// rejected rather than silently falling back.
    #[test]
    fn limit_overrides_the_default_and_rejects_garbage() {
        assert_eq!(
            parse(&vars(&[("op", "apply"), ("limit", "500")]))
                .expect("parses")
                .limit,
            500
        );
        assert!(parse(&vars(&[("op", "apply"), ("limit", "nope")])).is_err());
    }
}
