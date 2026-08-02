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
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use loco_rs::app::AppContext;
use loco_rs::bgworker::BackgroundWorker;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use authentication_verifier::Action;
use std::collections::BTreeMap;

use crate::api::ApiResponse;
use crate::api::rest::AppState;
use crate::api::rest::auth::{MaybeAuthUser, authorize_record};
use crate::bulk::pipeline::export_requires_elevation;
use crate::bulk::worker::{BulkJobArgs, BulkJobWorker};
use crate::bulk::{BulkFormat, BulkKind, MAX_IMPORT_BYTES, MaskingProfile};
use crate::db::bulk_jobs::{self, NewBulkJob};

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
/// bytes long push the running total past `max`? The pure boundary used by
/// [`read_field_capped`] (SEC-B2). Uses a saturating add so a hostile
/// length near `usize::MAX` trips the cap rather than overflowing.
fn exceeds_cap(have: usize, chunk_len: usize, max: usize) -> bool {
    have.saturating_add(chunk_len) > max
}

/// Read a multipart field chunk-by-chunk, bailing the instant the running
/// byte total exceeds `max` (SEC-B2). This is the pre-materialisation
/// guard: an oversized or unbounded (chunked-transfer) upload never gets
/// fully buffered in memory.
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

/// `202 Accepted` body for a submitted bulk job.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobAccepted {
    /// The id of the enqueued `bulk_jobs` row.
    pub job_id: Uuid,
}

/// Export request body: the person list/search filter (§4) plus the §8
/// privacy controls.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct ExportRequest {
    /// File format; `jsonl` (default) or `csv`.
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
    /// Masking profile: `masked` (default) or `full` (§8). `full` is
    /// privileged and requires elevated authorisation.
    #[serde(default)]
    pub masking_profile: Option<String>,
    /// Include soft-deleted records (§8). Defaults to `false`; `true` is
    /// privileged **and** not yet supported (rejected by the worker).
    #[serde(default)]
    pub include_soft_deleted: Option<bool>,
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

/// Parse a caller-supplied format, defaulting to [`BulkFormat::Jsonl`]
/// when omitted. Rejects anything [`BulkFormat::parse`] doesn't
/// recognise. Accepts every known token — including
/// [`BulkFormat::Parquet`], which is export-only — since whether a format
/// is valid for *this operation* is a separate question from whether the
/// token is valid at all; see [`parse_import_format`] for the import-side
/// enforcement.
fn parse_format(format: Option<&str>) -> Result<BulkFormat, (StatusCode, Json<ApiResponse<()>>)> {
    match format {
        None => Ok(BulkFormat::Jsonl),
        Some(f) => BulkFormat::parse(f).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    "UNSUPPORTED_FORMAT",
                    format!("format '{f}' is not supported; use 'jsonl', 'csv', or 'parquet'"),
                )),
            )
        }),
    }
}

/// [`parse_format`], plus rejecting an export-only format (§12 lean:
/// Parquet is export-only) before an import job is ever created.
fn parse_import_format(
    format: Option<&str>,
) -> Result<BulkFormat, (StatusCode, Json<ApiResponse<()>>)> {
    let parsed = parse_format(format)?;
    if parsed.is_export_only() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                "UNSUPPORTED_FORMAT",
                format!(
                    "format '{}' is export-only and cannot be used for import",
                    parsed.as_str()
                ),
            )),
        ));
    }
    Ok(parsed)
}

/// Actor pid (bearer `sub`) from the optional authenticated caller.
fn actor_of(caller: &MaybeAuthUser) -> Option<String> {
    caller.claims().map(|c| c.sub.clone())
}

/// The client-supplied idempotency key from the `Idempotency-Key` request
/// header (SEC-B9), if present and valid UTF-8. A retried submit carrying the
/// same key dedupes to the original job.
fn idempotency_key_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// `POST /api/persons/import` — accept a multipart JSONL or CSV upload, enqueue
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
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
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
                            return (
                                StatusCode::PAYLOAD_TOO_LARGE,
                                Json(ApiResponse::<JobAccepted>::error(
                                    "IMPORT_TOO_LARGE",
                                    format!(
                                        "uploaded file exceeds the {MAX_IMPORT_BYTES}-byte import cap"
                                    ),
                                )),
                            );
                        }
                        CappedRead::Read(e) => {
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

    let format = match parse_import_format(format_field.as_deref()) {
        Ok(f) => f,
        Err((status, body)) => return (status, Json(remap(body))),
    };

    let Some(bytes) = file_bytes else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<JobAccepted>::error(
                "MISSING_FILE",
                "an import requires a 'file' multipart field",
            )),
        );
    };

    match enqueue_import(
        &state,
        &ctx,
        format,
        &bytes,
        dry_run,
        actor_of(&caller),
        idempotency_key,
    )
    .await
    {
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
///
/// SEC-B9: when `idempotency_key` names an existing import job, the retried
/// submit resolves to that job — the uploaded bytes are **not** re-stored and
/// the worker is **not** re-enqueued, so the work runs exactly once.
async fn enqueue_import(
    state: &AppState,
    ctx: &AppContext,
    format: BulkFormat,
    bytes: &[u8],
    dry_run: bool,
    actor: Option<String>,
    idempotency_key: Option<String>,
) -> Result<Uuid, String> {
    let (job, reused) = bulk_jobs::create_or_get_idempotent(
        &state.db,
        NewBulkJob::import(format, serde_json::json!({ "dry_run": dry_run }), actor)
            .with_idempotency_key(idempotency_key),
    )
    .await
    .map_err(|e| e.to_string())?;

    if reused {
        // Retried submit: return the original job without re-running.
        return Ok(job.id);
    }

    // Store the uploaded input under the job id, then record it.
    let input_url = state
        .bulk_store
        .put(&format!("jobs/{}/input.{}", job.id, format.as_str()), bytes)
        .await
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
    headers: HeaderMap,
    Json(req): Json<ExportRequest>,
) -> impl IntoResponse {
    let idempotency_key = idempotency_key_of(&headers);
    let format = match parse_format(req.format.as_deref()) {
        Ok(f) => f,
        Err((status, body)) => return (status, Json(remap(body))),
    };

    // Parse the masking profile (default masked); an unknown token → 400.
    let masking_profile = match req.masking_profile.as_deref() {
        None => MaskingProfile::Masked,
        Some(s) => match MaskingProfile::parse(s) {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<JobAccepted>::error(
                        "UNSUPPORTED_MASKING_PROFILE",
                        format!("masking_profile '{s}' is not supported; use 'masked' or 'full'"),
                    )),
                );
            }
        },
    };
    let include_soft_deleted = req.include_soft_deleted.unwrap_or(false);

    // Gate the privileged paths — the unmasked `full` profile OR
    // soft-deleted inclusion (§8) — behind **elevated authorisation**,
    // reusing person's record-level guard (`authorize_record`): a no-op
    // when `PERSON_REQUIRE_AUTH` is off; otherwise the ABAC policy must
    // allow a `destructive` action (`access=admin` / `svc=true` under the
    // default policy). A default (masked, active-only) export skips this
    // and stays open to any authorised caller.
    if export_requires_elevation(masking_profile, include_soft_deleted)
        && let Err((status, msg)) = authorize_record(&caller, Action::Destructive, &BTreeMap::new())
    {
        let code = if status == StatusCode::UNAUTHORIZED {
            "UNAUTHORIZED"
        } else {
            "FORBIDDEN"
        };
        return (status, Json(ApiResponse::<JobAccepted>::error(code, msg)));
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

    let created = bulk_jobs::create_or_get_idempotent(
        &state.db,
        NewBulkJob::export(format, serde_json::Value::Object(params), actor_of(&caller))
            .with_idempotency_key(idempotency_key),
    )
    .await;

    match created {
        // SEC-B9: a retried submit resolves to the original job and is not
        // re-enqueued (the work runs exactly once).
        Ok((job, true)) => (
            StatusCode::ACCEPTED,
            Json(ApiResponse::success(JobAccepted { job_id: job.id })),
        ),
        Ok((job, false)) => {
            match BulkJobWorker::perform_later(&ctx, BulkJobArgs { job_id: job.id }).await {
                Ok(_job_ref) => (
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
            }
        }
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
    caller: MaybeAuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    job_status(&state, &caller, id, BulkKind::Import).await
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
    caller: MaybeAuthUser,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    job_status(&state, &caller, id, BulkKind::Export).await
}

/// Pure retention check (SEC-B4): has an artifact whose deadline is
/// `expires_at` passed, as of `now`? A `None` deadline (legacy rows) never
/// expires. Shared by the handler and its tests.
fn artifact_expired(expires_at: Option<time::OffsetDateTime>, now: time::OffsetDateTime) -> bool {
    expires_at.is_some_and(|exp| now >= exp)
}

/// Whether `job` has passed its retention deadline as of `now` (SEC-B4).
fn job_is_expired(job: &bulk_jobs::Model, now: time::OffsetDateTime) -> bool {
    artifact_expired(job.expires_at, now)
}

/// Pure ownership check (SEC-B4 IDOR/BOLA): does the caller identified by
/// `caller_sub` own a job whose `job_actor` is given? An unowned job
/// (`job_actor = None`) is never owned by anyone. Shared by the handler
/// and its tests so the exact comparison is pinned.
fn is_job_owner(caller_sub: &str, job_actor: Option<&str>) -> bool {
    job_actor == Some(caller_sub)
}

/// Whether `caller` may view `job` (SEC-B4 IDOR/BOLA guard). When auth
/// enforcement is off there is no caller identity and visibility is
/// unchanged. When on, a caller may see a job they **own**
/// ([`is_job_owner`]) or one they are **elevated** enough to reach (an
/// `access=admin` / `svc=true` token that the ABAC policy would allow a
/// `destructive` action) — mirroring the export-elevation gate so
/// operators/service peers keep full visibility.
fn caller_may_view_job(caller: &MaybeAuthUser, job: &bulk_jobs::Model) -> bool {
    let Some(claims) = caller.claims() else {
        // No verified identity (enforcement off, or a public/unauthenticated
        // read that the blanket guard already permitted) — unchanged.
        return true;
    };
    is_job_owner(&claims.sub, job.actor.as_deref())
        || authorize_record(caller, Action::Destructive, &BTreeMap::new()).is_ok()
}

/// Shared status lookup: load the job, confirming it is of `expect` kind,
/// that the caller may view it (SEC-B4 ownership), and that it has not
/// expired (SEC-B4 TTL). Ownership and expiry failures both return `404`
/// so a cross-actor probe cannot even learn the job exists.
async fn job_status(
    state: &AppState,
    caller: &MaybeAuthUser,
    id: Uuid,
    expect: BulkKind,
) -> (StatusCode, Json<ApiResponse<BulkJobView>>) {
    let not_found = || {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<BulkJobView>::error(
                "NOT_FOUND",
                format!("{} job '{id}' not found", expect.as_str()),
            )),
        )
    };
    match bulk_jobs::find_by_id(&state.db, id).await {
        Ok(Some(job)) if job.kind == expect.as_str() => {
            if !caller_may_view_job(caller, &job)
                || job_is_expired(&job, time::OffsetDateTime::now_utc())
            {
                return not_found();
            }
            (
                StatusCode::OK,
                Json(ApiResponse::success(BulkJobView::from(job))),
            )
        }
        Ok(_) => not_found(),
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

#[cfg(test)]
mod tests {
    use super::{artifact_expired, exceeds_cap, is_job_owner, parse_format, parse_import_format};
    use crate::bulk::BulkFormat;

    /// SEC-B2: the pre-materialisation byte-cap boundary. A chunk that keeps
    /// the running total at or under `max` is accepted; the chunk that
    /// crosses it trips the cap; and a hostile near-`usize::MAX` length trips
    /// it via the saturating add rather than overflowing.
    #[test]
    fn exceeds_cap_trips_only_past_the_ceiling() {
        // Exactly at the cap is fine; one more byte is not.
        assert!(!exceeds_cap(90, 10, 100), "reaching the cap exactly is ok");
        assert!(exceeds_cap(91, 10, 100), "crossing the cap is rejected");
        assert!(!exceeds_cap(0, 100, 100), "a single full-cap chunk is ok");
        assert!(exceeds_cap(0, 101, 100), "a single over-cap chunk trips");
        // Saturating add: a pathological length near usize::MAX must not
        // wrap around to a small total and slip under the cap.
        assert!(exceeds_cap(1, usize::MAX, 100));
    }

    /// SEC-B4: a job is expired once `now` reaches its deadline; a job with
    /// no deadline (legacy rows) never expires.
    #[test]
    fn artifact_expired_only_at_or_past_the_deadline() {
        let t0 = time::OffsetDateTime::UNIX_EPOCH;
        let deadline = t0 + time::Duration::seconds(100);
        assert!(!artifact_expired(Some(deadline), t0), "before the deadline");
        assert!(
            !artifact_expired(Some(deadline), deadline - time::Duration::seconds(1)),
            "one second before"
        );
        assert!(
            artifact_expired(Some(deadline), deadline),
            "at the deadline"
        );
        assert!(
            artifact_expired(Some(deadline), deadline + time::Duration::seconds(1)),
            "past the deadline"
        );
        assert!(
            !artifact_expired(None, deadline),
            "a job with no deadline never expires"
        );
    }

    /// SEC-B4: the IDOR/BOLA ownership comparison. A caller owns only a job
    /// whose `actor` is exactly their `sub`; a different actor or an unowned
    /// (actorless) job is not owned.
    #[test]
    fn is_job_owner_requires_an_exact_actor_match() {
        assert!(is_job_owner("actor-a", Some("actor-a")), "own job");
        assert!(
            !is_job_owner("actor-a", Some("actor-b")),
            "another actor's job is not owned (IDOR)"
        );
        assert!(
            !is_job_owner("actor-a", None),
            "an actorless job is owned by no one"
        );
    }

    /// `parse_format` (the export-side parser) accepts every known token,
    /// including `parquet`; defaults to JSONL when omitted; rejects an
    /// unknown token.
    #[test]
    fn parse_format_accepts_every_known_token_including_parquet() {
        assert_eq!(parse_format(None).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_format(Some("jsonl")).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_format(Some("csv")).unwrap(), BulkFormat::Csv);
        assert_eq!(parse_format(Some("parquet")).unwrap(), BulkFormat::Parquet);
        assert!(parse_format(Some("xml")).is_err());
    }

    /// §12 lean: `parse_import_format` accepts every `parse_format` token
    /// *except* an export-only one (Parquet), which it refuses before an
    /// import job is ever created.
    #[test]
    fn parse_import_format_refuses_export_only_formats() {
        assert_eq!(parse_import_format(None).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_import_format(Some("csv")).unwrap(), BulkFormat::Csv);
        assert!(
            parse_import_format(Some("parquet")).is_err(),
            "parquet is export-only"
        );
    }
}
