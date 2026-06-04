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
use crate::models::Course;
use crate::validation::{ValidationError, validate_course};

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
    Json(ApiResponse::success(updated)).into_response()
}

/// FR-4 — soft delete.
pub async fn delete_course(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.course_repository.soft_delete(&id).await {
        Ok(()) => {
            if let Err(e) = state.search_engine.delete_course(&id.to_string()) {
                tracing::warn!("removing course segment after soft-delete failed: {e}");
            }
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
