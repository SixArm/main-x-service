//! Axum handler functions for every REST endpoint, plus their
//! request/response DTOs.
//!
//! Each handler extracts [`AppState`] and the request body/query, runs the
//! relevant business logic (validation, duplicate detection, matching,
//! merging, masking, audit queries), and returns an [`ApiResponse`] wrapped in
//! the appropriate HTTP status. The `#[utoipa::path(...)]` attributes feed the
//! OpenAPI document assembled in [`super::ApiDoc`]; the route table that maps
//! paths to these functions lives in [`super::create_router`].
//!
//! Handlers are not shown with runnable doctests: they require a live
//! [`AppState`] (database, search index, matcher) that cannot be constructed
//! in a doctest. Behaviour is pinned by the integration tests in `tests/`.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::state::AppState;
use crate::api::ApiResponse;
use crate::models::Worker;

/// Body of the `/api/v1/health` response: a fixed liveness probe payload.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Always `"healthy"` while the process can serve requests.
    pub status: String,
    /// Service name (`"worker-service"`), useful when probing many services.
    pub service: String,
    /// Crate version from `CARGO_PKG_VERSION`, for deploy verification.
    pub version: String,
}

/// Liveness probe: returns a static [`HealthResponse`] for orchestrators
/// (Docker/Kubernetes health checks). Performs no I/O.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "worker-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Prometheus metrics endpoint (text-exposition format).
///
/// Renders [`crate::metrics::METRICS`] for scraping. Mounted at the
/// root (`/metrics.prom`) — not under `/api/v1` — so a default
/// Prometheus scrape config (`metrics_path: /metrics.prom`) finds it.
#[utoipa::path(
    get,
    path = "/metrics.prom",
    tag = "observability",
    responses(
        (status = 200, description = "Prometheus text-exposition format", content_type = "text/plain; version=0.0.4; charset=utf-8")
    )
)]
pub async fn metrics_prom() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            crate::metrics::CONTENT_TYPE,
        )],
        crate::metrics::METRICS.render(),
    )
}

/// Create-worker request body: a flattened [`Worker`], so the JSON is the
/// worker object itself rather than a nested `{ "worker": { ... } }`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkerRequest {
    /// The worker to create (flattened into the top-level JSON object).
    #[serde(flatten)]
    pub worker: Worker,
}

/// Creates a worker after validation and real-time duplicate detection.
///
/// Pipeline: validate (`422` on failure) → assign a UUID if missing →
/// check duplicates (`409` with candidate matches in `error.details` if any)
/// → persist via the repository → index in the search engine (a failure here
/// is logged, not fatal) → `201` with the stored worker.
#[utoipa::path(
    post,
    path = "/api/v1/workers",
    tag = "workers",
    request_body = Worker,
    responses(
        (status = 201, description = "Worker created successfully"),
        (status = 409, description = "Potential duplicates detected"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_worker(
    State(state): State<AppState>,
    Json(mut payload): Json<Worker>,
) -> impl IntoResponse {
    // Step 1 — validate. Required-field / format failures map to 422
    // Unprocessable Entity, joining every field error into one message so the
    // client sees all problems at once rather than one-at-a-time.
    let validation_errors = crate::validation::validate_worker(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Worker>::error(
            "VALIDATION_ERROR",
            format!(
                "Validation failed: {}",
                validation_errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        );
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error));
    }

    // Step 2 — assign a server-side UUID if the client did not supply one, so
    // the row (and the self-skip in dedup) has a stable identity.
    if payload.id == Uuid::nil() {
        payload.id = Uuid::new_v4();
    }

    // Step 3 — real-time duplicate detection BEFORE the insert. If any
    // candidate clears the review threshold we refuse the create with 409
    // Conflict and surface the matches in `error.details` so the operator can
    // review/merge instead of silently creating a near-duplicate record.
    let duplicates = check_duplicates_internal(&state, &payload).await;
    if !duplicates.is_empty() {
        // Carry the candidate matches in `error.details` so the client can
        // render them and decide whether to merge or force-create.
        let dup_response = DuplicateCheckResponse {
            has_duplicates: true,
            potential_matches: duplicates,
        };
        let details = serde_json::to_value(&dup_response).ok();
        let mut error = ApiResponse::<Worker>::error(
            "DUPLICATE_DETECTED",
            "Potential duplicate workers found. Review matches before proceeding.",
        );
        if let Some(ref mut err) = error.error {
            err.details = details;
        }
        return (StatusCode::CONFLICT, Json(error));
    }

    // Step 4 — persist. The repository INSERT also fans out the event publish
    // and audit-log write (wired in `AppState::new`), so a successful create
    // already emitted `WorkerCreated` and an audit row.
    match state.worker_repository.create(&payload).await {
        Ok(worker) => {
            // Step 5 — index in the search engine. A failure here is logged but
            // NOT fatal: the record exists in the DB and can be re-indexed
            // later, so we still return 201 rather than rolling back.
            if let Err(e) = state.search_engine.index_worker(&worker) {
                tracing::warn!("Failed to index worker in search engine: {}", e);
            }

            // 201 Created with the stored worker in the success envelope.
            (StatusCode::CREATED, Json(ApiResponse::success(worker)))
        }
        Err(e) => {
            // Any repository failure surfaces as 500 with a `DATABASE_ERROR` code.
            let error = ApiResponse::<Worker>::error(
                "DATABASE_ERROR",
                format!("Failed to create worker: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Fetches a single worker by UUID. `200` with the record, `404` if no
/// active worker has that id, `500` on a database error.
#[utoipa::path(
    get,
    path = "/api/v1/workers/{id}",
    tag = "workers",
    params(
        ("id" = Uuid, Path, description = "Worker UUID")
    ),
    responses(
        (status = 200, description = "Worker found"),
        (status = 404, description = "Worker not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_worker(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match state.worker_repository.get_by_id(&id).await {
        Ok(Some(worker)) => (StatusCode::OK, Json(ApiResponse::success(worker))),
        Ok(None) => {
            let error = ApiResponse::<Worker>::error(
                "NOT_FOUND",
                format!("Worker with id '{id}' not found"),
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Worker>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve worker: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Updates an existing worker. Validates the payload (`422` on failure),
/// forces the body's `id` to match the path id, persists the update, and
/// refreshes the search index (index failure is logged, not fatal).
#[utoipa::path(
    put,
    path = "/api/v1/workers/{id}",
    tag = "workers",
    params(
        ("id" = Uuid, Path, description = "Worker UUID")
    ),
    request_body = Worker,
    responses(
        (status = 200, description = "Worker updated successfully"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_worker(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut payload): Json<Worker>,
) -> impl IntoResponse {
    // Validate
    let validation_errors = crate::validation::validate_worker(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Worker>::error(
            "VALIDATION_ERROR",
            format!(
                "Validation failed: {}",
                validation_errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        );
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error));
    }

    // Force the body's id to the path id so the URL is authoritative and a
    // mismatched body id cannot retarget a different record.
    payload.id = id;

    match state.worker_repository.update(&payload).await {
        Ok(worker) => {
            // Refresh the search index; index failure is logged, not fatal
            // (the DB is the source of truth), so we still return 200.
            if let Err(e) = state.search_engine.index_worker(&worker) {
                tracing::warn!("Failed to update worker in search engine: {}", e);
            }

            (StatusCode::OK, Json(ApiResponse::success(worker)))
        }
        Err(e) => {
            let error = ApiResponse::<Worker>::error(
                "DATABASE_ERROR",
                format!("Failed to update worker: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Soft-deletes a worker (marks inactive; the row is retained for audit)
/// and removes it from the search index. Returns `204 No Content`.
#[utoipa::path(
    delete,
    path = "/api/v1/workers/{id}",
    tag = "workers",
    params(
        ("id" = Uuid, Path, description = "Worker UUID")
    ),
    responses(
        (status = 204, description = "Worker deleted successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_worker(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // The repository `delete` is a soft delete (marks the row inactive); the
    // row is retained for the audit trail rather than physically removed.
    match state.worker_repository.delete(&id).await {
        Ok(()) => {
            // Drop the worker from the search index so it stops appearing in
            // results; index failure is logged, not fatal.
            if let Err(e) = state.search_engine.delete_worker(&id.to_string()) {
                tracing::warn!("Failed to delete worker from search engine: {}", e);
            }

            // 204 No Content — success with an empty body.
            (StatusCode::NO_CONTENT, Json(ApiResponse::<()>::success(())))
        }
        Err(e) => {
            let error =
                ApiResponse::<()>::error("DATABASE_ERROR", format!("Failed to delete worker: {e}"));
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Query parameters for `/workers/search` (full-text + pagination + masking).
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,

    /// Maximum number of results (default: 10, max: 100)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,

    /// Use fuzzy search
    #[serde(default)]
    pub fuzzy: bool,

    /// Use phonetic (Soundex) search
    #[serde(default)]
    pub phonetic: bool,

    /// Mask sensitive data in response
    #[serde(default)]
    pub mask_sensitive: bool,
}

/// Serde default for [`SearchQuery::limit`] when the client omits it.
fn default_limit() -> usize {
    10
}

/// Body of a successful `/workers/search` response: the page of hits plus the
/// echoed-back pagination parameters.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    /// The workers on this page (already masked if `mask_sensitive` was set).
    pub workers: Vec<Worker>,
    /// Number of workers returned on this page (i.e. `workers.len()`).
    pub total: usize,
    /// The query string that was searched, echoed back.
    pub query: String,
    /// The pagination offset that was applied.
    pub offset: usize,
    /// The effective page size (capped at 100).
    pub limit: usize,
}

/// Full-text worker search with pagination and optional masking.
///
/// Caps `limit` at 100, asks the search engine for `offset + limit` ids
/// (fuzzy or exact per the query), then skips/takes for the requested page
/// and hydrates each id from the repository, optionally masking sensitive
/// fields. Ids present in the index but missing from the DB are skipped.
#[utoipa::path(
    get,
    path = "/api/v1/workers/search",
    tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 500, description = "Search error")
    )
)]
pub async fn search_workers(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    // Cap the page size so a client cannot request an unbounded result set.
    let limit = params.limit.min(100);

    // Ask the index for `offset + limit` ids: the engine returns ranked ids
    // from the top, and we apply the offset ourselves below by skip/take.
    let total_needed = params.offset + limit;
    let worker_ids = if params.fuzzy {
        state.search_engine.fuzzy_search(&params.q, total_needed)
    } else {
        state.search_engine.search(&params.q, total_needed)
    };

    match worker_ids {
        Ok(ids) => {
            // Slice out the requested page from the ranked id list.
            let paginated_ids: Vec<_> = ids.into_iter().skip(params.offset).take(limit).collect();

            // Hydrate each id from the repository (the index holds only ids).
            // Ids present in the index but missing from the DB are skipped
            // (logged), so a stale index entry never breaks the page.
            let mut workers = Vec::new();
            for worker_id_str in paginated_ids {
                let worker_id = match Uuid::parse_str(&worker_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse worker ID {}: {}", worker_id_str, e);
                        continue;
                    }
                };

                match state.worker_repository.get_by_id(&worker_id).await {
                    Ok(Some(worker)) => {
                        if params.mask_sensitive {
                            workers.push(crate::privacy::mask_worker(&worker));
                        } else {
                            workers.push(worker);
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            "Worker {} found in search index but not in database",
                            worker_id
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch worker {}: {}", worker_id, e);
                    }
                }
            }

            let response = SearchResponse {
                total: workers.len(),
                workers,
                query: params.q,
                offset: params.offset,
                limit,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error =
                ApiResponse::<SearchResponse>::error("SEARCH_ERROR", format!("Search failed: {e}"));
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Body of `/workers/match`: the probe worker plus optional scoring controls.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MatchRequest {
    /// Worker to match against existing records
    #[serde(flatten)]
    pub worker: Worker,

    /// Minimum match score threshold (0.0 to 1.0)
    #[serde(default)]
    pub threshold: Option<f64>,

    /// Maximum number of matches to return
    #[serde(default = "default_match_limit")]
    pub limit: usize,
}

/// Serde default for [`MatchRequest::limit`] when the client omits it.
fn default_match_limit() -> usize {
    10
}

/// A single scored candidate returned by the match / duplicate endpoints.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MatchResponse {
    /// The candidate worker that was scored against the probe.
    pub worker: Worker,
    /// Overall similarity score in `[0.0, 1.0]`.
    pub score: f64,
    /// Human-readable bucket: `"certain"` (≥0.95), `"probable"` (≥0.7),
    /// else `"possible"`.
    pub quality: String,
    /// How this candidate surfaced (e.g. `"probabilistic"`,
    /// `"duplicate_detection"`, `"batch_deduplication"`).
    pub detection_method: String,
    /// Per-component score breakdown as JSON, when available.
    pub score_breakdown: Option<serde_json::Value>,
}

/// Body of a successful `/workers/match` response: the scored candidates.
#[derive(Debug, Serialize, ToSchema)]
pub struct MatchResultsResponse {
    /// Candidates that met the threshold, best first, capped at `limit`.
    pub matches: Vec<MatchResponse>,
    /// Number of matches returned (i.e. `matches.len()`).
    pub total: usize,
}

/// Scores a probe worker against existing records.
///
/// Uses the search engine to fetch blocking candidates (by family name and
/// birth year), hydrates them, runs the matcher, filters by the requested
/// threshold (default 0.5), caps to `limit`, and labels each with a quality
/// bucket. This keeps scoring O(candidates) rather than O(all workers).
#[utoipa::path(
    post,
    path = "/api/v1/workers/match",
    tag = "matching",
    request_body = MatchRequest,
    responses(
        (status = 200, description = "Match results", body = MatchResultsResponse),
        (status = 500, description = "Matching error")
    )
)]
pub async fn match_worker(
    State(state): State<AppState>,
    Json(payload): Json<MatchRequest>,
) -> impl IntoResponse {
    // Blocking step: narrow the universe to candidates sharing family name +
    // birth year so the matcher scores O(candidates) records, not every worker.
    let family_name = &payload.worker.name.family;
    let birth_year = payload.worker.birth_date.map(|d| d.year());

    let candidate_ids = state
        .search_engine
        .search_by_name_and_year(family_name, birth_year, 100);

    match candidate_ids {
        Ok(ids) => {
            // Fetch full worker records from database
            let mut candidates = Vec::new();
            for worker_id_str in ids {
                let worker_id = match Uuid::parse_str(&worker_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse worker ID {}: {}", worker_id_str, e);
                        continue;
                    }
                };

                match state.worker_repository.get_by_id(&worker_id).await {
                    Ok(Some(worker)) => candidates.push(worker),
                    Ok(None) => {
                        tracing::warn!(
                            "Worker {} found in search index but not in database",
                            worker_id
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch worker {}: {}", worker_id, e);
                    }
                }
            }

            // Run matcher on candidates
            let match_results = match state.matcher.find_matches(&payload.worker, &candidates) {
                Ok(results) => results,
                Err(e) => {
                    let error = ApiResponse::<MatchResultsResponse>::error(
                        "MATCH_ERROR",
                        format!("Matching failed: {e}"),
                    );
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(error));
                }
            };

            // Keep only candidates at/above the requested threshold (default
            // 0.5), cap to `limit`, and label each with a quality bucket so the
            // caller can triage by confidence band.
            let threshold = payload.threshold.unwrap_or(0.5);
            let matches: Vec<MatchResponse> = match_results
                .into_iter()
                .filter(|m| m.score >= threshold)
                .take(payload.limit)
                .map(|m| {
                    // Quality bands: certain ≥0.95, probable ≥0.7, else possible.
                    let quality = if m.score >= 0.95 {
                        "certain"
                    } else if m.score >= 0.7 {
                        "probable"
                    } else {
                        "possible"
                    };

                    let breakdown_json = serde_json::to_value(&m.breakdown).ok();

                    MatchResponse {
                        worker: m.worker.clone(),
                        score: m.score,
                        quality: quality.to_string(),
                        detection_method: "probabilistic".to_string(),
                        score_breakdown: breakdown_json,
                    }
                })
                .collect();

            let response = MatchResultsResponse {
                total: matches.len(),
                matches,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error = ApiResponse::<MatchResultsResponse>::error(
                "MATCH_ERROR",
                format!("Matching failed: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

// ─── Duplicate Detection ────────────────────────────────────────────────────

/// Body of `/workers/check-duplicates` (and the `409` details on create).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateCheckResponse {
    /// `true` when at least one candidate scored at or above the review
    /// threshold (0.7).
    pub has_duplicates: bool,
    /// The candidate matches above threshold, best first.
    pub potential_matches: Vec<MatchResponse>,
}

/// Shared duplicate-detection core used by both [`create_worker`] (pre-insert
/// gate) and [`check_duplicates`] (explicit check). Blocks on family name +
/// birth year, skips the probe's own id, scores candidates, and returns those
/// at or above the 0.7 review threshold (capped at 10). Returns an empty vec
/// on any search/match error so a detection failure never blocks a create.
async fn check_duplicates_internal(state: &AppState, worker: &Worker) -> Vec<MatchResponse> {
    let family_name = &worker.name.family;
    let birth_year = worker.birth_date.map(|d| d.year());

    let Ok(candidate_ids) =
        state
            .search_engine
            .search_by_name_and_year(family_name, birth_year, 50)
    else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    for id_str in candidate_ids {
        if let Ok(pid) = Uuid::parse_str(&id_str) {
            if pid == worker.id {
                continue; // Skip self
            }
            if let Ok(Some(p)) = state.worker_repository.get_by_id(&pid).await {
                candidates.push(p);
            }
        }
    }

    // On a matcher error, return no duplicates rather than failing the caller:
    // a detection failure must never block a create.
    let Ok(match_results) = state.matcher.find_matches(worker, &candidates) else {
        return Vec::new();
    };

    // Surface only candidates at/above the 0.7 review threshold, best-first,
    // capped at 10 — these are what `create_worker` returns in its 409 details.
    match_results
        .into_iter()
        .filter(|m| m.score >= 0.7)
        .take(10)
        .map(|m| {
            let quality = if m.score >= 0.95 {
                "certain"
            } else if m.score >= 0.7 {
                "probable"
            } else {
                "possible"
            };

            MatchResponse {
                worker: m.worker.clone(),
                score: m.score,
                quality: quality.to_string(),
                detection_method: "duplicate_detection".to_string(),
                score_breakdown: serde_json::to_value(&m.breakdown).ok(),
            }
        })
        .collect()
}

/// Runs duplicate detection for a candidate worker without persisting it.
/// Always returns `200`; the body reports whether duplicates were found.
#[utoipa::path(
    post,
    path = "/api/v1/workers/check-duplicates",
    tag = "deduplication",
    request_body = Worker,
    responses(
        (status = 200, description = "Duplicate check results", body = DuplicateCheckResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(worker): Json<Worker>,
) -> impl IntoResponse {
    let matches = check_duplicates_internal(&state, &worker).await;
    let response = DuplicateCheckResponse {
        has_duplicates: !matches.is_empty(),
        potential_matches: matches,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Record Merging ─────────────────────────────────────────────────────────

/// Copies the duplicate's data into a clone of `main`, de-duping where it can,
/// and returns the merged worker plus a JSON snapshot of what was transferred.
///
/// The duplicate's primary name becomes an `Old` alias, and a `Replaces` link
/// to the duplicate is added.
fn transfer_worker_data(
    main: &Worker,
    duplicate: &Worker,
) -> (Worker, serde_json::Map<String, serde_json::Value>) {
    let mut merged = main.clone();
    let mut transferred = serde_json::Map::new();

    // Transfer identifiers not already present
    for id in &duplicate.identifiers {
        if !merged.identifiers.iter().any(|existing| {
            existing.value == id.value && existing.identifier_type == id.identifier_type
        }) {
            merged.identifiers.push(id.clone());
            let entry = transferred
                .entry("identifiers".to_string())
                .or_insert_with(|| serde_json::Value::Array(vec![]));
            if let Some(arr) = entry.as_array_mut() {
                arr.push(serde_json::to_value(id).unwrap_or_default());
            }
        }
    }

    // Transfer additional names
    for name in &duplicate.additional_names {
        merged.additional_names.push(name.clone());
    }
    // Add duplicate's primary name as an additional name (old/alias)
    let mut dup_name = duplicate.name.clone();
    dup_name.use_type = Some(crate::models::NameUse::Old);
    merged.additional_names.push(dup_name);

    // Transfer addresses not already present
    for addr in &duplicate.addresses {
        merged.addresses.push(addr.clone());
    }

    // Transfer contacts
    for cp in &duplicate.telecom {
        if !merged
            .telecom
            .iter()
            .any(|existing| existing.value == cp.value)
        {
            merged.telecom.push(cp.clone());
        }
    }

    // Transfer documents
    for doc in &duplicate.documents {
        if !merged.documents.iter().any(|existing| {
            existing.number == doc.number && existing.document_type == doc.document_type
        }) {
            merged.documents.push(doc.clone());
        }
    }

    // Transfer emergency contacts
    for ec in &duplicate.emergency_contacts {
        if !merged
            .emergency_contacts
            .iter()
            .any(|existing| existing.name == ec.name)
        {
            merged.emergency_contacts.push(ec.clone());
        }
    }

    // Transfer tax_id if main doesn't have one
    if merged.tax_id.is_none() && duplicate.tax_id.is_some() {
        merged.tax_id.clone_from(&duplicate.tax_id);
        transferred.insert(
            "tax_id".into(),
            serde_json::to_value(&duplicate.tax_id).unwrap_or_default(),
        );
    }

    // Add a link from main → replaces duplicate
    merged.links.push(crate::models::WorkerLink {
        other_worker_id: duplicate.id,
        link_type: crate::models::LinkType::Replaces,
    });

    (merged, transferred)
}

/// Merges a duplicate worker into a surviving main worker.
///
/// Fetches both (`404` if either is missing), copies the duplicate's
/// identifiers, names (its primary name becomes an `Old` alias), addresses,
/// contacts, documents, emergency contacts, and tax id into main (de-duping
/// where it can), adds a `Replaces` link, persists main, soft-deletes the
/// duplicate, updates the search index, publishes a `Merged` event, and
/// returns a [`crate::models::MergeRecord`] with a snapshot of transferred
/// data.
#[utoipa::path(
    post,
    path = "/api/v1/workers/merge",
    tag = "deduplication",
    request_body = crate::models::MergeRequest,
    responses(
        (status = 200, description = "Merge completed", body = crate::models::MergeResponse),
        (status = 404, description = "Worker not found"),
        (status = 500, description = "Merge error")
    )
)]
pub async fn merge_workers(
    State(state): State<AppState>,
    Json(req): Json<crate::models::MergeRequest>,
) -> impl IntoResponse {
    // Fetch both workers up front; a missing main or duplicate is a 404 (the
    // merge cannot proceed without both records).
    let main = match state.worker_repository.get_by_id(&req.main_worker_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "NOT_FOUND",
                    format!("Main worker {} not found", req.main_worker_id),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to fetch main worker: {e}"),
                )),
            );
        }
    };

    let duplicate = match state
        .worker_repository
        .get_by_id(&req.duplicate_worker_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "NOT_FOUND",
                    format!("Duplicate worker {} not found", req.duplicate_worker_id),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to fetch duplicate worker: {e}"),
                )),
            );
        }
    };

    // Merge data from duplicate into main, recording a snapshot of what moved.
    let (merged, transferred) = transfer_worker_data(&main, &duplicate);

    // Update main worker
    if let Err(e) = state.worker_repository.update(&merged).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR",
                format!("Failed to update main worker: {e}"),
            )),
        );
    }

    // Soft-delete the duplicate
    if let Err(e) = state.worker_repository.delete(&duplicate.id).await {
        tracing::error!("Failed to soft-delete duplicate worker: {}", e);
    }

    // Remove duplicate from search index
    if let Err(e) = state.search_engine.delete_worker(&duplicate.id.to_string()) {
        tracing::warn!("Failed to remove duplicate from search index: {}", e);
    }

    // Update search index for main
    if let Err(e) = state.search_engine.index_worker(&merged) {
        tracing::warn!("Failed to update search index for merged worker: {}", e);
    }

    // Publish a `Merged` event on the stream; `.ok()` ignores publish failure
    // so a streaming hiccup does not fail an already-committed merge.
    state
        .event_publisher
        .publish(crate::streaming::WorkerEvent::Merged {
            source_id: duplicate.id,
            target_id: merged.id,
            timestamp: chrono::Utc::now(),
        })
        .ok();

    // Record the merge with a snapshot of the transferred data for audit /
    // potential reversal, then return it (200) alongside the merged survivor.
    // Create merge record
    let merge_record = crate::models::MergeRecord {
        id: Uuid::new_v4(),
        main_worker_id: merged.id,
        duplicate_worker_id: duplicate.id,
        status: crate::models::MergeStatus::Completed,
        merged_by: req.merged_by,
        merge_reason: req.merge_reason,
        match_score: None,
        transferred_data: Some(serde_json::Value::Object(transferred)),
        merged_at: chrono::Utc::now(),
    };

    let response = crate::models::MergeResponse {
        merge_record,
        main_worker: merged,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Batch Deduplication ────────────────────────────────────────────────────

/// Scans all active workers pairwise for duplicates.
///
/// For each worker, blocks against the workers that follow it (so each pair is
/// considered once), scores them, and for pairs at or above `threshold`
/// records a [`crate::models::ReviewQueueItem`] — `AutoMerged` when the score
/// clears `auto_merge_threshold`, otherwise `Pending`. A `seen_pairs` set
/// guards against re-recording the same unordered pair.
#[utoipa::path(
    post,
    path = "/api/v1/workers/deduplicate",
    tag = "deduplication",
    request_body = crate::models::BatchDeduplicationRequest,
    responses(
        (status = 200, description = "Deduplication results", body = crate::models::BatchDeduplicationResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn batch_deduplicate(
    State(state): State<AppState>,
    Json(req): Json<crate::models::BatchDeduplicationRequest>,
) -> impl IntoResponse {
    // Get all active workers
    let workers = match state.worker_repository.list_active(1000, 0).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    ApiResponse::<crate::models::BatchDeduplicationResponse>::error(
                        "DATABASE_ERROR",
                        format!("Failed to list workers: {e}"),
                    ),
                ),
            );
        }
    };

    let workers_scanned = workers.len();
    let mut review_items = Vec::new();
    let mut auto_merged = 0usize;
    let mut seen_pairs: std::collections::HashSet<(Uuid, Uuid)> = std::collections::HashSet::new();

    for (i, worker) in workers.iter().enumerate() {
        // Compare each worker only with the ones that FOLLOW it, so every
        // unordered pair {a, b} is considered exactly once.
        let candidates: Vec<_> = workers[i + 1..]
            .iter()
            .take(req.max_candidates)
            .cloned()
            .collect();

        if candidates.is_empty() {
            continue;
        }

        let Ok(matches) = state.matcher.find_matches(worker, &candidates) else {
            continue;
        };

        for m in matches {
            if m.score < req.threshold {
                continue;
            }

            // Canonicalize the pair (smaller id first) and dedupe via the
            // `seen_pairs` set so the same pair is never queued twice even if
            // it surfaces from both directions of the blocking search.
            let pair = if worker.id < m.worker.id {
                (worker.id, m.worker.id)
            } else {
                (m.worker.id, worker.id)
            };

            if !seen_pairs.insert(pair) {
                continue;
            }

            let quality = if m.score >= 0.95 {
                "certain"
            } else if m.score >= 0.7 {
                "probable"
            } else {
                "possible"
            };

            // Pairs that clear `auto_merge_threshold` are flagged AutoMerged
            // (high enough confidence to merge without a human); the rest are
            // queued Pending for manual review.
            let status = if m.score >= req.auto_merge_threshold {
                auto_merged += 1;
                crate::models::ReviewStatus::AutoMerged
            } else {
                crate::models::ReviewStatus::Pending
            };

            review_items.push(crate::models::ReviewQueueItem {
                id: Uuid::new_v4(),
                worker_id_a: worker.id,
                worker_id_b: m.worker.id,
                match_score: m.score,
                match_quality: quality.to_string(),
                detection_method: "batch_deduplication".to_string(),
                score_breakdown: serde_json::to_value(&m.breakdown).ok(),
                status,
                reviewed_by: None,
                created_at: chrono::Utc::now(),
                reviewed_at: None,
            });
        }
    }

    let queued = review_items
        .iter()
        .filter(|r| r.status == crate::models::ReviewStatus::Pending)
        .count();

    let response = crate::models::BatchDeduplicationResponse {
        workers_scanned,
        duplicates_found: review_items.len(),
        auto_merged,
        queued_for_review: queued,
        review_items,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Data Export (GDPR Right of Access) ─────────────────────────────────────

/// Exports all stored data for a worker as JSON (GDPR right of access).
/// `200` with the export document, `404` if not found.
#[utoipa::path(
    get,
    path = "/api/v1/workers/{id}/export",
    tag = "privacy",
    params(
        ("id" = Uuid, Path, description = "Worker UUID")
    ),
    responses(
        (status = 200, description = "Worker data export"),
        (status = 404, description = "Worker not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn export_worker_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.worker_repository.get_by_id(&id).await {
        Ok(Some(worker)) => {
            let export = crate::privacy::export_worker_data(&worker);
            (StatusCode::OK, Json(ApiResponse::success(export)))
        }
        Ok(None) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "NOT_FOUND",
                format!("Worker with id '{id}' not found"),
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve worker: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Returns a worker with sensitive fields (tax id, document numbers, phone,
/// email, address) masked for least-privilege display. `404` if not found.
#[utoipa::path(
    get,
    path = "/api/v1/workers/{id}/masked",
    tag = "privacy",
    params(
        ("id" = Uuid, Path, description = "Worker UUID")
    ),
    responses(
        (status = 200, description = "Masked worker data"),
        (status = 404, description = "Worker not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_worker_masked(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.worker_repository.get_by_id(&id).await {
        Ok(Some(worker)) => {
            let masked = crate::privacy::mask_worker(&worker);
            (StatusCode::OK, Json(ApiResponse::success(masked)))
        }
        Ok(None) => {
            let error = ApiResponse::<Worker>::error(
                "NOT_FOUND",
                format!("Worker with id '{id}' not found"),
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Worker>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve worker: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

// ─── Audit Log Endpoints ────────────────────────────────────────────────────

/// Query parameters shared by the per-worker and recent audit endpoints.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AuditLogQuery {
    /// Maximum number of results (default: 50, max: 500)
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
}

/// Serde default for the audit-query `limit` fields when the client omits it.
fn default_audit_limit() -> i64 {
    50
}

/// Returns the audit trail for one worker, newest first (limit capped at 500).
#[utoipa::path(
    get,
    path = "/api/v1/workers/{id}/audit",
    tag = "audit",
    params(
        ("id" = Uuid, Path, description = "Worker UUID"),
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "Audit logs retrieved successfully"),
        (status = 500, description = "Database error")
    )
)]
pub async fn get_worker_audit_logs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    // Cap the page at 500 so an audit query cannot pull an unbounded history.
    let limit = params.limit.min(500);

    // `"Worker"` is the audit entity-type discriminator; logs are returned
    // newest-first by the repository.
    match state
        .audit_log
        .get_logs_for_entity("Worker", id, u64::try_from(limit).unwrap_or(0))
        .await
    {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => {
            let error = ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve audit logs: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Returns the most recent audit entries system-wide (limit capped at 500).
#[utoipa::path(
    get,
    path = "/api/v1/audit/recent",
    tag = "audit",
    params(AuditLogQuery),
    responses(
        (status = 200, description = "Recent audit logs retrieved successfully"),
        (status = 500, description = "Database error")
    )
)]
pub async fn get_recent_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);

    match state
        .audit_log
        .get_recent_logs(u64::try_from(limit).unwrap_or(0))
        .await
    {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => {
            let error = ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve audit logs: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Query parameters for the by-user audit endpoint.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct UserAuditLogQuery {
    /// User ID to filter by
    pub user_id: String,

    /// Maximum number of results (default: 50, max: 500)
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
}

/// Returns audit entries performed by a given user (limit capped at 500).
#[utoipa::path(
    get,
    path = "/api/v1/audit/user",
    tag = "audit",
    params(UserAuditLogQuery),
    responses(
        (status = 200, description = "User audit logs retrieved successfully"),
        (status = 500, description = "Database error")
    )
)]
pub async fn get_user_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<UserAuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);

    match state
        .audit_log
        .get_logs_by_user(&params.user_id, u64::try_from(limit).unwrap_or(0))
        .await
    {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => {
            let error = ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve audit logs: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}
