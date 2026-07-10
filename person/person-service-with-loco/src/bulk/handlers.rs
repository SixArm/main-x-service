//! REST handlers for the bulk import/export surface
//! (`agents/share/bulk-import-export.md` §4).
//!
//! ```text
//! POST /api/persons/import        202 {job_id}   multipart file upload
//! GET  /api/persons/import/{id}    job status + counts + errors_url
//! POST /api/persons/export        202 {job_id}   JSON filter body
//! GET  /api/persons/export/{id}    job status + download_url
//! GET  /api/persons/bulk-jobs      list recent jobs
//! ```
//!
//! Submits are async: a handler stores the input (import), inserts a
//! `queued` `bulk_jobs` row, enqueues the [`BulkJobWorker`], and returns
//! `202 Accepted` with the job id. These routes live on the loco router
//! (`persons_routes`), so they can extract the loco `AppContext` to
//! enqueue the job. Mutating routes are gated by the blanket auth guard;
//! `import` is a declared destructive POST (`auth::DESTRUCTIVE_POST_SUFFIXES`).

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use loco_rs::app::AppContext;
use loco_rs::bgworker::BackgroundWorker;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::ApiResponse;
use crate::api::rest::AppState;
use crate::api::rest::auth::MaybeAuthUser;
use crate::bulk::worker::{BulkJobArgs, BulkJobWorker};
use crate::bulk::{BulkFormat, BulkKind};
use crate::db::bulk_jobs::{self, NewBulkJob};

/// `202 Accepted` body for a submitted bulk job.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobAccepted {
    /// The id of the enqueued `bulk_jobs` row.
    pub job_id: Uuid,
}

/// Export request body: the person list/search filter (§4).
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct ExportRequest {
    /// File format; defaults to `jsonl` (the only supported value).
    #[serde(default)]
    pub format: Option<String>,
    /// Optional family-name search query.
    #[serde(default)]
    pub q: Option<String>,
    /// Max records for the unfiltered listing path.
    #[serde(default)]
    pub limit: Option<u64>,
    /// Offset for the unfiltered listing path.
    #[serde(default)]
    pub offset: Option<u64>,
}

/// A bulk job as returned by the status/list endpoints. Mirrors the row
/// with `download_url`/`errors_url` aliases per §4.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkJobView {
    /// The job id.
    pub id: Uuid,
    /// `import` | `export`.
    pub kind: String,
    /// The entity (`person`).
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
    /// Rows routed to review (always 0 in step 1).
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

/// Reject a format that is not `jsonl` (CSV/Parquet are later steps).
fn require_jsonl(format: Option<&str>) -> Result<BulkFormat, (StatusCode, Json<ApiResponse<()>>)> {
    match format {
        None => Ok(BulkFormat::Jsonl),
        Some(f) => BulkFormat::parse(f).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    "UNSUPPORTED_FORMAT",
                    format!("format '{f}' is not supported in this rollout step; use 'jsonl'"),
                )),
            )
        }),
    }
}

/// Actor pid (bearer `sub`) from the optional authenticated caller.
fn actor_of(caller: &MaybeAuthUser) -> Option<String> {
    caller.claims().map(|c| c.sub.clone())
}

/// `POST /api/persons/import` — accept a multipart JSONL upload, enqueue
/// an import job, and return `202 {job_id}`.
#[utoipa::path(
    post,
    path = "/api/persons/import",
    tag = "bulk",
    responses(
        (status = 202, description = "Import job accepted", body = JobAccepted),
        (status = 400, description = "Bad request / unsupported format"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn import_person(
    State(state): State<AppState>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut format_field: Option<String> = None;
    let mut dry_run = false;

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "file" => match field.bytes().await {
                        Ok(b) => file_bytes = Some(b.to_vec()),
                        Err(e) => {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(ApiResponse::<JobAccepted>::error(
                                    "BAD_UPLOAD",
                                    format!("failed to read uploaded file: {e}"),
                                )),
                            );
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
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<JobAccepted>::error(
                        "BAD_MULTIPART",
                        format!("malformed multipart body: {e}"),
                    )),
                );
            }
        }
    }

    if let Err((status, body)) = require_jsonl(format_field.as_deref()) {
        return (status, Json(remap(body)));
    }

    let Some(bytes) = file_bytes else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<JobAccepted>::error(
                "MISSING_FILE",
                "an import requires a 'file' multipart field",
            )),
        );
    };

    match enqueue_import(&state, &ctx, &bytes, dry_run, actor_of(&caller)).await {
        Ok(job_id) => (
            StatusCode::ACCEPTED,
            Json(ApiResponse::success(JobAccepted { job_id })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<JobAccepted>::error("BULK_ENQUEUE_FAILED", e)),
        ),
    }
}

/// Store the input, insert the `queued` job, and enqueue the worker.
async fn enqueue_import(
    state: &AppState,
    ctx: &AppContext,
    bytes: &[u8],
    dry_run: bool,
    actor: Option<String>,
) -> Result<Uuid, String> {
    let job = bulk_jobs::create(
        &state.db,
        NewBulkJob::import(serde_json::json!({ "dry_run": dry_run }), actor),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Store the uploaded input under the job id, then record it.
    let input_url = state
        .bulk_store
        .put(&format!("jobs/{}/input.jsonl", job.id), bytes)
        .map_err(|e| e.to_string())?;
    bulk_jobs::set_input_url(&state.db, job.id, input_url)
        .await
        .map_err(|e| e.to_string())?;

    BulkJobWorker::perform_later(ctx, BulkJobArgs { job_id: job.id })
        .await
        .map_err(|e| e.to_string())?;
    Ok(job.id)
}

/// `POST /api/persons/export` — enqueue an export job and return
/// `202 {job_id}`.
#[utoipa::path(
    post,
    path = "/api/persons/export",
    tag = "bulk",
    request_body = ExportRequest,
    responses(
        (status = 202, description = "Export job accepted", body = JobAccepted),
        (status = 400, description = "Bad request / unsupported format"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn export_person(
    State(state): State<AppState>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<ExportRequest>,
) -> impl IntoResponse {
    if let Err((status, body)) = require_jsonl(req.format.as_deref()) {
        return (status, Json(remap(body)));
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

    let created = bulk_jobs::create(
        &state.db,
        NewBulkJob::export(serde_json::Value::Object(params), actor_of(&caller)),
    )
    .await;

    match created {
        Ok(job) => match BulkJobWorker::perform_later(&ctx, BulkJobArgs { job_id: job.id }).await {
            Ok(()) => (
                StatusCode::ACCEPTED,
                Json(ApiResponse::success(JobAccepted { job_id: job.id })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<JobAccepted>::error(
                    "BULK_ENQUEUE_FAILED",
                    e.to_string(),
                )),
            ),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<JobAccepted>::error(
                "DATABASE_ERROR",
                e.to_string(),
            )),
        ),
    }
}

/// `GET /api/persons/import/{id}` — import job status + counts.
#[utoipa::path(
    get,
    path = "/api/persons/import/{id}",
    tag = "bulk",
    params(("id" = Uuid, Path, description = "Bulk job id")),
    responses(
        (status = 200, description = "Job status", body = BulkJobView),
        (status = 404, description = "Job not found")
    )
)]
pub async fn get_import_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    job_status(&state, id, BulkKind::Import).await
}

/// `GET /api/persons/export/{id}` — export job status + `download_url`.
#[utoipa::path(
    get,
    path = "/api/persons/export/{id}",
    tag = "bulk",
    params(("id" = Uuid, Path, description = "Bulk job id")),
    responses(
        (status = 200, description = "Job status", body = BulkJobView),
        (status = 404, description = "Job not found")
    )
)]
pub async fn get_export_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    job_status(&state, id, BulkKind::Export).await
}

/// Shared status lookup: load the job, confirming it is of `expect` kind.
async fn job_status(
    state: &AppState,
    id: Uuid,
    expect: BulkKind,
) -> (StatusCode, Json<ApiResponse<BulkJobView>>) {
    match bulk_jobs::find_by_id(&state.db, id).await {
        Ok(Some(job)) if job.kind == expect.as_str() => (
            StatusCode::OK,
            Json(ApiResponse::success(BulkJobView::from(job))),
        ),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<BulkJobView>::error(
                "NOT_FOUND",
                format!("{} job '{id}' not found", expect.as_str()),
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<BulkJobView>::error(
                "DATABASE_ERROR",
                e.to_string(),
            )),
        ),
    }
}

/// Query params for the bulk-jobs list.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListQuery {
    /// Max rows to return (default 50, max 500).
    #[serde(default)]
    pub limit: Option<u64>,
}

/// `GET /api/persons/bulk-jobs` — list recent bulk jobs, newest first.
#[utoipa::path(
    get,
    path = "/api/persons/bulk-jobs",
    tag = "bulk",
    responses((status = 200, description = "Recent bulk jobs", body = [BulkJobView]))
)]
pub async fn list_bulk_jobs(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(50).min(500);
    match bulk_jobs::list_recent(&state.db, limit).await {
        Ok(jobs) => {
            let views: Vec<BulkJobView> = jobs.into_iter().map(BulkJobView::from).collect();
            (StatusCode::OK, Json(ApiResponse::success(views)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<BulkJobView>>::error(
                "DATABASE_ERROR",
                e.to_string(),
            )),
        ),
    }
}

/// Re-wrap a unit-typed error envelope as a `JobAccepted`-typed one, so
/// the format-validation helper can be shared by both submit handlers.
fn remap(body: Json<ApiResponse<()>>) -> ApiResponse<JobAccepted> {
    let ApiResponse { error, .. } = body.0;
    ApiResponse {
        success: false,
        data: None,
        error,
    }
}
