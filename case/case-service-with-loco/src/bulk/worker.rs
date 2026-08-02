//! The loco `BackgroundWorker` that drains `bulk_jobs`
//! (`agents/share/bulk-import-export.md` §3).
//!
//! The worker is a thin adapter: it loads the job row, marks it
//! `running`, reads the input artifact, delegates to the pure-ish
//! [`pipeline`](crate::bulk::pipeline) functions, writes the output /
//! error-report artifacts, and records the outcome back on the job row.
//! All the per-row logic lives in the pipeline so it can be tested
//! directly (DB-gated) without the live worker drain.
//!
//! **SEC-B8 — export audit gates delivery.** An import's audit write is
//! best-effort (logged on failure, never blocks the job — matching the
//! person reference implementation's own asymmetry, since a failed
//! import audit does not by itself withhold anything the caller did not
//! already ask to import). An **export**'s audit write is different: it
//! runs, and its result is propagated, *before* [`bulk_jobs::finish_export`]
//! — so a failed audit write fails the whole job rather than letting a
//! `download_url` for personal-data records reach `completed` with no
//! accounting of who extracted them.

use loco_rs::app::AppContext;
use loco_rs::bgworker::BackgroundWorker;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bulk::pipeline::{
    ExportParams, ImportOutcome, ImportParams, process_export_job, process_import_job,
};
use crate::bulk::{BulkFormat, BulkKind, JobStatus, MaskingProfile, error_report, store};
use crate::models::audit_logs::Model as AuditModel;
use crate::models::bulk_jobs;

/// The loco background worker that runs one bulk job.
pub struct BulkJobWorker {
    /// The shared loco application context (database, queue, config).
    pub ctx: AppContext,
}

/// Arguments for a [`BulkJobWorker`] run: the id of the `bulk_jobs` row
/// to process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkJobArgs {
    /// The `bulk_jobs.id` to process.
    pub job_id: Uuid,
}

#[async_trait::async_trait]
impl BackgroundWorker<BulkJobArgs> for BulkJobWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: BulkJobArgs) -> loco_rs::Result<()> {
        run_bulk_job(&self.ctx, args.job_id).await
    }
}

/// Drain one bulk job end-to-end. Loads the job, marks it running,
/// dispatches on kind, and records the outcome. A whole-job failure
/// marks the row `failed` and returns the error.
///
/// # Errors
///
/// Returns an error if the job row is absent, an artifact is unreadable,
/// or the pipeline / persistence / (export) audit fails.
pub async fn run_bulk_job(ctx: &AppContext, job_id: Uuid) -> loco_rs::Result<()> {
    let job = bulk_jobs::find_by_id(&ctx.db, job_id)
        .await?
        .ok_or_else(|| loco_rs::Error::Message(format!("bulk job {job_id} not found")))?;

    bulk_jobs::set_status(&ctx.db, job_id, JobStatus::Running).await?;

    let result = match BulkKind::parse(&job.kind) {
        Some(BulkKind::Import) => run_import(ctx, &job).await,
        Some(BulkKind::Export) => run_export(ctx, &job).await,
        None => Err(loco_rs::Error::Message(format!(
            "unknown bulk job kind: {}",
            job.kind
        ))),
    };

    if let Err(ref e) = result {
        tracing::error!("bulk job {job_id} failed: {e}");
        // Best-effort: mark the job failed; keep the original error.
        let _ = bulk_jobs::fail(&ctx.db, job_id).await;
    }
    result
}

/// Run an import job: read the input artifact, run the pipeline, persist
/// the error report (if any), and record the counts.
async fn run_import(ctx: &AppContext, job: &bulk_jobs::Model) -> loco_rs::Result<()> {
    let input_ref = job
        .input_url
        .as_deref()
        .ok_or_else(|| loco_rs::Error::Message("import job has no input artifact".to_string()))?;
    let artifact_store = store::from_env().await?;
    let input = artifact_store.get(input_ref).await?;

    let dry_run = job
        .params
        .get("dry_run")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    // The handler already validated `job.format` against `BulkFormat::parse`
    // before the job was ever created; an unrecognised token here would
    // only mean the row was hand-edited in the database, so falling back
    // to the reference format is the safe direction rather than failing
    // the whole job.
    let format = BulkFormat::parse(&job.format).unwrap_or(BulkFormat::Jsonl);

    let outcome = process_import_job(
        &ctx.db,
        &input,
        format,
        &ImportParams { dry_run },
        job.actor.as_deref(),
    )
    .await?;

    let error_report_url = if outcome.errors.is_empty() {
        None
    } else {
        let csv = error_report::to_csv(&outcome.errors);
        Some(
            artifact_store
                .put(&format!("jobs/{}/errors.csv", job.id), csv.as_bytes())
                .await?,
        )
    };

    bulk_jobs::finish_import(&ctx.db, job.id, &outcome, error_report_url).await?;

    // SEC-B8: a bulk load of personal data is itself an audited event —
    // write a job-level import audit row carrying the acting operator and
    // the reconciled counts (best-effort; every written row already has
    // its own per-row audit from `streaming::create_and_emit`/
    // `update_and_emit`, so an audit-write failure here must not fail the
    // whole import — it is logged instead, matching the asymmetry the
    // export path does not share; see the module docs).
    if let Err(e) = AuditModel::record(
        &ctx.db,
        job.id,
        "bulk_import",
        job.actor.as_deref(),
        Some(import_audit_summary(
            &job.format,
            job.actor.as_deref(),
            dry_run,
            &outcome,
        )),
    )
    .await
    {
        tracing::error!("failed to write bulk-import audit row: {e}");
    }
    Ok(())
}

/// Run an export job: run the pipeline (applying the masking profile),
/// write the output artifact, record the row count, and write the export
/// audit row (§8) — which **gates delivery** (SEC-B8, see module docs).
async fn run_export(ctx: &AppContext, job: &bulk_jobs::Model) -> loco_rs::Result<()> {
    let params = export_params_from_json(&job.params, &job.format);
    let (bytes, rows_total) = process_export_job(&ctx.db, &params).await?;

    let artifact_store = store::from_env().await?;
    let result_url = artifact_store
        .put(
            &format!("jobs/{}/export.{}", job.id, params.format.as_str()),
            &bytes,
        )
        .await?;

    // SEC-B8: the audit write is recorded — and its result propagated —
    // *before* the job is finished, so a failure to audit fails the whole
    // job (`?` here bubbles up through `run_bulk_job`, which marks the row
    // `failed`) rather than letting a `download_url` for case records
    // reach `completed` unaccounted for.
    let summary = export_audit_summary(&job.format, job.actor.as_deref(), &params, rows_total);
    AuditModel::record(
        &ctx.db,
        job.id,
        "bulk_export",
        job.actor.as_deref(),
        Some(summary),
    )
    .await?;

    bulk_jobs::finish_export(&ctx.db, job.id, rows_total, result_url).await?;
    Ok(())
}

/// Derive [`ExportParams`] from a job's stored `params` JSON plus its
/// `format` column, including the §8 privacy controls (`masking_profile`,
/// `include_soft_deleted`). An unrecognised `masking_profile` or `format`
/// token falls back to its default (`masked` / `jsonl`) — the safe
/// direction; the handler already validated `format` before the job was
/// created, so a fallback here only matters for a hand-edited row.
fn export_params_from_json(params: &serde_json::Value, job_format: &str) -> ExportParams {
    let defaults = ExportParams::default();
    ExportParams {
        query: params
            .get("q")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
        limit: crate::bulk::pipeline::clamp_export_limit(
            params
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(defaults.limit),
        ),
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
        format: BulkFormat::parse(job_format).unwrap_or(defaults.format),
    }
}

/// Build the job-level **import** audit summary (SEC-B8): the reconciled
/// counts, the dry-run flag, and the actor. Takes primitives (not the
/// whole row) so it is unit-testable without constructing a `SeaORM`
/// model.
fn import_audit_summary(
    format: &str,
    actor: Option<&str>,
    dry_run: bool,
    outcome: &ImportOutcome,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "import",
        "format": format,
        "dry_run": dry_run,
        "actor": actor,
        "rows_total": outcome.rows_total,
        "rows_created": outcome.rows_created,
        "rows_upserted": outcome.rows_upserted,
        "rows_to_review": outcome.rows_to_review,
        "rows_errored": outcome.rows_errored,
    })
}

/// Build the job-level **export** audit summary (§8): actor, the filter
/// (query / limit / offset), format, masking profile,
/// `include_soft_deleted`, and the row count. Takes primitives so it is
/// unit-testable without constructing a `SeaORM` model.
fn export_audit_summary(
    format: &str,
    actor: Option<&str>,
    params: &ExportParams,
    rows_total: u64,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "export",
        "format": format,
        "actor": actor,
        "filter": {
            "q": params.query,
            "limit": params.limit,
            "offset": params.offset,
        },
        "masking_profile": params.masking_profile.as_str(),
        "include_soft_deleted": params.include_soft_deleted,
        "rows_total": rows_total,
    })
}

#[cfg(test)]
mod tests {
    use super::{ExportParams, ImportOutcome, export_audit_summary, import_audit_summary};

    /// SEC-B8: the import audit summary carries the real actor and the
    /// reconciled counts.
    #[test]
    fn import_audit_summary_carries_actor_and_counts() {
        let outcome = ImportOutcome {
            rows_total: 5,
            rows_created: 3,
            rows_upserted: 1,
            rows_to_review: 0,
            rows_errored: 1,
            errors: vec![],
        };
        let s = import_audit_summary("jsonl", Some("actor-42"), false, &outcome);
        assert_eq!(s["kind"], "import");
        assert_eq!(s["actor"], "actor-42");
        assert_eq!(s["dry_run"], false);
        assert_eq!(s["rows_total"], 5);
        assert_eq!(s["rows_created"], 3);
        assert_eq!(s["rows_upserted"], 1);
        assert_eq!(s["rows_errored"], 1);
    }

    /// A dry-run import is recorded as such, and an actorless (system)
    /// job records a null actor rather than fabricating one.
    #[test]
    fn import_audit_summary_marks_dry_run_and_null_actor() {
        let s = import_audit_summary("jsonl", None, true, &ImportOutcome::default());
        assert_eq!(s["dry_run"], true);
        assert!(s["actor"].is_null(), "no actor ⇒ null, not fabricated");
    }

    /// SEC-B8: the export audit summary carries the actor, the filter,
    /// and the masking profile / soft-deleted flag.
    #[test]
    fn export_audit_summary_carries_actor_filter_and_profile() {
        let params = ExportParams {
            query: Some("Smith".to_string()),
            limit: 25,
            offset: 5,
            ..ExportParams::default()
        };
        let s = export_audit_summary("jsonl", Some("actor-7"), &params, 42);
        assert_eq!(s["kind"], "export");
        assert_eq!(s["actor"], "actor-7");
        assert_eq!(s["filter"]["q"], "Smith");
        assert_eq!(s["filter"]["limit"], 25);
        assert_eq!(s["filter"]["offset"], 5);
        assert_eq!(s["masking_profile"], "masked");
        assert_eq!(s["include_soft_deleted"], false);
        assert_eq!(s["rows_total"], 42);
    }
}
