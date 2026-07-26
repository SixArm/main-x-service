//! FHIR Bulk Data `$export` worker (loco `BackgroundWorker` on `bg_pg`).
//!
//! `$export` kickoff enqueues a job here and returns `202` immediately;
//! this worker materialises the NDJSON, writes it to the
//! [`ArtifactStore`](crate::bulk::store::ArtifactStore), and moves the
//! `bulk_jobs` row to `completed`.
//!
//! ## What this changes
//!
//! The previous implementation materialised the export **inside the
//! kickoff request** and kept it in a process-local registry. That met
//! the Bulk Data IG's shape but not its intent, and it had three real
//! limits the spec had to admit to: jobs vanished on restart, another
//! replica could not see them (a client polling through a load balancer
//! got a `404` for a job that had succeeded), and a large export blocked
//! the request thread.
//!
//! Now the work happens off the request path, the state is a row in
//! Postgres, and the bytes are in the artifact store — so a poll from any
//! replica, at any time inside the retention window, answers correctly.
//!
//! Jobs run on `bg_pg` (`agents/share/loco.md`), the family's
//! Postgres-backed queue, so there is still no external broker.

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bulk::store::{ArtifactStore, LocalFsArtifactStore};
use crate::compliance::bulk;
use crate::fhir::to_fhir_plan_definition;
use crate::models::bulk_jobs::Model as JobModel;
use crate::models::care_pathways::Model as PathwayModel;

/// Background worker that materialises a queued `$export` job.
pub struct BulkExportWorker {
    /// The shared loco application context (database, queue, config).
    pub ctx: AppContext,
}

/// Arguments for a [`BulkExportWorker`] job: just the `bulk_jobs` row id.
///
/// Deliberately nothing else — the job row is the single source of truth
/// for what to export, so a worker retry cannot act on stale arguments
/// that disagree with the persisted request.
#[derive(Deserialize, Debug, Serialize)]
pub struct BulkExportArgs {
    /// The `bulk_jobs.id` to materialise.
    pub job_id: Uuid,
}

#[async_trait]
impl BackgroundWorker<BulkExportArgs> for BulkExportWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    /// Materialise one export.
    ///
    /// A missing job is a no-op rather than an error: the row may have
    /// been swept by retention, and failing the queue item would only
    /// retry a job whose state is gone.
    ///
    /// # Errors
    ///
    /// Propagates database errors; a failure to *render* is recorded on
    /// the job row as `failed` rather than propagated, so the client sees
    /// a definite outcome instead of a job stuck in `running`.
    async fn perform(&self, args: BulkExportArgs) -> Result<()> {
        let Some(job) = JobModel::find_by_id(&self.ctx.db, args.job_id).await? else {
            tracing::warn!(job_id = %args.job_id, "bulk export job not found; nothing to do");
            return Ok(());
        };
        // A client may have cancelled between enqueue and pickup.
        if crate::models::bulk_jobs::is_terminal(&job.status) {
            tracing::info!(job_id = %args.job_id, status = %job.status, "job already terminal");
            return Ok(());
        }
        let job = job.start(&self.ctx.db).await?;

        let rows = PathwayModel::list(&self.ctx.db, bulk::MAX_RESOURCES as u64).await?;
        let resources: Vec<_> = rows
            .iter()
            .filter_map(|model| {
                let pathway = model.to_pathway().ok()?;
                Some(to_fhir_plan_definition(
                    &pathway,
                    &model.pid.to_string(),
                    model.active,
                    Some(model.updated_at.to_rfc3339()),
                ))
            })
            .collect();
        let (ndjson, count, truncated) = bulk::to_ndjson(&resources);

        let store = LocalFsArtifactStore::from_env();
        let key = format!("exports/{}/PlanDefinition.ndjson", job.id);
        match store.put(&key, ndjson.as_bytes()) {
            Ok(reference) => {
                let rows_written = i64::try_from(count).unwrap_or(i64::MAX);
                job.complete(&self.ctx.db, &reference, rows_written).await?;
                tracing::info!(
                    job_id = %args.job_id,
                    rows = count,
                    truncated,
                    "bulk export completed"
                );
            }
            Err(error) => {
                // Record the failure on the row: a client polling must get
                // a definite answer, not a job that sits in `running`.
                tracing::error!(job_id = %args.job_id, %error, "bulk export failed");
                job.fail(&self.ctx.db, &error.to_string()).await?;
            }
        }
        Ok(())
    }
}
