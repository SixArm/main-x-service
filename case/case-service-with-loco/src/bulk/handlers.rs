//! REST handlers for the bulk import/export surface
//! (`agents/share/bulk-import-export.md` §4).
//!
//! ```text
//! POST /api/cases/import        202 {job_id}   multipart file upload
//! GET  /api/cases/import/{id}    job status + counts + errors_url
//! POST /api/cases/export        202 {job_id}   JSON filter body
//! GET  /api/cases/export/{id}    job status + download_url
//! GET  /api/cases/bulk-jobs      list recent jobs
//! ```
//!
//! Submits are async: a handler stores the input (import), inserts a
//! `queued` `bulk_jobs` row, enqueues the [`BulkJobWorker`], and returns
//! `202 Accepted` with the job id. `POST /import` is a declared
//! destructive named POST ([`crate::auth::DESTRUCTIVE_POST_SUFFIXES`]
//! already lists `/import`, anticipating exactly this rollout); the
//! privileged export paths (unmasked `full` / `include_soft_deleted`)
//! are gated explicitly in [`export_case`] (see [`crate::bulk`]'s module
//! docs, "Export governance").

use authentication_verifier::Action;
use axum::extract::Multipart;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse as _;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{MaybeAuthUser, authorize_record};
use crate::bulk::pipeline::export_requires_elevation;
use crate::bulk::worker::{BulkJobArgs, BulkJobWorker};
use crate::bulk::{BulkFormat, MAX_IMPORT_BYTES, MaskingProfile, store};
use crate::models::bulk_jobs::{self, NewBulkJob};

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
/// bytes long push the running total past `max`? Uses a saturating add
/// so a hostile length near `usize::MAX` trips the cap rather than
/// overflowing.
fn exceeds_cap(have: usize, chunk_len: usize, max: usize) -> bool {
    have.saturating_add(chunk_len) > max
}

/// Read a multipart field chunk-by-chunk, bailing the instant the running
/// byte total exceeds `max` (SEC-B2) — the pre-materialisation guard: an
/// oversized or unbounded (chunked-transfer) upload never gets fully
/// buffered in memory.
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
#[derive(Debug, Serialize)]
struct JobAccepted {
    /// The id of the enqueued `bulk_jobs` row.
    job_id: Uuid,
}

/// Export request body: the case list/search filter (§4) plus the §8
/// privacy controls.
#[derive(Debug, Default, Deserialize)]
struct ExportRequest {
    /// File format; `jsonl` (default) or `csv`.
    #[serde(default)]
    format: Option<String>,
    /// Optional title search query.
    #[serde(default)]
    q: Option<String>,
    /// Max records for the listing path.
    #[serde(default)]
    limit: Option<u64>,
    /// Offset for the listing path.
    #[serde(default)]
    offset: Option<u64>,
    /// Masking profile: `masked` (default) or `full` (§8). `full` is
    /// privileged and requires elevated authorisation.
    #[serde(default)]
    masking_profile: Option<String>,
    /// Include soft-deleted records (§8). Defaults to `false`; `true` is
    /// privileged **and** not yet supported (rejected by the worker).
    #[serde(default)]
    include_soft_deleted: Option<bool>,
}

/// A bulk job as returned by the status/list endpoints.
#[derive(Debug, Serialize)]
struct BulkJobView {
    /// The job id.
    id: Uuid,
    /// `import` | `export`.
    kind: String,
    /// The entity (`case`).
    entity: String,
    /// File format token.
    format: String,
    /// Lifecycle status.
    status: String,
    /// Total record rows, once known.
    rows_total: Option<i64>,
    /// Rows processed so far.
    rows_processed: i64,
    /// Rows created.
    rows_created: i64,
    /// Rows upserted.
    rows_upserted: i64,
    /// Rows routed to review.
    rows_to_review: i64,
    /// Rows errored.
    rows_errored: i64,
    /// Export output reference (`download_url` alias).
    download_url: Option<String>,
    /// Per-row error-report reference (`errors_url` alias).
    errors_url: Option<String>,
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
/// when omitted. Unlike the person reference implementation there is no
/// export-only format in this rollout's scope (no Parquet — see
/// [`crate::bulk`]'s module docs), so every recognised token is valid for
/// both import and export.
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

/// Actor pid (bearer `sub`) from the optional authenticated caller.
fn actor_of(caller: &MaybeAuthUser) -> Option<String> {
    caller.claims().map(|c| c.sub.clone())
}

/// The client-supplied idempotency key from the `Idempotency-Key` request
/// header (SEC-B9), if present and valid UTF-8. A retried submit carrying
/// the same key dedupes to the original job.
fn idempotency_key_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// `POST /api/cases/import` — accept a multipart JSONL or CSV upload,
/// enqueue an import job, and return `202 {job_id}`.
#[debug_handler]
async fn import_case(
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

    let job_id = enqueue_import(
        &ctx,
        format,
        &bytes,
        dry_run,
        actor_of(&caller),
        idempotency_key,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(JobAccepted { job_id })).into_response())
}

/// Store the input, insert the `queued` job, and enqueue the worker.
///
/// SEC-B9: when `idempotency_key` names an existing import job, the
/// retried submit resolves to that job — the uploaded bytes are **not**
/// re-stored and the worker is **not** re-enqueued, so the work runs
/// exactly once.
async fn enqueue_import(
    ctx: &AppContext,
    format: BulkFormat,
    bytes: &[u8],
    dry_run: bool,
    actor: Option<String>,
    idempotency_key: Option<String>,
) -> Result<Uuid> {
    let (job, reused) = bulk_jobs::create_or_get_idempotent(
        &ctx.db,
        NewBulkJob::import(format, serde_json::json!({ "dry_run": dry_run }), actor)
            .with_idempotency_key(idempotency_key),
    )
    .await?;

    if reused {
        // Retried submit: return the original job without re-running.
        return Ok(job.id);
    }

    let artifact_store = store::from_env().await?;
    let input_url = artifact_store
        .put(&format!("jobs/{}/input.{}", job.id, format.as_str()), bytes)
        .await?;
    bulk_jobs::set_input_url(&ctx.db, job.id, input_url).await?;

    BulkJobWorker::perform_later(ctx, BulkJobArgs { job_id: job.id }).await?;
    Ok(job.id)
}

/// `POST /api/cases/export` — enqueue an export job and return
/// `202 {job_id}`.
#[debug_handler]
async fn export_case(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
    Json(req): Json<ExportRequest>,
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

    // Gate the privileged paths — the unmasked `full` profile OR
    // soft-deleted inclusion (§8) — behind **elevated authorisation**,
    // reusing case's own record-level guard: a no-op when
    // `CASE_REQUIRE_AUTH` is off; otherwise the ABAC policy must allow a
    // `destructive` action (`access=admin` / `svc=true` under the
    // default policy). A default (masked, active-only) export skips this
    // and stays open to any authorised caller. See `crate::bulk`'s
    // module docs, "Export governance", for the documented scope
    // limitation this check does **not** cover (per-row record-level
    // concealment inside the async worker).
    if export_requires_elevation(masking_profile, include_soft_deleted) {
        authorize_record(
            &caller,
            Action::Destructive,
            &std::collections::BTreeMap::new(),
        )
        .map_err(|(status, reason)| {
            let code = if status == StatusCode::FORBIDDEN {
                "forbidden"
            } else {
                "unauthorized"
            };
            Error::CustomError(status, ErrorDetail::new(code, reason))
        })?;
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

    let (job, reused) = bulk_jobs::create_or_get_idempotent(
        &ctx.db,
        NewBulkJob::export(format, serde_json::Value::Object(params), actor_of(&caller))
            .with_idempotency_key(idempotency_key),
    )
    .await?;

    // SEC-B9: a retried submit resolves to the original job and is not
    // re-enqueued (the work runs exactly once).
    if !reused {
        BulkJobWorker::perform_later(&ctx, BulkJobArgs { job_id: job.id }).await?;
    }
    Ok((StatusCode::ACCEPTED, Json(JobAccepted { job_id: job.id })).into_response())
}

/// `GET /api/cases/import/{id}` — import job status + counts.
#[debug_handler]
async fn get_import_job(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    job_status(&ctx, &caller, id, "import").await
}

/// `GET /api/cases/export/{id}` — export job status + `download_url`.
#[debug_handler]
async fn get_export_job(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    job_status(&ctx, &caller, id, "export").await
}

/// Pure retention check (SEC-B4): has an artifact whose deadline is
/// `expires_at` passed, as of `now`? A `None` deadline never expires.
fn artifact_expired(
    expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    expires_at.is_some_and(|exp| now >= exp)
}

/// Pure ownership check (SEC-B4 IDOR/BOLA): does the caller identified by
/// `caller_sub` own a job whose `job_actor` is given? An unowned job
/// (`job_actor = None`) is never owned by anyone.
fn is_job_owner(caller_sub: &str, job_actor: Option<&str>) -> bool {
    job_actor == Some(caller_sub)
}

/// Whether `caller` may view `job` (SEC-B4 IDOR/BOLA guard). When auth
/// enforcement is off there is no caller identity and visibility is
/// unchanged. When on, a caller may see a job they **own** or one they
/// are **elevated** enough to reach (mirroring the export-elevation
/// gate, so operators/service peers keep full visibility).
fn caller_may_view_job(caller: &MaybeAuthUser, job: &bulk_jobs::Model) -> bool {
    let Some(claims) = caller.claims() else {
        return true;
    };
    is_job_owner(&claims.sub, job.actor.as_deref())
        || authorize_record(
            caller,
            Action::Destructive,
            &std::collections::BTreeMap::new(),
        )
        .is_ok()
}

/// Shared status lookup: load the job, confirming it is of `expect` kind,
/// that the caller may view it (SEC-B4 ownership), and that it has not
/// expired (SEC-B4 TTL). Ownership and expiry failures both return `404`
/// so a cross-actor probe cannot even learn the job exists.
async fn job_status(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    id: Uuid,
    expect: &str,
) -> Result<Response> {
    let not_found = || {
        Error::CustomError(
            StatusCode::NOT_FOUND,
            ErrorDetail::new("not_found", format!("{expect} job '{id}' not found")),
        )
    };
    let job = bulk_jobs::find_by_id(&ctx.db, id)
        .await?
        .ok_or_else(not_found)?;
    if job.kind != expect {
        return Err(not_found());
    }
    if !caller_may_view_job(caller, &job) || artifact_expired(job.expires_at, chrono::Utc::now()) {
        return Err(not_found());
    }
    format::json(BulkJobView::from(job))
}

/// Query params for the bulk-jobs list.
#[derive(Debug, Deserialize)]
struct ListQuery {
    /// Max rows to return (default 50, max 500).
    #[serde(default)]
    limit: Option<u64>,
}

/// `GET /api/cases/bulk-jobs` — list recent bulk jobs, newest first.
#[debug_handler]
async fn list_bulk_jobs(
    State(ctx): State<AppContext>,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<Response> {
    let limit = q.limit.unwrap_or(50).min(500);
    let jobs = bulk_jobs::list_recent(&ctx.db, limit).await?;
    let views: Vec<BulkJobView> = jobs.into_iter().map(BulkJobView::from).collect();
    format::json(views)
}

/// Build the bulk import/export route table. Literal sub-paths
/// (`/import`, `/export`, `/bulk-jobs`) must be registered ahead of the
/// `/{pid}` catch-all in [`crate::controllers::cases::routes`] (same
/// concern that module's own docs flag for `/search`, `/match`, …) —
/// mounted as a separate route table from [`crate::app`], merged before
/// the CRUD table.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/cases")
        .add("/import", post(import_case))
        .add("/import/{id}", get(get_import_job))
        .add("/export", post(export_case))
        .add("/export/{id}", get(get_export_job))
        .add("/bulk-jobs", get(list_bulk_jobs))
}

#[cfg(test)]
mod tests {
    use super::{artifact_expired, exceeds_cap, is_job_owner, parse_format};
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

    /// SEC-B4: a job is expired once `now` reaches its deadline; a job
    /// with no deadline never expires.
    #[test]
    fn artifact_expired_only_at_or_past_the_deadline() {
        let t0: chrono::DateTime<chrono::FixedOffset> =
            chrono::DateTime::<chrono::Utc>::UNIX_EPOCH.fixed_offset();
        let deadline = t0 + chrono::Duration::seconds(100);
        let now = |dt: chrono::DateTime<chrono::FixedOffset>| dt.to_utc();
        assert!(!artifact_expired(Some(deadline), now(t0)), "before");
        assert!(
            !artifact_expired(Some(deadline), now(deadline - chrono::Duration::seconds(1))),
            "one second before"
        );
        assert!(artifact_expired(Some(deadline), now(deadline)), "at");
        assert!(
            artifact_expired(Some(deadline), now(deadline + chrono::Duration::seconds(1))),
            "past"
        );
        assert!(!artifact_expired(None, now(deadline)), "no deadline");
    }

    /// SEC-B4: the IDOR/BOLA ownership comparison.
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

    /// `parse_format` accepts every known token; defaults to JSONL when
    /// omitted; rejects an unknown token. No export-only format exists
    /// in this rollout's scope (no Parquet).
    #[test]
    fn parse_format_accepts_jsonl_and_csv_only() {
        assert_eq!(parse_format(None).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_format(Some("jsonl")).unwrap(), BulkFormat::Jsonl);
        assert_eq!(parse_format(Some("csv")).unwrap(), BulkFormat::Csv);
        assert!(parse_format(Some("parquet")).is_err());
        assert!(parse_format(Some("xml")).is_err());
    }
}
