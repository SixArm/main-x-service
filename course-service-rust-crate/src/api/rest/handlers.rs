//! REST handlers.
//!
//! FR-1..FR-5 + FR-7 are wired against the repository, search engine,
//! matcher, and validation module. FR-6 (match-against-existing),
//! FR-8 (merge), FR-9 (batch dedup), FR-14..FR-16 (audit / privacy)
//! continue to return `501 Not Implemented` via [`not_implemented`].
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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::AppState;
use crate::api::ApiResponse;
use crate::db::audit::AuditContext;
use crate::models::{Course, CourseInstance};
use crate::streaming::{CourseEvent, EventKind};
use crate::validation::{ValidationError, validate_course, validate_instance};

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Health check — always returns `200 healthy` so orchestrators can
/// distinguish "process is up" from "process can talk to DB".
pub async fn health(State(_state): State<AppState>) -> impl IntoResponse {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy",
        service: "course-service",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

/// Catch-all stub handler — every endpoint not yet ticked off in
/// `spec.md §13` routes here.
pub async fn not_implemented(State(_state): State<AppState>) -> impl IntoResponse {
    let body: ApiResponse<()> = ApiResponse::error(
        "NOT_IMPLEMENTED",
        "Endpoint not yet implemented — see spec.md §13 for status.",
    );
    (StatusCode::NOT_IMPLEMENTED, Json(body))
}

// ────────────────── Query / body types ──────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

#[derive(Debug, Deserialize, Default)]
pub struct SearchQuery {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
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

fn default_limit() -> u64 {
    20
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub items: Vec<Course>,
    pub total: usize,
}

/// `MatchResult` carries the candidate id so the front-end can navigate
/// back to the matched record from a duplicate-detection response.
#[derive(Debug, Serialize)]
pub struct ScoredCandidate {
    pub course_id: Uuid,
    pub score: f64,
    pub is_match: bool,
    pub confidence: &'static str,
    pub breakdown: crate::matching::MatchBreakdown,
}

// ────────────────── Handlers (FR-1..FR-5, FR-7) ──────────────────

/// FR-1 — create with duplicate detection.
pub async fn create_course(
    State(state): State<AppState>,
    Json(course): Json<Course>,
) -> impl IntoResponse {
    let errs = validate_course(&course);
    if !errs.is_empty() {
        return validation_response(errs);
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
        Err(e) => return error_response(e),
    }

    let created = match state.course_repository.create(&course).await {
        Ok(c) => c,
        Err(e) => return error_response(e),
    };
    if let Err(e) = state.search_engine.index_course(&created) {
        tracing::warn!("indexing course after create failed: {e}");
    }
    record_create(&state, "Course", created.id, &created, EventKind::CourseCreated).await;
    (StatusCode::CREATED, Json(ApiResponse::success(created))).into_response()
}

/// FR-2 — get by id (includes the in-memory child collections that the
/// repository hydrates today; instances + syllabus are deferred to T-8).
pub async fn get_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.course_repository.get_by_id(&id).await {
        Ok(Some(c)) => Json(ApiResponse::success(c)).into_response(),
        Ok(None) => not_found_response("Course not found"),
        Err(e) => error_response(e),
    }
}

/// FR-3 — replace.
pub async fn update_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut course): Json<Course>,
) -> impl IntoResponse {
    course.id = id;
    let errs = validate_course(&course);
    if !errs.is_empty() {
        return validation_response(errs);
    }
    // Snapshot the existing row so the audit entry can carry old/new
    // values. Failure to read the prior state is non-fatal — the
    // update itself is the source of truth.
    let prior = state
        .course_repository
        .get_by_id(&id)
        .await
        .ok()
        .flatten();
    let updated = match state.course_repository.update(&course).await {
        Ok(c) => c,
        Err(crate::Error::NotFound) => return not_found_response("Course not found"),
        Err(e) => return error_response(e),
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
    Json(ApiResponse::success(updated)).into_response()
}

/// FR-4 — soft delete.
pub async fn delete_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let prior = state
        .course_repository
        .get_by_id(&id)
        .await
        .ok()
        .flatten();
    match state.course_repository.soft_delete(&id).await {
        Ok(()) => {
            if let Err(e) = state.search_engine.delete_course(&id.to_string()) {
                tracing::warn!("removing course segment after soft-delete failed: {e}");
            }
            record_delete(&state, "Course", id, prior.as_ref(), EventKind::CourseDeleted).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(crate::Error::NotFound) => not_found_response("Course not found"),
        Err(e) => error_response(e),
    }
}

/// FR-5 — search.
pub async fn search_courses(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    // Empty query → page through `list`.
    let query = q.q.unwrap_or_default();
    let ids: Vec<String> = if query.trim().is_empty() {
        match state.course_repository.list(q.limit, q.offset).await {
            Ok(rows) => return Json(ApiResponse::success(SearchResponse {
                total: rows.len(),
                items: rows,
            })).into_response(),
            Err(e) => return error_response(e),
        }
    } else if q.fuzzy {
        match state.search_engine.fuzzy_search(&query, q.limit as usize) {
            Ok(v) => v,
            Err(e) => return error_response(e),
        }
    } else {
        match state.search_engine.search(&query, q.limit as usize) {
            Ok(v) => v,
            Err(e) => return error_response(e),
        }
    };

    let mut items = Vec::with_capacity(ids.len());
    for sid in ids {
        let Ok(uuid) = Uuid::parse_str(&sid) else { continue };
        match state.course_repository.get_by_id(&uuid).await {
            Ok(Some(c)) => items.push(c),
            Ok(None) => {} // stale index entry
            Err(e) => return error_response(e),
        }
    }
    let total = items.len();
    Json(ApiResponse::success(SearchResponse { items, total })).into_response()
}

// ────────────────── Instance sub-resource (FR-10..FR-13) ──────────────────

/// FR-10 — list instances ordered `schedule.start_date DESC NULLS LAST`.
pub async fn list_instances(
    State(state): State<AppState>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(e) = require_course_exists(&state, &course_id).await {
        return e;
    }
    match state.course_repository.list_instances(&course_id).await {
        Ok(items) => Json(ApiResponse::success(items)).into_response(),
        Err(e) => error_response(e),
    }
}

/// FR-11 — create instance.
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
        return validation_response(errs);
    }
    match state.course_repository.create_instance(&instance).await {
        Ok(created) => {
            record_instance_create(&state, course_id, &created).await;
            (StatusCode::CREATED, Json(ApiResponse::success(created))).into_response()
        }
        Err(e) => error_response(e),
    }
}

/// FR-12 — replace instance.
pub async fn update_instance_handler(
    State(state): State<AppState>,
    Path((course_id, instance_id)): Path<(Uuid, Uuid)>,
    Json(mut instance): Json<CourseInstance>,
) -> impl IntoResponse {
    instance.course_id = course_id;
    instance.id = instance_id;
    let errs = validate_instance(&instance);
    if !errs.is_empty() {
        return validation_response(errs);
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
        Err(e) => error_response(e),
    }
}

/// FR-13 — soft-delete instance.
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
        Err(e) => error_response(e),
    }
}

async fn require_course_exists(
    state: &AppState,
    course_id: &Uuid,
) -> std::result::Result<(), axum::response::Response> {
    match state.course_repository.get_by_id(course_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found_response("Course not found")),
        Err(e) => Err(error_response(e)),
    }
}

/// FR-7 — duplicate check (no write).
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(course): Json<Course>,
) -> impl IntoResponse {
    match find_probable_duplicates(&state, &course).await {
        Ok(hits) => Json(ApiResponse::success(hits)).into_response(),
        Err(e) => error_response(e),
    }
}

// ────────────────── Helpers ──────────────────

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
        let Ok(uuid) = Uuid::parse_str(&sid) else { continue };
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
                score: r.score,
                is_match: r.is_match,
                confidence: confidence_label(r.confidence),
                breakdown: r.breakdown,
            }
        })
        .filter(|r| r.is_match)
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(scored)
}

fn confidence_label(c: crate::matching::MatchConfidence) -> &'static str {
    match c {
        crate::matching::MatchConfidence::High => "High",
        crate::matching::MatchConfidence::Medium => "Medium",
        crate::matching::MatchConfidence::Low => "Low",
    }
}

fn validation_response(errs: Vec<ValidationError>) -> axum::response::Response {
    let body: ApiResponse<Vec<ValidationError>> = ApiResponse::error_with_details(
        "VALIDATION_FAILED",
        "Request failed validation; see `details` for field-scoped errors.",
        &errs,
    );
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

fn not_found_response(msg: &str) -> axum::response::Response {
    let body: ApiResponse<()> = ApiResponse::error("NOT_FOUND", msg);
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

// ────────────────── Audit / streaming hooks (FR-17, FR-18) ──────────────────

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
}

/// FR-14 — entries for a Course (and any child whose audit row carries
/// the same `entity_id` once the merge / instance handlers tag them
/// against the parent). Newest first.
pub async fn audit_for_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    match state.audit_log.list_for_entity(id, q.limit).await {
        Ok(rows) => Json(ApiResponse::success(rows)).into_response(),
        Err(e) => error_response(e),
    }
}

/// System-wide recent-activity tail, newest first.
pub async fn audit_recent(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    match state.audit_log.list_recent(q.limit).await {
        Ok(rows) => Json(ApiResponse::success(rows)).into_response(),
        Err(e) => error_response(e),
    }
}

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
        .log_create(entity_type, entity_id, new_json.clone(), &AuditContext::default())
        .await
    {
        tracing::warn!("audit_log.log_create failed: {e}");
    }
    let evt = CourseEvent::course(event_kind, entity_id, new_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed: {e}");
    }
}

async fn record_update(
    state: &AppState,
    entity_type: &str,
    entity_id: Uuid,
    old: Option<&impl Serialize>,
    new_value: &impl Serialize,
    event_kind: EventKind,
) {
    let old_json = old
        .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
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

async fn record_delete(
    state: &AppState,
    entity_type: &str,
    entity_id: Uuid,
    old: Option<&impl Serialize>,
    event_kind: EventKind,
) {
    let old_json = old
        .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_delete(entity_type, entity_id, old_json.clone(), &AuditContext::default())
        .await
    {
        tracing::warn!("audit_log.log_delete failed: {e}");
    }
    let evt = CourseEvent::course(event_kind, entity_id, old_json);
    if let Err(e) = state.event_publisher.publish(evt).await {
        tracing::warn!("event_publisher.publish failed: {e}");
    }
}

async fn record_instance_create(state: &AppState, course_id: Uuid, instance: &CourseInstance) {
    let payload = serde_json::to_value(instance).unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_create("CourseInstance", course_id, payload.clone(), &AuditContext::default())
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

async fn record_instance_update(
    state: &AppState,
    course_id: Uuid,
    prior: Option<&CourseInstance>,
    updated: &CourseInstance,
) {
    let old_json = prior
        .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
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

async fn record_instance_delete(
    state: &AppState,
    course_id: Uuid,
    instance_id: Uuid,
    prior: Option<&CourseInstance>,
) {
    let payload = prior
        .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
    if let Err(e) = state
        .audit_log
        .log_delete("CourseInstance", course_id, payload.clone(), &AuditContext::default())
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

fn error_response(e: crate::Error) -> axum::response::Response {
    let (status, code) = match &e {
        crate::Error::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        crate::Error::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED"),
        crate::Error::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
    };
    let body: ApiResponse<()> = ApiResponse::error(code, e.to_string());
    (status, Json(body)).into_response()
}
