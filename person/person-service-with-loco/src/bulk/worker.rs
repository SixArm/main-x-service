//! The loco `BackgroundWorker` that drains `bulk_jobs`
//! (`agents/share/bulk-import-export.md` §3).
//!
//! The worker is a thin adapter: it loads the job row, marks it
//! `running`, reads the input artifact, delegates to the pure-ish
//! [`pipeline`](crate::bulk::pipeline) functions, writes the output /
//! error-report artifacts, and records the outcome back on the job row.
//! All the per-row logic lives in the pipeline so it can be tested
//! directly (DB-gated) without the live `bg_pg` drain.

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::rest::AppState;
use crate::bulk::pipeline::{ExportParams, ImportParams, process_export_job, process_import_job};
use crate::bulk::{BulkKind, JobStatus, MaskingProfile, error_report};
use crate::db::bulk_jobs;

/// The loco background worker that runs one bulk job.
pub struct BulkJobWorker {
    /// The shared loco application context (database, queue, config).
    pub ctx: AppContext,
}

/// Arguments for a [`BulkJobWorker`] run: the id of the `bulk_jobs` row
/// to drain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobArgs {
    /// The `bulk_jobs.id` to process.
    pub job_id: Uuid,
}

#[async_trait]
impl BackgroundWorker<BulkJobArgs> for BulkJobWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: BulkJobArgs) -> Result<()> {
        run_bulk_job(&self.ctx, args.job_id)
            .await
            .map_err(|e| loco_rs::Error::string(&e.to_string()))
    }
}

/// Drain one bulk job end-to-end. Loads the job, marks it running,
/// dispatches on kind, and records the outcome. A whole-job failure marks
/// the row `failed` and returns the error.
///
/// # Errors
///
/// Returns an error if the shared state is missing, the job row is
/// absent, an artifact is unreadable, or the pipeline / persistence fails.
pub async fn run_bulk_job(ctx: &AppContext, job_id: Uuid) -> crate::Result<()> {
    let state = ctx
        .shared_store
        .get::<AppState>()
        .ok_or_else(|| crate::Error::Internal("AppState missing from shared store".to_string()))?;

    let job = bulk_jobs::find_by_id(&state.db, job_id)
        .await?
        .ok_or_else(|| crate::Error::Internal(format!("bulk job {job_id} not found")))?;

    bulk_jobs::set_status(&state.db, job_id, JobStatus::Running).await?;

    let result = match BulkKind::parse(&job.kind) {
        Some(BulkKind::Import) => run_import(&state, &job).await,
        Some(BulkKind::Export) => run_export(&state, &job).await,
        None => Err(crate::Error::Internal(format!(
            "unknown bulk job kind: {}",
            job.kind
        ))),
    };

    if let Err(ref e) = result {
        tracing::error!("bulk job {job_id} failed: {e}");
        // Best-effort: mark the job failed; keep the original error.
        let _ = bulk_jobs::set_status(&state.db, job_id, JobStatus::Failed).await;
    }
    result
}

/// Run an import job: read the input artifact, run the pipeline, persist
/// the error report (if any), and record the counts.
async fn run_import(state: &AppState, job: &bulk_jobs::Model) -> crate::Result<()> {
    let input_ref = job
        .input_url
        .as_deref()
        .ok_or_else(|| crate::Error::Validation("import job has no input artifact".to_string()))?;
    let input = state.bulk_store.get(input_ref)?;

    let dry_run = job
        .params
        .get("dry_run")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let outcome = process_import_job(
        &state.db,
        state.person_repository.as_ref(),
        &state.search_engine,
        &input,
        &ImportParams { dry_run },
    )
    .await?;

    let error_report_url = if outcome.errors.is_empty() {
        None
    } else {
        let csv = error_report::to_csv(&outcome.errors);
        Some(
            state
                .bulk_store
                .put(&format!("jobs/{}/errors.csv", job.id), csv.as_bytes())?,
        )
    };

    bulk_jobs::finish_import(&state.db, job.id, &outcome, error_report_url).await?;
    Ok(())
}

/// Run an export job: run the pipeline (applying the masking profile),
/// write the JSONL output artifact, record the row count, and write an
/// export audit row (§8).
async fn run_export(state: &AppState, job: &bulk_jobs::Model) -> crate::Result<()> {
    let params = export_params_from_json(&job.params);
    let (bytes, rows_total) = process_export_job(state.person_repository.as_ref(), &params).await?;

    let result_url = state
        .bulk_store
        .put(&format!("jobs/{}/export.jsonl", job.id), &bytes)?;

    // A bulk extract of personal data is itself a compliance event (§8):
    // audit it even for a zero-row export.
    audit_export(state, job, &params, rows_total).await;

    bulk_jobs::finish_export(&state.db, job.id, rows_total, result_url).await?;
    Ok(())
}

/// Derive [`ExportParams`] from a job's stored `params` JSON, including
/// the §8 privacy controls (`masking_profile`, `include_soft_deleted`).
/// An unrecognised `masking_profile` token falls back to the default
/// (`masked`) — the safe direction.
fn export_params_from_json(params: &serde_json::Value) -> ExportParams {
    let defaults = ExportParams::default();
    ExportParams {
        query: params
            .get("q")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        limit: params
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(defaults.limit),
        offset: params
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(defaults.offset),
        masking_profile: params
            .get("masking_profile")
            .and_then(serde_json::Value::as_str)
            .and_then(MaskingProfile::parse)
            .unwrap_or(defaults.masking_profile),
        include_soft_deleted: params
            .get("include_soft_deleted")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(defaults.include_soft_deleted),
    }
}

/// Write an audit row for a completed export (§8): actor, the filter
/// (query / limit / offset), format, masking profile,
/// `include_soft_deleted`, and the row count — even for a zero-row export.
async fn audit_export(
    state: &AppState,
    job: &bulk_jobs::Model,
    params: &ExportParams,
    rows_total: u64,
) {
    let ctx = crate::db::AuditContext {
        user_id: job.actor.clone().or_else(|| Some("system".to_string())),
        ip_address: None,
        user_agent: None,
    };
    let summary = serde_json::json!({
        "kind": "export",
        "format": job.format,
        "filter": {
            "q": params.query,
            "limit": params.limit,
            "offset": params.offset,
        },
        "masking_profile": params.masking_profile.as_str(),
        "include_soft_deleted": params.include_soft_deleted,
        "rows_total": rows_total,
    });
    if let Err(e) = state
        .audit_log
        .log_export("PersonBulkExport", job.id, summary, &ctx)
        .await
    {
        tracing::error!("failed to write bulk-export audit row: {e}");
    }
}
