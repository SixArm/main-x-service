//! REST handlers for the bulk import/export surface
//! (`agents/share/bulk-import-export.md` §4).
//!
//! ```text
//! POST /api/organizations/import        202 {job_id}   multipart file upload
//! GET  /api/organizations/import/{id}    job status + counts + errors_url
//! POST /api/organizations/export        202 {job_id}   JSON filter body
//! GET  /api/organizations/export/{id}    job status + download_url
//! GET  /api/organizations/bulk-jobs      list recent jobs
//! ```
//!
//! Submits are async: a handler stores the input (import), inserts a
//! `queued` `bulk_jobs` row, enqueues the [`BulkJobWorker`], and returns
//! `202 Accepted` with the job id. Mutating routes sit behind the
//! blanket auth guard; `import` is already a declared destructive POST
//! (`crate::auth::DESTRUCTIVE_POST_SUFFIXES`). These routes are mounted
//! **before** `controllers::organizations::routes()` in `app.rs` so their
//! literal paths (`/import`, `/export`, `/bulk-jobs`) are registered
//! ahead of the `/{pid}` capture, mirroring the convention already
//! documented on `controllers::organizations::routes`.

use std::collections::BTreeMap;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use authentication_verifier::Action;

use crate::auth::{MaybeAuthUser, authorize_record};
use crate::bulk::pipeline::export_requires_elevation;
use crate::bulk::worker::{BulkJobArgs, BulkJobWorker};
use crate::bulk::{BulkFormat, BulkKind, MAX_IMPORT_BYTES, MaskingProfile};
use crate::models::bulk_jobs::{self, NewBulkJob};

/// `202 Accepted` body for a submitted bulk job.
#[derive(Debug, Serialize)]
pub struct JobAccepted {
    /// The id of the enqueued `bulk_jobs` row.
    pub job_id: Uuid,
}

/// Build a `202 Accepted` JSON response.
fn accepted<T: Serialize>(body: T) -> Response {
    (StatusCode::ACCEPTED, axum::Json(body)).into_response()
}

/// Export request body: the organization search filter (§4) plus the §8
/// privacy controls.
#[derive(Debug, Default, Deserialize)]
pub struct ExportRequest {
    /// File format; `jsonl` (default) or `csv`.
    #[serde(default)]
    pub format: Option<String>,
    /// Optional full-text search query.
    #[serde(default)]
    pub q: Option<String>,
    /// Max records for the unfiltered listing path.
    #[serde(default)]
    pub limit: Option<u64>,
    /// Offset for the unfiltered listing path.
    #[serde(default)]
    pub offset: Option<u64>,
    /// Masking profile: `masked` (default) or `full` (§8). `full` is
    /// privileged and requires elevated authorisation.
    #[serde(default)]
    pub masking_profile: Option<String>,
    /// Include soft-deleted records (§8). Defaults to `false`; `true` is
    /// privileged **and** not yet supported (rejected before a job is
    /// ever created).
    #[serde(default)]
    pub include_soft_deleted: Option<bool>,
}

/// A bulk job as returned by the status/list endpoints.
#[derive(Debug, Serialize)]
pub struct BulkJobView {
    /// The job id.
    pub id: Uuid,
    /// `import` | `export`.
    pub kind: String,
    /// The entity (`organization`).
    pub entity: String,
    /// File format token.
    pub format: String,
    /// Lifecycle status.
    pub status: String,
    /// Total record rows, once known.
    pub rows_total: Option<i64>,
    /// Rows processed so far.
    pub rows_processed: i64,
    /// Rows created.
    pub rows_created: i64,
    /// Rows upserted.
    pub rows_upserted: i64,
    /// Rows routed to review.
    pub rows_to_review: i64,
    /// Rows errored.
    pub rows_errored: i64,
    /// Export output reference (`download_url` alias).
    pub download_url: Option<String>,
    /// Per-row error-report reference (`errors_url` alias).
    pub errors_url: Option<String>,
}

impl From<bulk_jobs::Model> for BulkJobView {
    fn from(m: bulk_jobs::Model) -> Self {
        Self {
            id: m.id,
            kind: m.kind,
            entity: m.entity,
            format: m.format,
            status: m.status,
            rows_total: m.rows_total,
            rows_processed: m.rows_processed,
            rows_created: m.rows_created,
            rows_upserted: m.rows_upserted,
            rows_to_review: m.rows_to_review,
            rows_errored: m.rows_errored,
            download_url: m.result_url,
            errors_url: m.error_report_url,
        }
    }
}

/// Parse a caller-supplied format, defaulting to [`BulkFormat::Jsonl`]
/// when omitted. Rejects anything [`BulkFormat::parse`] doesn't
/// recognise (`400`) — BLK-5 scope is `jsonl`/`csv` only, so both
/// import and export share this one parser (no Parquet export-only case
/// to special-case).
fn parse_format(format: Option<&str>) -> Result<BulkFormat> {
    match format {
        None => Ok(BulkFormat::Jsonl),
        Some(f) => BulkFormat::parse(f).ok_or_else(|| {
            Error::CustomError(
                StatusCode::BAD_REQUEST,
                ErrorDetail::new(
                    "unsupported_format",
                    format!("format '{f}' is not supported; use 'jsonl' or 'csv'"),
                ),
            )
        }),
    }
}

/// The client-supplied idempotency key from the `Idempotency-Key`
/// request header (SEC-B9), if present and valid UTF-8. A retried submit
/// carrying the same key dedupes to the original job.
fn idempotency_key_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// Outcome of reading a multipart field under a byte cap (SEC-B2).
enum CappedRead {
    /// The field was read in full (within the cap).
    Ok(Vec<u8>),
    /// The running total exceeded the cap before the field ended — the
    /// upload is rejected without being fully materialised.
    TooLarge,
    /// The multipart stream errored mid-field.
    Read(String),
}

/// Would appending a `chunk_len`-byte chunk to a buffer already `have`
/// bytes long push the running total past `max`? The pure boundary used
/// by [`read_field_capped`] (SEC-B2). Uses a saturating add so a hostile
/// length near `usize::MAX` trips the cap rather than overflowing.
fn exceeds_cap(have: usize, chunk_len: usize, max: usize) -> bool {
    have.saturating_add(chunk_len) > max
}

/// Read a multipart field chunk-by-chunk, bailing the instant the
/// running byte total exceeds `max` (SEC-B2). This is the
/// pre-materialisation guard: an oversized or unbounded upload never
/// gets fully buffered in memory.
async fn read_field_capped(
    mut field: axum::extract::multipart::Field<'_>,
    max: usize,
) -> CappedRead {
    let mut buf = Vec::new();
    loop {
        match field.chunk().await {
            Ok(Some(chunk)) => {
                if exceeds_cap(buf.len(), chunk.len(), max) {
                    return CappedRead::TooLarge;
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(None) => return CappedRead::Ok(buf),
            Err(e) => return CappedRead::Read(e.to_string()),
        }
    }
}

/// `POST /api/organizations/import` — accept a multipart JSONL or CSV
/// upload, enqueue an import job, and return `202 {job_id}`.
async fn import_organizations(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response> {
    let idempotency_key = idempotency_key_of(&headers);
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut format_field: Option<String> = None;
    let mut dry_run = false;

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "file" => match read_field_capped(field, MAX_IMPORT_BYTES).await {
                        CappedRead::Ok(b) => file_bytes = Some(b),
                        CappedRead::TooLarge => {
                            return Err(Error::CustomError(
                                StatusCode::PAYLOAD_TOO_LARGE,
                                ErrorDetail::new(
                                    "import_too_large",
                                    format!(
                                        "uploaded file exceeds the {MAX_IMPORT_BYTES}-byte import cap"
                                    ),
                                ),
                            ));
                        }
                        CappedRead::Read(e) => {
                            return Err(Error::CustomError(
                                StatusCode::BAD_REQUEST,
                                ErrorDetail::new(
                                    "bad_upload",
                                    format!("failed to read uploaded file: {e}"),
                                ),
                            ));
                        }
                    },
                    "format" => format_field = field.text().await.ok(),
                    "dry_run" => {
                        dry_run = field
                            .text()
                            .await
                            .is_ok_and(|t| matches!(t.trim(), "1" | "true" | "yes" | "on"));
                    }
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => {
                return Err(Error::CustomError(
                    StatusCode::BAD_REQUEST,
                    ErrorDetail::new("bad_multipart", format!("malformed multipart body: {e}")),
                ));
            }
        }
    }

    let format = parse_format(format_field.as_deref())?;

    let Some(bytes) = file_bytes else {
        return Err(Error::CustomError(
            StatusCode::BAD_REQUEST,
            ErrorDetail::new(
                "missing_file",
                "an import requires a 'file' multipart field",
            ),
        ));
    };

    let actor = caller.actor().map(str::to_string);
    let (job, reused) = bulk_jobs::Model::create_or_get_idempotent(
        &ctx.db,
        NewBulkJob::import(format, serde_json::json!({ "dry_run": dry_run }), actor)
            .with_idempotency_key(idempotency_key),
    )
    .await?;

    if !reused {
        // Store the uploaded input under the job id, then record it.
        let store = crate::bulk::store::from_env().await;
        let input_url = store
            .put(
                &format!("jobs/{}/input.{}", job.id, format.as_str()),
                &bytes,
            )
            .await?;
        bulk_jobs::Model::set_input_url(&ctx.db, job.id, input_url).await?;

        BulkJobWorker::perform_later(&ctx, BulkJobArgs { job_id: job.id }).await?;
    }
    Ok(accepted(JobAccepted { job_id: job.id }))
}

/// `POST /api/organizations/export` — enqueue an export job and return
/// `202 {job_id}`.
async fn export_organizations(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
    axum::Json(req): axum::Json<ExportRequest>,
) -> Result<Response> {
    let idempotency_key = idempotency_key_of(&headers);
    let format = parse_format(req.format.as_deref())?;

    let masking_profile = match req.masking_profile.as_deref() {
        None => MaskingProfile::Masked,
        Some(s) => MaskingProfile::parse(s).ok_or_else(|| {
            Error::CustomError(
                StatusCode::BAD_REQUEST,
                ErrorDetail::new(
                    "unsupported_masking_profile",
                    format!("masking_profile '{s}' is not supported; use 'masked' or 'full'"),
                ),
            )
        })?,
    };
    let include_soft_deleted = req.include_soft_deleted.unwrap_or(false);

    // Fail fast: a real soft-deleted export query is deferred (§12), so
    // this is rejected before a job is ever created rather than only
    // discovered once the worker marks the job `failed`.
    if include_soft_deleted {
        return Err(Error::CustomError(
            StatusCode::BAD_REQUEST,
            ErrorDetail::new(
                "not_yet_supported",
                "include_soft_deleted=true is not yet supported for export",
            ),
        ));
    }

    // Gate the privileged path — the unmasked `full` profile (§8) —
    // behind elevated authorisation, reusing the record-level guard
    // (`authorize_record`): a no-op when `ORGANIZATION_REQUIRE_AUTH` is
    // off; otherwise the ABAC policy must allow a `destructive` action
    // (`access=admin` / `svc=true` under the default policy). The
    // default (masked) export skips this and stays open to any
    // authorised caller.
    if export_requires_elevation(masking_profile, include_soft_deleted) {
        authorize_record(&caller, Action::Destructive, &BTreeMap::new()).map_err(
            |(status, message)| Error::CustomError(status, ErrorDetail::new("forbidden", &message)),
        )?;
    }

    let mut params = serde_json::Map::new();
    if let Some(q) = req.q {
        params.insert("q".to_string(), serde_json::Value::String(q));
    }
    if let Some(limit) = req.limit {
        params.insert("limit".to_string(), serde_json::json!(limit));
    }
    if let Some(offset) = req.offset {
        params.insert("offset".to_string(), serde_json::json!(offset));
    }
    params.insert(
        "masking_profile".to_string(),
        serde_json::Value::String(masking_profile.as_str().to_string()),
    );
    params.insert(
        "include_soft_deleted".to_string(),
        serde_json::json!(include_soft_deleted),
    );

    let actor = caller.actor().map(str::to_string);
    let (job, reused) = bulk_jobs::Model::create_or_get_idempotent(
        &ctx.db,
        NewBulkJob::export(format, serde_json::Value::Object(params), actor)
            .with_idempotency_key(idempotency_key),
    )
    .await?;

    if !reused {
        BulkJobWorker::perform_later(&ctx, BulkJobArgs { job_id: job.id }).await?;
    }
    Ok(accepted(JobAccepted { job_id: job.id }))
}

/// Whether `job` has passed its SEC-B4 retention deadline.
fn job_is_expired(job: &bulk_jobs::Model) -> bool {
    job.is_expired()
}

/// Shared status lookup: load the job, confirming it is of `expect`
/// kind and has not expired (SEC-B4 TTL). Both an unknown id and an
/// expired one return `404`.
async fn job_status(ctx: &AppContext, id: Uuid, expect: BulkKind) -> Result<Response> {
    let not_found = || {
        Error::CustomError(
            StatusCode::NOT_FOUND,
            ErrorDetail::new(
                "not_found",
                format!("{} job '{id}' not found", expect.as_str()),
            ),
        )
    };
    let job = bulk_jobs::Model::find_by_id(&ctx.db, id).await?;
    match job {
        Some(job) if job.kind == expect.as_str() => {
            if job_is_expired(&job) {
                return Err(not_found());
            }
            format::json(BulkJobView::from(job))
        }
        _ => Err(not_found()),
    }
}

/// `GET /api/organizations/import/{id}` — import job status + counts.
async fn get_import_job(State(ctx): State<AppContext>, Path(id): Path<Uuid>) -> Result<Response> {
    job_status(&ctx, id, BulkKind::Import).await
}

/// `GET /api/organizations/export/{id}` — export job status +
/// `download_url`.
async fn get_export_job(State(ctx): State<AppContext>, Path(id): Path<Uuid>) -> Result<Response> {
    job_status(&ctx, id, BulkKind::Export).await
}

/// Query params for the bulk-jobs list.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Max rows to return (default 50, max 500).
    #[serde(default)]
    pub limit: Option<u64>,
}

/// `GET /api/organizations/bulk-jobs` — list recent bulk jobs, newest first.
async fn list_bulk_jobs(
    State(ctx): State<AppContext>,
    Query(q): Query<ListQuery>,
) -> Result<Response> {
    let limit = q.limit.unwrap_or(50).min(500);
    let jobs = bulk_jobs::Model::list_recent(&ctx.db, limit).await?;
    let views: Vec<BulkJobView> = jobs.into_iter().map(BulkJobView::from).collect();
    format::json(views)
}

/// The bulk import/export routes, mounted under `/api/organizations`.
/// Kept as a separate [`Routes`] tree (rather than folded into
/// `controllers::organizations::routes()`) so the module boundary
/// matches the family's `src/bulk/` convention; `app.rs` adds this
/// **before** `controllers::organizations::routes()` so these literal
/// paths are registered ahead of the `/{pid}` capture.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/organizations")
        .add("/import", post(import_organizations))
        .add("/import/{id}", get(get_import_job))
        .add("/export", post(export_organizations))
        .add("/export/{id}", get(get_export_job))
        .add("/bulk-jobs", get(list_bulk_jobs))
}

#[cfg(test)]
mod tests {
    use super::{exceeds_cap, parse_format};
    use crate::bulk::BulkFormat;

    /// SEC-B2: the pre-materialisation byte-cap boundary.
    #[test]
    fn exceeds_cap_trips_only_past_the_ceiling() {
        assert!(!exceeds_cap(90, 10, 100), "reaching the cap exactly is ok");
        assert!(exceeds_cap(91, 10, 100), "crossing the cap is rejected");
        assert!(!exceeds_cap(0, 100, 100), "a single full-cap chunk is ok");
        assert!(exceeds_cap(0, 101, 100), "a single over-cap chunk trips");
        assert!(exceeds_cap(1, usize::MAX, 100));
    }

    /// `parse_format` accepts `jsonl`/`csv`, defaults to JSONL when
    /// omitted, and rejects an unknown token (including `parquet`,
    /// out of scope for this rollout step).
    #[test]
    fn parse_format_accepts_jsonl_and_csv_only() {
        assert_eq!(parse_format(None).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_format(Some("jsonl")).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_format(Some("csv")).unwrap(), BulkFormat::Csv);
        assert!(parse_format(Some("parquet")).is_err());
        assert!(parse_format(Some("xml")).is_err());
    }
}
