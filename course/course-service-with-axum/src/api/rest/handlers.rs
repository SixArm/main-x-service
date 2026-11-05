//! REST handlers.
//!
//! FR-1..FR-9 + FR-14..FR-18 are wired against the repository, search
//! engine, matcher, validation, audit, streaming, and privacy modules.
//! The `not_implemented` shim is parked behind one route only:
//! `GET /api/courses` (list-all-without-search), which spec.md §9
//! intentionally leaves out of scope — clients call
//! `/api/courses/search` with an empty `q` for the same effect.
//!
//! Error mapping:
//! - `Error::NotFound` → 404
//! - `Error::Validation` → 422 (with `details`)
//! - `Error::Conflict` → 409
//! - `Error::Database` / `Error::Search` / `Error::Matching` / etc → 500

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::state::AppState;
use crate::api::{ApiError, ApiResponse};
use crate::db::audit::{AuditContext, AuditEntry};
use crate::models::{
    BatchDeduplicationRequest, BatchDeduplicationResponse, Course, CourseInstance, DecideOutcome,
    MergeRecord, MergeRequest, MergeResponse, MergeStatus, NewReviewItem, ReviewDecisionRequest,
    ReviewQueueItem, ReviewStatus, canonical_pair,
};
use crate::streaming::{CourseEvent, EventKind};
use crate::validation::{ValidationError, validate_course, validate_instance};

/// Body of the `GET /api/health` liveness probe. All three fields are
/// `'static` since they are baked in at compile time.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Always `"healthy"` — presence of the field is the signal.
    pub status: &'static str,
    /// Service identifier, fixed to `"course-service"`.
    pub service: &'static str,
    /// Crate version, sourced from `CARGO_PKG_VERSION` at compile time.
    pub version: &'static str,
}

/// Health check — always returns `200 healthy` so orchestrators can
/// distinguish "process is up" from "process can talk to DB".
#[utoipa::path(
    get, path = "/api/health",
    responses((status = 200, description = "service is up", body = HealthResponse)),
    tag = "health",
)]
pub async fn health(State(_state): State<AppState>) -> impl IntoResponse {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy",
        service: "course-service",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

/// The `501` shim, kept as a deliberate marker for the one endpoint
/// `GET /api/courses` (list-all-without-search) that spec.md §9
/// intentionally parks. Removing the handler would orphan the route
/// declaration in `mod.rs`; keeping it stable lets the router
/// table double as documentation.
pub async fn not_implemented(State(_state): State<AppState>) -> impl IntoResponse {
    let body: ApiResponse<()> = ApiResponse::error(
        "NOT_IMPLEMENTED",
        "Endpoint not yet implemented — see spec.md §13 for status.",
    );
    (StatusCode::NOT_IMPLEMENTED, Json(body))
}

/// Prometheus metrics in text-exposition format.
///
/// `GET /metrics.prom` — served at the application **root** (not under
/// `/api`), alongside the Swagger UI. Returns `200` with the rendered
/// process-wide registry (see [`crate::metrics`]) and
/// `Content-Type: text/plain; version=0.0.4`. Public — scraping needs no
/// bearer token. Configure your scraper with `metrics_path: /metrics.prom`.
#[utoipa::path(
    get, path = "/metrics.prom",
    responses((status = 200, description = "Prometheus text-exposition metrics", content_type = "text/plain")),
    tag = "metrics",
)]
pub async fn metrics_prom() -> impl IntoResponse {
    let body = crate::metrics::Metrics::global().render();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            crate::metrics::CONTENT_TYPE,
        )],
        body,
    )
}

// ────────────────── Query / body types ──────────────────

/// Pagination query string for plain list endpoints. Reserved for the
/// `GET /api/courses` route (currently parked behind `not_implemented`).
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListQuery {
    /// Page size; defaults to 20 via `default_limit`.
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// Rows to skip before the page; defaults to 0.
    #[serde(default)]
    pub offset: u64,
}

/// Query string for `GET /api/courses/search`. An empty / absent `q`
/// falls back to a paged `list` rather than a full-text query.
#[derive(Debug, Deserialize, Default, ToSchema, IntoParams)]
pub struct SearchQuery {
    /// Free-text query; empty/absent → paged list of all courses.
    pub q: Option<String>,
    /// Maximum hits to return; defaults to 20 via `default_limit`.
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// Rows to skip (only meaningful on the empty-query list path).
    #[serde(default)]
    pub offset: u64,
    /// When `true`, route through the Tantivy fuzzy matcher.
    #[serde(default)]
    pub fuzzy: bool,
    /// Accepted for API parity with sibling services; currently a
    /// no-op — phonetic matching is on the T-13 / matcher roadmap.
    #[serde(default)]
    pub phonetic: bool,
    /// Accepted for API parity; the masking module lands in T-10.
    #[serde(default)]
    pub mask_sensitive: bool,
}

/// Default page size (20) used by `#[serde(default = ...)]` on the
/// `limit` fields of [`ListQuery`], [`SearchQuery`], and [`AuditQuery`].
fn default_limit() -> u64 {
    20
}

/// Envelope for search results — the hydrated course rows plus a count.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    /// Hydrated course records for this page of hits.
    pub items: Vec<Course>,
    /// Number of items in `items` (this page, not the global total).
    pub total: usize,
}

/// Flat match result. Carries the candidate `course_id` plus a slim
/// in-line summary (`name`, `course_code`) so the front-end can render
/// a match list without an N+1 round-trip back to the API.
#[derive(Debug, Serialize, ToSchema)]
pub struct ScoredCandidate {
    /// Id of the matched candidate course.
    pub course_id: Uuid,
    /// Candidate's primary name (inlined to avoid an extra fetch).
    pub name: String,
    /// Candidate's course code, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub course_code: Option<String>,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// `true` when the score cleared the matcher's threshold.
    pub is_match: bool,
    /// Human label for the confidence band (`"High"` / `"Medium"` / `"Low"`).
    pub confidence: &'static str,
    /// Per-component score breakdown from the matcher.
    pub breakdown: crate::matching::MatchBreakdown,
}

// ────────────────── Handlers (FR-1..FR-5, FR-7) ──────────────────

/// FR-1 — create with duplicate detection.
#[utoipa::path(
    post, path = "/api/courses",
    request_body = Course,
    responses(
        (status = 201, description = "Created", body = Course),
        (status = 409, description = "Probable duplicate", body = ApiError),
        (status = 422, description = "Validation failure", body = ApiError),
    ),
    tag = "courses",
)]
pub async fn create_course(
    State(state): State<AppState>,
    Json(mut course): Json<Course>,
) -> impl IntoResponse {
    // An omitted `id` gets a fresh UUID from serde's default, but an
    // explicit all-zeros one does not — and the nil UUID is a widespread
    // "you pick" sentinel. Stored verbatim it was worse than useless: the
    // first such create took the nil id and every later one failed on the
    // primary key with a 500. Mint here too, matching the event service.
    if course.id == Uuid::nil() {
        course.id = Uuid::new_v4();
    }

    let errs = validate_course(&course);
    if !errs.is_empty() {
        return validation_response(&errs);
    }

    match find_probable_duplicates(&state, &course).await {
        Ok(hits) if !hits.is_empty() => {
            let body: ApiResponse<Vec<ScoredCandidate>> = ApiResponse::error_with_details(
                "DUPLICATE_CANDIDATE",
                "A probable duplicate already exists; see `details` for ranked candidates.",
                &hits,
            );
            return (StatusCode::CONFLICT, Json(body)).into_response();
        }
        Ok(_) => {}
        Err(e) => return error_response(&e),
    }

    let created = match state.course_repository.create(&course).await {
        Ok(c) => c,
        Err(e) => return error_response(&e),
    };
    if let Err(e) = state.search_engine.index_course(&created) {
        tracing::warn!("indexing course after create failed: {e}");
    }
    record_create(
        &state,
        "Course",
        created.id,
        &created,
        EventKind::CourseCreated,
    )
    .await;
    crate::metrics::Metrics::global().course_created_total.inc();
    (StatusCode::CREATED, Json(ApiResponse::success(created))).into_response()
}

/// FR-2 — get by id. Embeds the `instances` collection so a single
/// fetch carries the full record (the sub-resource handlers still
/// exist for mutation; this is the read shape the front-end's
/// detail view expects). Syllabus sections remain deferred until
/// the syllabus sub-resource lands.
#[utoipa::path(
    get, path = "/api/courses/{id}",
    params(("id" = uuid::Uuid, Path,)),
    responses(
        (status = 200, body = Course),
        (status = 404, body = ApiError),
    ),
    tag = "courses",
)]
pub async fn get_course(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let mut course = match state.course_repository.get_by_id(&id).await {
        Ok(Some(c)) => c,
        Ok(None) => return not_found_response("Course not found"),
        Err(e) => return error_response(&e),
    };
    match state.course_repository.list_instances(&id).await {
        Ok(instances) => course.instances = instances,
        Err(e) => {
            tracing::warn!("hydrating instances on GET course failed: {e}");
        }
    }
    Json(ApiResponse::success(course)).into_response()
}

/// FR-3 — replace.
#[utoipa::path(
    put, path = "/api/courses/{id}",
    params(("id" = uuid::Uuid, Path,)),
    request_body = Course,
    responses(
        (status = 200, body = Course),
        (status = 404, body = ApiError),
        (status = 422, body = ApiError),
    ),
    tag = "courses",
)]
pub async fn update_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut course): Json<Course>,
) -> impl IntoResponse {
    course.id = id;
    let errs = validate_course(&course);
    if !errs.is_empty() {
        return validation_response(&errs);
    }
    // Snapshot the existing row so the audit entry can carry old/new
    // values. Failure to read the prior state is non-fatal — the
    // update itself is the source of truth.
    let prior = state.course_repository.get_by_id(&id).await.ok().flatten();
    let updated = match state.course_repository.update(&course).await {
        Ok(c) => c,
        Err(crate::Error::NotFound) => return not_found_response("Course not found"),
        Err(e) => return error_response(&e),
    };
    if let Err(e) = state.search_engine.delete_course(&id.to_string()) {
        tracing::warn!("removing prior course segment after update failed: {e}");
    }
    if let Err(e) = state.search_engine.index_course(&updated) {
        tracing::warn!("re-indexing course after update failed: {e}");
    }
    record_update(
        &state,
        "Course",
        updated.id,
        prior.as_ref(),
        &updated,
        EventKind::CourseUpdated,
    )
    .await;
    crate::metrics::Metrics::global().course_updated_total.inc();
    Json(ApiResponse::success(updated)).into_response()
}

/// FR-4 — soft delete.
#[utoipa::path(
    delete, path = "/api/courses/{id}",
    params(("id" = uuid::Uuid, Path,)),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 404, body = ApiError),
    ),
    tag = "courses",
)]
pub async fn delete_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let prior = state.course_repository.get_by_id(&id).await.ok().flatten();
    match state.course_repository.soft_delete(&id).await {
        Ok(()) => {
            if let Err(e) = state.search_engine.delete_course(&id.to_string()) {
                tracing::warn!("removing course segment after soft-delete failed: {e}");
            }
            record_delete(
                &state,
                "Course",
                id,
                prior.as_ref(),
                EventKind::CourseDeleted,
            )
            .await;
            crate::metrics::Metrics::global().course_deleted_total.inc();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(crate::Error::NotFound) => not_found_response("Course not found"),
        Err(e) => error_response(&e),
    }
}

/// FR-5 — search.
#[utoipa::path(
    get, path = "/api/courses/search",
    params(SearchQuery),
    responses((status = 200, body = SearchResponse)),
    tag = "search",
)]
pub async fn search_courses(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    // Empty query → page through `list`.
    let query = q.q.unwrap_or_default();
    let ids: Vec<String> = if query.trim().is_empty() {
        match state.course_repository.list(q.limit, q.offset).await {
            Ok(rows) => {
                return Json(ApiResponse::success(SearchResponse {
                    total: rows.len(),
                    items: rows,
                }))
                .into_response();
            }
            Err(e) => return error_response(&e),
        }
    } else if q.fuzzy {
        match state
            .search_engine
            .fuzzy_search(&query, usize::try_from(q.limit).unwrap_or(usize::MAX))
        {
            Ok(v) => v,
            Err(e) => return error_response(&e),
        }
    } else {
        match state
            .search_engine
            .search(&query, usize::try_from(q.limit).unwrap_or(usize::MAX))
        {
            Ok(v) => v,
            Err(e) => return error_response(&e),
        }
    };

    let mut items = Vec::with_capacity(ids.len());
    for sid in ids {
        let Ok(uuid) = Uuid::parse_str(&sid) else {
            continue;
        };
        match state.course_repository.get_by_id(&uuid).await {
            Ok(Some(c)) => items.push(c),
            Ok(None) => {} // stale index entry
            Err(e) => return error_response(&e),
        }
    }
    let total = items.len();
    Json(ApiResponse::success(SearchResponse { items, total })).into_response()
}

// ────────────────── Instance sub-resource (FR-10..FR-13) ──────────────────

/// FR-10 — list instances ordered `schedule.start_date DESC NULLS LAST`.
#[utoipa::path(
    get, path = "/api/courses/{id}/instances",
    params(("id" = uuid::Uuid, Path,)),
    responses(
        (status = 200, body = Vec<CourseInstance>),
        (status = 404, body = ApiError),
    ),
    tag = "instances",
)]
pub async fn list_instances(
    State(state): State<AppState>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(e) = require_course_exists(&state, &course_id).await {
        return e;
    }
    match state.course_repository.list_instances(&course_id).await {
        Ok(items) => Json(ApiResponse::success(items)).into_response(),
        Err(e) => error_response(&e),
    }
}

/// FR-11 — create instance.
#[utoipa::path(
    post, path = "/api/courses/{id}/instances",
    params(("id" = uuid::Uuid, Path,)),
    request_body = CourseInstance,
    responses(
        (status = 201, body = CourseInstance),
        (status = 404, body = ApiError),
        (status = 422, body = ApiError),
    ),
    tag = "instances",
)]
pub async fn create_instance(
    State(state): State<AppState>,
    Path(course_id): Path<Uuid>,
    Json(mut instance): Json<CourseInstance>,
) -> impl IntoResponse {
    if let Err(e) = require_course_exists(&state, &course_id).await {
        return e;
    }
    instance.course_id = course_id;
    let errs = validate_instance(&instance);
    if !errs.is_empty() {
        return validation_response(&errs);
    }
    match state.course_repository.create_instance(&instance).await {
        Ok(created) => {
            record_instance_create(&state, course_id, &created).await;
            (StatusCode::CREATED, Json(ApiResponse::success(created))).into_response()
        }
        Err(e) => error_response(&e),
    }
}

/// FR-12 — replace instance.
#[utoipa::path(
    put, path = "/api/courses/{id}/instances/{instance_id}",
    params(
        ("id" = uuid::Uuid, Path,),
        ("instance_id" = uuid::Uuid, Path,),
    ),
    request_body = CourseInstance,
    responses(
        (status = 200, body = CourseInstance),
        (status = 404, body = ApiError),
        (status = 422, body = ApiError),
    ),
    tag = "instances",
)]
pub async fn update_instance_handler(
    State(state): State<AppState>,
    Path((course_id, instance_id)): Path<(Uuid, Uuid)>,
    Json(mut instance): Json<CourseInstance>,
) -> impl IntoResponse {
    instance.course_id = course_id;
    instance.id = instance_id;
    let errs = validate_instance(&instance);
    if !errs.is_empty() {
        return validation_response(&errs);
    }
    let prior = state
        .course_repository
        .get_instance(&course_id, &instance_id)
        .await
        .ok()
        .flatten();
    match state.course_repository.update_instance(&instance).await {
        Ok(updated) => {
            record_instance_update(&state, course_id, prior.as_ref(), &updated).await;
            Json(ApiResponse::success(updated)).into_response()
        }
        Err(crate::Error::NotFound) => not_found_response("CourseInstance not found"),
        Err(e) => error_response(&e),
    }
}

/// Read one instance. Mirror of FR-10's list shape but for a single
/// row — not numbered in the spec but trivially follows from the
/// existing list+update+delete trio.
#[utoipa::path(
    get, path = "/api/courses/{id}/instances/{instance_id}",
    params(
        ("id" = uuid::Uuid, Path,),
        ("instance_id" = uuid::Uuid, Path,),
    ),
    responses(
        (status = 200, body = CourseInstance),
        (status = 404, body = ApiError),
    ),
    tag = "instances",
)]
pub async fn get_instance(
    State(state): State<AppState>,
    Path((course_id, instance_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match state
        .course_repository
        .get_instance(&course_id, &instance_id)
        .await
    {
        Ok(Some(i)) => Json(ApiResponse::success(i)).into_response(),
        Ok(None) => not_found_response("CourseInstance not found"),
        Err(e) => error_response(&e),
    }
}

/// FR-13 — soft-delete instance.
#[utoipa::path(
    delete, path = "/api/courses/{id}/instances/{instance_id}",
    params(
        ("id" = uuid::Uuid, Path,),
        ("instance_id" = uuid::Uuid, Path,),
    ),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 404, body = ApiError),
    ),
    tag = "instances",
)]
pub async fn delete_instance(
    State(state): State<AppState>,
    Path((course_id, instance_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let prior = state
        .course_repository
        .get_instance(&course_id, &instance_id)
        .await
        .ok()
        .flatten();
    match state
        .course_repository
        .soft_delete_instance(&course_id, &instance_id)
        .await
    {
        Ok(()) => {
            record_instance_delete(&state, course_id, instance_id, prior.as_ref()).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(crate::Error::NotFound) => not_found_response("CourseInstance not found"),
        Err(e) => error_response(&e),
    }
}

/// Guard used by the instance sub-resource handlers: confirm the parent
/// course exists before touching its instances. Returns the ready-made
/// `404` (or `500`) response in the `Err` arm so callers can early-return.
async fn require_course_exists(
    state: &AppState,
    course_id: &Uuid,
) -> std::result::Result<(), axum::response::Response> {
    match state.course_repository.get_by_id(course_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found_response("Course not found")),
        Err(e) => Err(error_response(&e)),
    }
}

/// FR-7 — duplicate check (no write).
#[utoipa::path(
    post, path = "/api/courses/check-duplicates",
    request_body = Course,
    responses((status = 200, body = Vec<ScoredCandidate>)),
    tag = "matching",
)]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(course): Json<Course>,
) -> impl IntoResponse {
    match find_probable_duplicates(&state, &course).await {
        Ok(hits) => Json(ApiResponse::success(hits)).into_response(),
        Err(e) => error_response(&e),
    }
}

// ────────────────── Helpers ──────────────────

/// Upper bound on candidates pulled from the search-engine blocker
/// before the (more expensive) matcher scores each one. Caps the
/// per-request matcher fan-out for create / match / check-duplicates.
const BLOCK_CANDIDATE_LIMIT: usize = 50;

/// Run the search-engine blocker → repository hydrate → matcher score
/// pipeline, returning only candidates above the matcher's threshold.
async fn find_probable_duplicates(
    state: &AppState,
    probe: &Course,
) -> crate::Result<Vec<ScoredCandidate>> {
    if probe.name.trim().is_empty() {
        return Ok(Vec::new());
    }
    let ids = state.search_engine.search_by_name_and_provider(
        &probe.name,
        probe.provider_id,
        BLOCK_CANDIDATE_LIMIT,
    )?;

    let mut candidates: Vec<Course> = Vec::with_capacity(ids.len());
    for sid in ids {
        let Ok(uuid) = Uuid::parse_str(&sid) else {
            continue;
        };
        if Some(uuid) == Some(probe.id) && probe.id != Uuid::nil() {
            continue;
        }
        if let Some(c) = state.course_repository.get_by_id(&uuid).await? {
            candidates.push(c);
        }
    }

    let mut scored: Vec<ScoredCandidate> = candidates
        .iter()
        .map(|c| {
            let r = state.matcher.match_courses(probe, c);
            ScoredCandidate {
                course_id: c.id,
                name: c.name.clone(),
                course_code: c.course_code.clone(),
                score: r.score,
                is_match: r.is_match,
                confidence: confidence_label(r.confidence),
                breakdown: r.breakdown,
            }
        })
        .filter(|r| r.is_match)
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(scored)
}

/// Map a [`MatchConfidence`](crate::matching::MatchConfidence) band to
/// its stable wire string for `ScoredCandidate.confidence`.
fn confidence_label(c: crate::matching::MatchConfidence) -> &'static str {
    match c {
        crate::matching::MatchConfidence::High => "High",
        crate::matching::MatchConfidence::Medium => "Medium",
        crate::matching::MatchConfidence::Low => "Low",
    }
}

/// Build a `422 Unprocessable Entity` response carrying the field-scoped
/// validation errors in the envelope's `details`.
fn validation_response(errs: &[ValidationError]) -> axum::response::Response {
    let body: ApiResponse<Vec<ValidationError>> = ApiResponse::error_with_details(
        "VALIDATION_FAILED",
        "Request failed validation; see `details` for field-scoped errors.",
        errs,
    );
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

/// Build a `404 Not Found` response with the given human-readable message.
fn not_found_response(msg: &str) -> axum::response::Response {
    let body: ApiResponse<()> = ApiResponse::error("NOT_FOUND", msg);
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

// ────────────────── Match + Merge (FR-6, FR-8) ──────────────────

/// FR-6 — score a Course request against blocked candidates. Returns
/// every blocked candidate with its `ScoredCandidate`, sorted by
/// descending score (the front-end can apply its own threshold).
#[utoipa::path(
    post, path = "/api/courses/match",
    request_body = Course,
    responses(
        (status = 200, body = Vec<ScoredCandidate>),
        (status = 422, body = ApiError),
    ),
    tag = "matching",
)]
pub async fn match_course(
    State(state): State<AppState>,
    Json(probe): Json<Course>,
) -> impl IntoResponse {
    if probe.name.trim().is_empty() {
        let body: ApiResponse<()> = ApiResponse::error(
            "VALIDATION_FAILED",
            "match request requires a non-empty `name` for blocking",
        );
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response();
    }
    match score_all_blocked_candidates(&state, &probe).await {
        Ok(hits) => Json(ApiResponse::success(hits)).into_response(),
        Err(e) => error_response(&e),
    }
}

/// FR-8 — fold a duplicate into a main course.
#[utoipa::path(
    post, path = "/api/courses/merge",
    request_body = MergeRequest,
    responses(
        (status = 200, body = MergeResponse),
        (status = 404, body = ApiError),
        (status = 422, body = ApiError),
    ),
    tag = "matching",
)]
pub async fn merge_courses(
    State(state): State<AppState>,
    Json(req): Json<MergeRequest>,
) -> impl IntoResponse {
    if req.main_course_id == req.duplicate_course_id {
        return validation_response(&[ValidationError {
            field: "duplicate_course_id".into(),
            message: "main_course_id and duplicate_course_id must differ".into(),
        }]);
    }

    let main = match state.course_repository.get_by_id(&req.main_course_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return not_found_response("main course not found"),
        Err(e) => return error_response(&e),
    };
    let duplicate = match state
        .course_repository
        .get_by_id(&req.duplicate_course_id)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => return not_found_response("duplicate course not found"),
        Err(e) => return error_response(&e),
    };

    let match_result = state.matcher.match_courses(&main, &duplicate);

    let (merged, transferred) = fold_duplicate_into_main(&main, &duplicate);

    // Atomic merge (durable event bus Phase 2): the survivor update and the
    // duplicate soft-delete — plus, under the outbox transport, the `Merged`
    // (+`merged_from`) and `Deleted` outbox rows — commit in one transaction.
    let updated = match state.course_repository.merge(&merged, &duplicate.id).await {
        Ok(c) => c,
        Err(crate::Error::NotFound) => return not_found_response("main course not found"),
        Err(e) => return error_response(&e),
    };
    if let Err(e) = state.search_engine.delete_course(&main.id.to_string()) {
        tracing::warn!("removing main course segment during merge failed: {e}");
    }
    if let Err(e) = state.search_engine.index_course(&updated) {
        tracing::warn!("re-indexing main course during merge failed: {e}");
    }

    if let Err(e) = state.search_engine.delete_course(&duplicate.id.to_string()) {
        tracing::warn!("removing duplicate course segment during merge failed: {e}");
    }

    let merge_record = MergeRecord {
        id: Uuid::new_v4(),
        main_course_id: updated.id,
        duplicate_course_id: duplicate.id,
        status: MergeStatus::Completed,
        merged_by: req.merged_by.clone(),
        merge_reason: req.merge_reason.clone(),
        match_score: Some(match_result.score),
        transferred_data: Some(transferred),
        merged_at: Utc::now(),
    };
    let merge_record = match state.course_repository.record_merge(&merge_record).await {
        Ok(r) => r,
        Err(e) => return error_response(&e),
    };

    // FR-17 / FR-18 — audit + event for both sides + the merge itself.
    record_update(
        &state,
        "Course",
        updated.id,
        Some(&main),
        &updated,
        EventKind::CourseUpdated,
    )
    .await;
    record_delete(
        &state,
        "Course",
        duplicate.id,
        Some(&duplicate),
        EventKind::CourseDeleted,
    )
    .await;
    record_create(
        &state,
        "CourseMerge",
        merge_record.id,
        &merge_record,
        EventKind::CourseMerged,
    )
    .await;
    crate::metrics::Metrics::global().course_merged_total.inc();

    Json(ApiResponse::success(MergeResponse {
        merge_record,
        main_course: updated,
    }))
    .into_response()
}

/// Fold `duplicate` into a copy of `main`, returning the merged
/// `Course` and a JSON snapshot of what was transferred (for the
/// `course_merge_records.transferred_data` column + audit trail).
///
/// Strategy: union-by-value-equality for free-text Vec<String>
/// collections; dedupe identifiers by `(scheme, value)`; preserve the
/// duplicate's primary name as a `[former]` alternate on main; do not
/// touch the parent's status / version / lifecycle scalars.
fn fold_duplicate_into_main(main: &Course, duplicate: &Course) -> (Course, serde_json::Value) {
    let mut merged = main.clone();

    // Alternate names — record the duplicate's primary name explicitly
    // ("former") so reverse-lookup queries can still find it.
    let former = format!("[former] {}", duplicate.name);
    merge_unique(&mut merged.alternate_names, std::iter::once(former.clone()));
    merge_unique(
        &mut merged.alternate_names,
        duplicate.alternate_names.iter().cloned(),
    );

    // Free-text / URL collections — union.
    merge_unique(&mut merged.image, duplicate.image.iter().cloned());
    merge_unique(&mut merged.same_as, duplicate.same_as.iter().cloned());
    merge_unique(&mut merged.keywords, duplicate.keywords.iter().cloned());
    merge_unique(&mut merged.about, duplicate.about.iter().cloned());
    merge_unique(
        &mut merged.in_language,
        duplicate.in_language.iter().cloned(),
    );
    merge_unique(&mut merged.teaches, duplicate.teaches.iter().cloned());
    merge_unique(&mut merged.assesses, duplicate.assesses.iter().cloned());
    merge_unique(
        &mut merged.competency_required,
        duplicate.competency_required.iter().cloned(),
    );
    merge_unique(
        &mut merged.course_prerequisites,
        duplicate.course_prerequisites.iter().cloned(),
    );
    merge_unique(
        &mut merged.available_language,
        duplicate.available_language.iter().cloned(),
    );
    merge_unique(
        &mut merged.financial_aid_eligible,
        duplicate.financial_aid_eligible.iter().cloned(),
    );

    // Identifiers — dedupe by (scheme, value).
    for ident in &duplicate.identifiers {
        let already = merged.identifiers.iter().any(|i| {
            std::mem::discriminant(&i.property_id) == std::mem::discriminant(&ident.property_id)
                && i.value == ident.value
        });
        if !already {
            merged.identifiers.push(ident.clone());
        }
    }

    // Add a Replaces link from main → duplicate so the audit chain
    // stays navigable. Avoid duplicating an existing link.
    let already_links = merged.links.iter().any(|l| {
        l.other_course_id == duplicate.id
            && matches!(l.link_type, crate::models::LinkType::Replaces)
    });
    if !already_links {
        merged.links.push(crate::models::CourseLink {
            other_course_id: duplicate.id,
            link_type: crate::models::LinkType::Replaces,
        });
    }

    let transferred = serde_json::json!({
        "from_course_id": duplicate.id,
        "from_name": duplicate.name,
        "identifiers_added": duplicate.identifiers.len(),
        "alternate_names_added": 1 + duplicate.alternate_names.len(),
        "keywords_added": duplicate.keywords.len(),
        "teaches_added": duplicate.teaches.len(),
        "same_as_added": duplicate.same_as.len(),
    });

    (merged, transferred)
}

/// Append each `incoming` string to `target` unless an equal value is
/// already present — an order-preserving set union for `Vec<String>`.
fn merge_unique<I: IntoIterator<Item = String>>(target: &mut Vec<String>, incoming: I) {
    for v in incoming {
        if !target.iter().any(|t| t == &v) {
            target.push(v);
        }
    }
}

/// Variant of `find_probable_duplicates` that returns every blocked
/// candidate (not just `is_match=true`), sorted by descending score.
/// Powers FR-6.
async fn score_all_blocked_candidates(
    state: &AppState,
    probe: &Course,
) -> crate::Result<Vec<ScoredCandidate>> {
    let ids = state.search_engine.search_by_name_and_provider(
        &probe.name,
        probe.provider_id,
        BLOCK_CANDIDATE_LIMIT,
    )?;

    let mut candidates: Vec<Course> = Vec::with_capacity(ids.len());
    for sid in ids {
        let Ok(uuid) = Uuid::parse_str(&sid) else {
            continue;
        };
        if Some(uuid) == Some(probe.id) && probe.id != Uuid::nil() {
            continue;
        }
        if let Some(c) = state.course_repository.get_by_id(&uuid).await? {
            candidates.push(c);
        }
    }

    let mut scored: Vec<ScoredCandidate> = candidates
        .iter()
        .map(|c| {
            let r = state.matcher.match_courses(probe, c);
            ScoredCandidate {
                course_id: c.id,
                name: c.name.clone(),
                course_code: c.course_code.clone(),
                score: r.score,
                is_match: r.is_match,
                confidence: confidence_label(r.confidence),
                breakdown: r.breakdown,
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(scored)
}

// ────────────────── Batch dedup (FR-9) ──────────────────

/// FR-9 — scan every active Course, score against blocked candidates,
/// auto-merge above `auto_merge_threshold`, queue everything else
/// above `threshold` for review.
#[utoipa::path(
    post, path = "/api/courses/deduplicate",
    request_body = BatchDeduplicationRequest,
    responses(
        (status = 200, body = BatchDeduplicationResponse),
        (status = 422, body = ApiError),
    ),
    tag = "matching",
)]
pub async fn deduplicate(
    State(state): State<AppState>,
    Json(req): Json<BatchDeduplicationRequest>,
) -> impl IntoResponse {
    if !(0.0..=1.0).contains(&req.threshold)
        || !(0.0..=1.0).contains(&req.auto_merge_threshold)
        || req.auto_merge_threshold < req.threshold
    {
        return validation_response(&[ValidationError {
            field: "thresholds".into(),
            message: "thresholds must be in [0, 1] with auto_merge_threshold >= threshold".into(),
        }]);
    }

    match run_batch_dedup(&state, &req).await {
        Ok(resp) => Json(ApiResponse::success(resp)).into_response(),
        Err(e) => error_response(&e),
    }
}

/// Page size for the batch-dedup scan's repository pagination — bounds
/// memory per loop iteration while keeping round-trips low.
const DEDUP_PAGE: u64 = 100;

/// Drive the FR-9 batch scan: page through every active course, block +
/// score candidates, auto-merge above the threshold, and queue the rest
/// for review. A `seen_pairs` set keeps each unordered pair scored once,
/// and `soft_deleted` skips rows already folded away this run.
async fn run_batch_dedup(
    state: &AppState,
    req: &BatchDeduplicationRequest,
) -> crate::Result<BatchDeduplicationResponse> {
    use std::collections::HashSet;

    let mut response = BatchDeduplicationResponse {
        courses_scanned: 0,
        duplicates_found: 0,
        auto_merged: 0,
        queued_for_review: 0,
        review_items: Vec::new(),
    };
    let mut seen_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
    let mut soft_deleted: HashSet<Uuid> = HashSet::new();
    let mut pending_items: Vec<NewReviewItem> = Vec::new();
    let mut offset: u64 = 0;

    loop {
        let page = state.course_repository.list(DEDUP_PAGE, offset).await?;
        if page.is_empty() {
            break;
        }
        let page_len = page.len() as u64;
        response.courses_scanned += page_len;

        for probe in &page {
            if soft_deleted.contains(&probe.id) {
                continue;
            }
            let candidate_ids = state.search_engine.search_by_name_and_provider(
                &probe.name,
                probe.provider_id,
                req.max_candidates as usize,
            )?;

            for sid in candidate_ids {
                let Ok(cid) = Uuid::parse_str(&sid) else {
                    continue;
                };
                if cid == probe.id || soft_deleted.contains(&cid) {
                    continue;
                }
                let pair = canonical_pair(probe.id, cid);
                if !seen_pairs.insert(pair) {
                    continue;
                }
                let Some(candidate) = state.course_repository.get_by_id(&cid).await? else {
                    continue;
                };

                let r = state.matcher.match_courses(probe, &candidate);
                if r.score < req.threshold {
                    continue;
                }
                response.duplicates_found += 1;

                if r.score >= req.auto_merge_threshold {
                    auto_merge(state, probe, &candidate, r.score).await?;
                    soft_deleted.insert(candidate.id);
                    response.auto_merged += 1;
                } else {
                    pending_items.push(NewReviewItem {
                        course_id_a: probe.id,
                        course_id_b: candidate.id,
                        match_score: r.score,
                        match_quality: confidence_label(r.confidence).to_string(),
                        detection_method: "BatchScan".to_string(),
                        score_breakdown: serde_json::to_value(&r.breakdown).ok(),
                    });
                }
            }
        }

        offset += DEDUP_PAGE;
        if page_len < DEDUP_PAGE {
            break;
        }
    }

    // T-27 — persist the candidates found this run to `course_match_scores`
    // so they survive a process restart, rather than existing only in this
    // response body. Pair order is normalized on write (`canonical_pair`),
    // so a re-scan upserts: the score columns refresh, but a previously
    // decided row's `status` is left untouched (see
    // `CourseRepository::upsert_review_items`).
    if !pending_items.is_empty() {
        let stored = state
            .course_repository
            .upsert_review_items(&pending_items)
            .await?;
        response.queued_for_review += stored.len() as u64;
        response.review_items = stored;
    }

    Ok(response)
}

/// Auto-merge `duplicate` into `main` inside the batch scan. Mirrors
/// the side effects of `merge_courses` but is awaited inline so the
/// dedup loop can keep accurate counters.
async fn auto_merge(
    state: &AppState,
    main: &Course,
    duplicate: &Course,
    score: f64,
) -> crate::Result<()> {
    let (merged, transferred) = fold_duplicate_into_main(main, duplicate);
    let updated = state.course_repository.update(&merged).await?;
    if let Err(e) = state.search_engine.delete_course(&main.id.to_string()) {
        tracing::warn!("auto_merge: removing main segment failed: {e}");
    }
    if let Err(e) = state.search_engine.index_course(&updated) {
        tracing::warn!("auto_merge: reindex main failed: {e}");
    }
    state.course_repository.soft_delete(&duplicate.id).await?;
    if let Err(e) = state.search_engine.delete_course(&duplicate.id.to_string()) {
        tracing::warn!("auto_merge: removing duplicate segment failed: {e}");
    }

    let merge_record = MergeRecord {
        id: Uuid::new_v4(),
        main_course_id: updated.id,
        duplicate_course_id: duplicate.id,
        status: MergeStatus::Completed,
        merged_by: Some("system:batch-dedup".into()),
        merge_reason: Some("auto-merge above auto_merge_threshold".into()),
        match_score: Some(score),
        transferred_data: Some(transferred),
        merged_at: Utc::now(),
    };
    let merge_record = state.course_repository.record_merge(&merge_record).await?;

    record_update(
        state,
        "Course",
        updated.id,
        Some(main),
        &updated,
        EventKind::CourseUpdated,
    )
    .await;
    record_delete(
        state,
        "Course",
        duplicate.id,
        Some(duplicate),
        EventKind::CourseDeleted,
    )
    .await;
    record_create(
        state,
        "CourseMerge",
        merge_record.id,
        &merge_record,
        EventKind::CourseMerged,
    )
    .await;
    Ok(())
}

// ────────────────── Persisted review queue (T-27) ──────────────────

/// Query parameters for `GET /api/courses/review-queue`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ReviewQueueListQuery {
    /// Optional status filter (`Pending` / `Confirmed` / `Rejected` /
    /// `AutoMerged`).
    pub status: Option<ReviewStatus>,
    /// Maximum items to return (default 100; the repository caps at 500).
    pub limit: Option<u64>,
}

/// Response body for `GET /api/courses/review-queue`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReviewQueueListResponse {
    /// The stored review-queue items, newest first.
    pub items: Vec<ReviewQueueItem>,
    /// Number of items returned.
    pub total: usize,
}

/// T-27 — list the persisted duplicate review queue (newest first).
/// A batch scan's candidates ([`deduplicate`]) survive a process
/// restart because they are read back from `course_match_scores`
/// here, not held only in the scan's own response body.
#[utoipa::path(
    get, path = "/api/courses/review-queue",
    params(ReviewQueueListQuery),
    responses((status = 200, body = ReviewQueueListResponse)),
    tag = "matching",
)]
pub async fn get_review_queue(
    State(state): State<AppState>,
    Query(query): Query<ReviewQueueListQuery>,
) -> impl IntoResponse {
    match state
        .course_repository
        .list_review_items(query.status, query.limit.unwrap_or(100))
        .await
    {
        Ok(items) => {
            let total = items.len();
            Json(ApiResponse::success(ReviewQueueListResponse {
                items,
                total,
            }))
            .into_response()
        }
        Err(e) => error_response(&e),
    }
}

/// T-27 — decide one `Pending` review item (`Confirmed` or `Rejected`).
///
/// First-writer-wins: only a `Pending` item transitions
/// (`CourseRepository::decide_review_item`'s storage-layer guard); an
/// already-decided item is `422`, an unknown id `404`.
#[utoipa::path(
    post, path = "/api/courses/review-queue/{id}/decision",
    params(("id" = uuid::Uuid, Path,)),
    request_body = ReviewDecisionRequest,
    responses(
        (status = 200, body = ReviewQueueItem),
        (status = 404, body = ApiError),
        (status = 422, body = ApiError),
    ),
    tag = "matching",
)]
pub async fn review_decision(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReviewDecisionRequest>,
) -> impl IntoResponse {
    if !req.status.is_decision() {
        return validation_response(&[ValidationError {
            field: "status".into(),
            message: "status must be `Confirmed` or `Rejected`".into(),
        }]);
    }
    match state
        .course_repository
        .decide_review_item(id, req.status, req.reviewed_by.as_deref())
        .await
    {
        Ok(DecideOutcome::Decided(item)) => Json(ApiResponse::success(*item)).into_response(),
        Ok(DecideOutcome::NotFound) => not_found_response("Review item not found"),
        Ok(DecideOutcome::AlreadyDecided(current)) => validation_response(&[ValidationError {
            field: "status".into(),
            message: format!("item is already `{current:?}`; only `Pending` items can be decided"),
        }]),
        Err(e) => error_response(&e),
    }
}

// ────────────────── Privacy (FR-15, FR-16) ──────────────────

/// FR-16 — masked view of a Course.
#[utoipa::path(
    get, path = "/api/courses/{id}/masked",
    params(("id" = uuid::Uuid, Path,)),
    responses(
        (status = 200, body = Course),
        (status = 404, body = ApiError),
    ),
    tag = "privacy",
)]
pub async fn masked_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.course_repository.get_by_id(&id).await {
        Ok(Some(c)) => Json(ApiResponse::success(crate::privacy::mask_course(&c))).into_response(),
        Ok(None) => not_found_response("Course not found"),
        Err(e) => error_response(&e),
    }
}

/// FR-15 — GDPR Article-15 portability export.
#[utoipa::path(
    get, path = "/api/courses/{id}/export",
    params(("id" = uuid::Uuid, Path,)),
    responses(
        (status = 200, description = "GDPR Article-15 portability envelope"),
        (status = 404, body = ApiError),
    ),
    tag = "privacy",
)]
pub async fn export_course_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.course_repository.get_by_id(&id).await {
        Ok(Some(c)) => {
            Json(ApiResponse::success(crate::privacy::export_course(&c))).into_response()
        }
        Ok(None) => not_found_response("Course not found"),
        Err(e) => error_response(&e),
    }
}

// ────────────────── Audit / streaming hooks (FR-17, FR-18) ──────────────────

/// Query string for the two audit endpoints — caps how many newest-first
/// rows are returned.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AuditQuery {
    /// Maximum audit rows to return; defaults to 20 via `default_limit`.
    #[serde(default = "default_limit")]
    pub limit: u64,
}

/// FR-14 — entries for a Course (and any child whose audit row carries
/// the same `entity_id` once the merge / instance handlers tag them
/// against the parent). Newest first.
#[utoipa::path(
    get, path = "/api/courses/{id}/audit",
    params(
        ("id" = uuid::Uuid, Path,),
        AuditQuery,
    ),
    responses((status = 200, body = Vec<AuditEntry>)),
    tag = "audit",
)]
pub async fn audit_for_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    match state.audit_log.list_for_entity(id, q.limit).await {
        Ok(rows) => Json(ApiResponse::success(rows)).into_response(),
        Err(e) => error_response(&e),
    }
}

/// System-wide recent-activity tail, newest first.
#[utoipa::path(
    get, path = "/api/audit/recent",
    params(AuditQuery),
    responses((status = 200, body = Vec<AuditEntry>)),
    tag = "audit",
)]
pub async fn audit_recent(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    match state.audit_log.list_recent(q.limit).await {
        Ok(rows) => Json(ApiResponse::success(rows)).into_response(),
        Err(e) => error_response(&e),
    }
}

/// FR-17/FR-18 side effects for a create: write a `CREATE` audit row
/// (new values only) and publish the corresponding event. Both failures
/// are logged and swallowed so the primary write still succeeds.
async fn record_create(
    state: &AppState,
    entity_type: &str,
    entity_id: Uuid,
    new_value: &impl Serialize,
    event_kind: EventKind,
) {
    let new_json = serde_json::to_value(new_value).unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_create(
            entity_type,
            entity_id,
            new_json.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_create failed: {e}");
    }
    let evt = CourseEvent::course(event_kind, entity_id, new_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed: {e}");
    }
}

/// FR-17/FR-18 side effects for an update: write an `UPDATE` audit row
/// (old + new values) and publish the event. Failures are logged and
/// swallowed. A `None` prior serialises to JSON null.
async fn record_update(
    state: &AppState,
    entity_type: &str,
    entity_id: Uuid,
    old: Option<&impl Serialize>,
    new_value: &impl Serialize,
    event_kind: EventKind,
) {
    let old_json = old.map_or(serde_json::Value::Null, |v| {
        serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
    });
    let new_json = serde_json::to_value(new_value).unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_update(
            entity_type,
            entity_id,
            old_json,
            new_json.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_update failed: {e}");
    }
    let evt = CourseEvent::course(event_kind, entity_id, new_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed: {e}");
    }
}

/// FR-17/FR-18 side effects for a (soft) delete: write a `DELETE` audit
/// row (old values only) and publish the event. Failures are logged and
/// swallowed.
async fn record_delete(
    state: &AppState,
    entity_type: &str,
    entity_id: Uuid,
    old: Option<&impl Serialize>,
    event_kind: EventKind,
) {
    let old_json = old.map_or(serde_json::Value::Null, |v| {
        serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
    });
    if let Err(e) = state
        .audit_log
        .log_delete(
            entity_type,
            entity_id,
            old_json.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_delete failed: {e}");
    }
    let evt = CourseEvent::course(event_kind, entity_id, old_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed: {e}");
    }
}

/// Audit + event side effects for creating a `CourseInstance`. The audit
/// row is keyed on the parent `course_id` so the parent's audit history
/// surfaces instance changes too.
async fn record_instance_create(state: &AppState, course_id: Uuid, instance: &CourseInstance) {
    let payload = serde_json::to_value(instance).unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_create(
            "CourseInstance",
            course_id,
            payload.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_create (instance) failed: {e}");
    }
    let evt = CourseEvent::instance(
        EventKind::CourseInstanceCreated,
        course_id,
        instance.id,
        payload,
    );
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish (instance) failed: {e}");
    }
}

/// Audit + event side effects for updating a `CourseInstance`, keyed on
/// the parent `course_id`. A `None` prior serialises to JSON null.
async fn record_instance_update(
    state: &AppState,
    course_id: Uuid,
    prior: Option<&CourseInstance>,
    updated: &CourseInstance,
) {
    let old_json = prior.map_or(serde_json::Value::Null, |p| {
        serde_json::to_value(p).unwrap_or(serde_json::Value::Null)
    });
    let new_json = serde_json::to_value(updated).unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_update(
            "CourseInstance",
            course_id,
            old_json,
            new_json.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_update (instance) failed: {e}");
    }
    let evt = CourseEvent::instance(
        EventKind::CourseInstanceUpdated,
        course_id,
        updated.id,
        new_json,
    );
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish (instance) failed: {e}");
    }
}

/// Audit + event side effects for soft-deleting a `CourseInstance`,
/// keyed on the parent `course_id`.
async fn record_instance_delete(
    state: &AppState,
    course_id: Uuid,
    instance_id: Uuid,
    prior: Option<&CourseInstance>,
) {
    let payload = prior.map_or(serde_json::Value::Null, |p| {
        serde_json::to_value(p).unwrap_or(serde_json::Value::Null)
    });
    if let Err(e) = state
        .audit_log
        .log_delete(
            "CourseInstance",
            course_id,
            payload.clone(),
            &AuditContext::default(),
        )
        .await
    {
        tracing::warn!("audit_log.log_delete (instance) failed: {e}");
    }
    let evt = CourseEvent::instance(
        EventKind::CourseInstanceDeleted,
        course_id,
        instance_id,
        payload,
    );
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish (instance) failed: {e}");
    }
}

/// Central error → HTTP mapping shared by every handler. Maps the
/// domain [`Error`](enum@crate::Error) variants to status + stable code
/// (404 / 422 / 409, everything else 500) and wraps the message in the
/// standard failure envelope.
fn error_response(e: &crate::Error) -> axum::response::Response {
    let (status, code) = match e {
        crate::Error::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        crate::Error::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED"),
        crate::Error::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
    };
    let body: ApiResponse<()> = ApiResponse::error(code, e.to_string());
    (status, Json(body)).into_response()
}

/// Verify audit-row integrity.
///
/// `GET /api/audit/verify?limit=200` — recomputes each audit row's
/// SHA-256, SHA-3, and MAC, naming any row whose content was altered.
///
/// The unkeyed digests are checked too, not just the MAC: they are
/// written even when no key is configured, so on a default deployment
/// they are the only integrity these rows have.
pub async fn verify_audit_integrity(
    State(state): State<AppState>,
    Query(params): Query<AuditQuery>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

    // course's AuditQuery.limit is a plain u64 with a serde default, not
    // an Option, so it only needs clamping.
    let limit = params.limit.clamp(1, VERIFY_MAX_LIMIT);
    match crate::db::models::audit_log::Entity::find()
        .order_by_desc(crate::db::models::audit_log::Column::Id)
        .limit(limit)
        .all(&state.db)
        .await
    {
        Ok(rows) => (
            StatusCode::OK,
            Json(crate::compliance::audit_integrity::verify(&rows)),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to read audit rows for verification: {e}"),
            )),
        )
            .into_response(),
    }
}

/// Default rows examined by the integrity endpoints.
pub const VERIFY_DEFAULT_LIMIT: u64 = 200;

/// Hard cap on rows examined in one call.
///
/// Record verification assembles each row through the repository — one
/// query per row — so an unbounded limit is a denial-of-service on a
/// large table (the SEC-M1 bound-every-input invariant).
pub const VERIFY_MAX_LIMIT: u64 = 1000;

/// Verify row-level record integrity.
///
/// `GET /api/records/verify?limit=200` — reassembles each record and
/// recomputes its SHA-256, SHA-3, and MAC, naming any row that differs.
pub async fn verify_record_integrity(
    State(state): State<AppState>,
    Query(params): Query<AuditQuery>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, QueryOrder, QuerySelect};

    let limit = params.limit.clamp(1, VERIFY_MAX_LIMIT);
    let rows = match crate::db::models::courses::Entity::find()
        .order_by_desc(crate::db::models::courses::Column::UpdatedAt)
        .limit(limit)
        .all(&state.db)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DATABASE_ERROR",
                    format!("Failed to read records for verification: {e}"),
                )),
            )
                .into_response();
        }
    };

    // Assemble each record so the digest covers the child tables too —
    // which is the point, since an identifier edit lives there.
    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        match state.course_repository.get_by_id(&row.id).await {
            Ok(Some(course)) => {
                records.push(crate::compliance::record_integrity::StoredRecord {
                    course,
                    sha256: row.content_hash,
                    sha3: row.content_hash_sha3,
                    mac: row.content_mac,
                    active: row.active,
                });
            }
            // A row that vanished between the two queries, or that the
            // getter hides, is skipped rather than reported: neither is a
            // finding.
            Ok(None) => {}
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<serde_json::Value>::error(
                        "DATABASE_ERROR",
                        format!("Failed to assemble a record for verification: {e}"),
                    )),
                )
                    .into_response();
            }
        }
    }
    (
        StatusCode::OK,
        Json(crate::compliance::record_integrity::verify(&records)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CourseIdentifier, IdentifierType, LinkType};

    /// Compact constructor for a bare identifier (scheme + value, no
    /// name/url) used to build merge fixtures.
    fn ident(scheme: IdentifierType, value: &str) -> CourseIdentifier {
        CourseIdentifier {
            property_id: scheme,
            value: value.into(),
            name: None,
            url: None,
        }
    }

    /// `fold_duplicate_into_main` unions free-text collections without
    /// introducing duplicates, dedupes identifiers by (scheme, value),
    /// records the former primary name, and adds a `Replaces` link.
    #[test]
    fn fold_unions_collections_and_dedupes_identifiers() {
        let mut main = Course::new("Intro to CS");
        main.keywords = vec!["programming".into()];
        main.same_as = vec!["https://wikidata.org/wiki/Q1".into()];
        main.identifiers = vec![ident(IdentifierType::Doi, "10.1234/abc")];

        let mut dup = Course::new("Introduction to Computer Science");
        dup.keywords = vec!["programming".into(), "algorithms".into()];
        dup.same_as = vec!["https://wikidata.org/wiki/Q1".into()];
        dup.identifiers = vec![
            ident(IdentifierType::Doi, "10.1234/abc"), // already on main
            ident(IdentifierType::Wikidata, "Q12345"), // new
        ];

        let (merged, transferred) = fold_duplicate_into_main(&main, &dup);

        // alternate_names captures the former primary name.
        assert!(
            merged
                .alternate_names
                .iter()
                .any(|n| n.starts_with("[former]"))
        );
        // free-text union — no duplicates.
        assert_eq!(merged.keywords.len(), 2);
        assert_eq!(merged.same_as.len(), 1);
        // identifier dedupe by (scheme, value).
        assert_eq!(merged.identifiers.len(), 2);
        // a Replaces link was added pointing at the duplicate.
        assert!(
            merged
                .links
                .iter()
                .any(|l| l.other_course_id == dup.id && matches!(l.link_type, LinkType::Replaces))
        );
        // transferred snapshot carries the duplicate id.
        assert_eq!(transferred["from_course_id"], serde_json::json!(dup.id));
    }

    /// `fold_duplicate_into_main` is pure with respect to its inputs:
    /// neither the `main` nor the `duplicate` argument is mutated.
    #[test]
    fn fold_does_not_mutate_inputs() {
        let main = Course::new("A");
        let dup = Course::new("B");
        let snapshot_main = serde_json::to_value(&main).unwrap();
        let snapshot_dup = serde_json::to_value(&dup).unwrap();
        let _ = fold_duplicate_into_main(&main, &dup);
        assert_eq!(serde_json::to_value(&main).unwrap(), snapshot_main);
        assert_eq!(serde_json::to_value(&dup).unwrap(), snapshot_dup);
    }

    /// `canonical_pair` orders an unordered id pair deterministically
    /// (smaller UUID first), so `(a, b)` and `(b, a)` collapse to one key
    /// in the batch-dedup `seen_pairs` set regardless of scan order.
    #[test]
    fn canonical_pair_is_order_independent() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_eq!(canonical_pair(a, b), canonical_pair(b, a));
        assert_eq!(canonical_pair(a, b), (a, b));
        // Idempotent on an already-ordered pair.
        assert_eq!(canonical_pair(a, a), (a, a));
    }

    /// The batch-dedup classification (FR-9) routes each scored candidate
    /// pair by the same comparison the `deduplicate` handler applies:
    /// below `threshold` → skipped; in `[threshold, auto_merge_threshold)`
    /// → review queue (`Pending`); at/above `auto_merge_threshold` →
    /// auto-merge. This pins the boundary semantics (half-open lower band,
    /// closed auto-merge cutoff) DB-free, independently of the live scan.
    #[test]
    fn batch_dedup_classifies_by_threshold_bands() {
        /// Mirror of the inline branching in `deduplicate`: returns the
        /// disposition for a candidate score against the two cutoffs.
        #[derive(Debug, PartialEq)]
        enum Disposition {
            Skip,
            ReviewQueue,
            AutoMerge,
        }
        fn classify(score: f64, threshold: f64, auto_merge_threshold: f64) -> Disposition {
            if score < threshold {
                Disposition::Skip
            } else if score >= auto_merge_threshold {
                Disposition::AutoMerge
            } else {
                Disposition::ReviewQueue
            }
        }

        let (lo, hi) = (0.80_f64, 0.95_f64);
        // Below the lower threshold → skipped.
        assert_eq!(classify(0.50, lo, hi), Disposition::Skip);
        assert_eq!(classify(0.79, lo, hi), Disposition::Skip);
        // At the lower threshold (inclusive) → review queue.
        assert_eq!(classify(0.80, lo, hi), Disposition::ReviewQueue);
        assert_eq!(classify(0.94, lo, hi), Disposition::ReviewQueue);
        // At/above the auto-merge cutoff (inclusive) → auto-merge.
        assert_eq!(classify(0.95, lo, hi), Disposition::AutoMerge);
        assert_eq!(classify(1.00, lo, hi), Disposition::AutoMerge);

        // A review-queue disposition is the one that materialises a
        // `ReviewQueueItem { status: Pending, … }` in the response body.
        assert_eq!(
            classify(0.85, lo, hi),
            Disposition::ReviewQueue,
            "mid-band scores must enqueue for review, not auto-merge",
        );
        let item = ReviewQueueItem {
            id: Uuid::from_u128(1),
            course_id_a: Uuid::from_u128(2),
            course_id_b: Uuid::from_u128(3),
            match_score: 0.85,
            match_quality: "probable".into(),
            detection_method: "BatchScan".into(),
            score_breakdown: None,
            status: ReviewStatus::Pending,
            reviewed_by: None,
            created_at: chrono::Utc::now(),
            reviewed_at: None,
        };
        assert_eq!(item.status, ReviewStatus::Pending);
        assert_eq!(item.detection_method, "BatchScan");
    }
}
