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
use crate::bulk::pipeline::{
    ExportParams, ImportOutcome, ImportParams, process_export_job, process_import_job,
};
use crate::bulk::{BulkFormat, BulkKind, JobStatus, MaskingProfile, error_report};
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
    let input = state.bulk_store.get(input_ref).await?;

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

    // SEC-B8: the job's actor (bearer `sub`, else "system") is threaded
    // into every per-row CREATE/UPDATE audit record the import writes, not
    // just the job-level summary row below — a bulk-imported record's
    // audit trail now names who ran the import.
    let ctx = actor_audit_context(job);
    let outcome = process_import_job(
        &state.db,
        state.person_repository.as_ref(),
        &state.search_engine,
        state.matcher.as_ref(),
        &input,
        format,
        &ImportParams { dry_run },
        &ctx,
    )
    .await?;

    let error_report_url = if outcome.errors.is_empty() {
        None
    } else {
        let csv = error_report::to_csv(&outcome.errors);
        Some(
            state
                .bulk_store
                .put(&format!("jobs/{}/errors.csv", job.id), csv.as_bytes())
                .await?,
        )
    };

    bulk_jobs::finish_import(&state.db, job.id, &outcome, error_report_url).await?;

    // SEC-B8: a bulk load of personal data is itself an audited event —
    // write a job-level import audit row carrying the acting operator and the
    // reconciled counts (best-effort; the rows are already committed with
    // their own per-row audit, so an audit-write failure must not fail the
    // whole import — it is logged instead).
    if let Err(e) = state
        .audit_log
        .log_import(
            "PersonBulkImport",
            job.id,
            import_audit_summary(&job.format, job.actor.as_deref(), dry_run, &outcome),
            &ctx,
        )
        .await
    {
        tracing::error!("failed to write bulk-import audit row: {e}");
    }
    Ok(())
}

/// Run an export job: run the pipeline (applying the masking profile),
/// write the JSONL output artifact, record the row count, and write an
/// export audit row (§8).
async fn run_export(state: &AppState, job: &bulk_jobs::Model) -> crate::Result<()> {
    let params = export_params_from_json(&job.params, &job.format);
    let (bytes, rows_total) = process_export_job(state.person_repository.as_ref(), &params).await?;

    let result_url = state
        .bulk_store
        .put(
            &format!("jobs/{}/export.{}", job.id, params.format.as_str()),
            &bytes,
        )
        .await?;

    // SEC-B8: a bulk extract of personal data is a compliance event (§8) and
    // the audit **gates delivery** — it is written (even for a zero-row
    // export) *before* the job is finished, and a failure to audit propagates
    // so the job goes `failed` and the `download_url` is never surfaced. A
    // bulk export must never be retrievable without its audit trail.
    audit_export(state, job, &params, rows_total).await?;

    bulk_jobs::finish_export(&state.db, job.id, rows_total, result_url).await?;
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

/// The [`AuditContext`](crate::db::AuditContext) for a bulk job: the acting
/// operator from the job's `actor` (bearer `sub`), falling back to a
/// `"system"` actor when the job was created without an authenticated
/// caller (SEC-B8 — a real actor is threaded whenever one exists).
fn actor_audit_context(job: &bulk_jobs::Model) -> crate::db::AuditContext {
    crate::db::AuditContext {
        user_id: job.actor.clone().or_else(|| Some("system".to_string())),
        ip_address: None,
        user_agent: None,
    }
}

/// Build the job-level **import** audit summary (SEC-B8): the reconciled
/// counts, the dry-run flag, and the actor. Takes primitives (not the whole
/// row) so it is unit-testable without constructing a `SeaORM` model.
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

/// Write the audit row for a completed export (§8), even for a zero-row
/// export. SEC-B8: the caller runs this **before** finishing the job and
/// propagates its error, so an audit failure blocks delivery of the export
/// artifact (the job goes `failed`, no `download_url`).
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the audit row insert fails.
async fn audit_export(
    state: &AppState,
    job: &bulk_jobs::Model,
    params: &ExportParams,
    rows_total: u64,
) -> crate::Result<()> {
    let ctx = actor_audit_context(job);
    let summary = export_audit_summary(&job.format, job.actor.as_deref(), params, rows_total);
    state
        .audit_log
        .log_export("PersonBulkExport", job.id, summary, &ctx)
        .await
}

#[cfg(test)]
mod tests {
    use super::{ExportParams, ImportOutcome, export_audit_summary, import_audit_summary};

    /// SEC-B8: the import audit summary carries the real actor and the
    /// reconciled counts (not a `system`/empty default).
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

    /// A dry-run import is recorded as such, and an actorless (system) job
    /// records a null actor rather than fabricating one.
    #[test]
    fn import_audit_summary_marks_dry_run_and_null_actor() {
        let s = import_audit_summary("jsonl", None, true, &ImportOutcome::default());
        assert_eq!(s["dry_run"], true);
        assert!(s["actor"].is_null(), "no actor ⇒ null, not fabricated");
    }

    /// SEC-B8: the export audit summary carries the actor, the filter, and
    /// the masking profile / soft-deleted flag (the compliance-relevant
    /// facts of a personal-data extract).
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
