//! Axum handler functions and their request/response DTOs.
//!
//! Each handler takes [`AppState`] plus extracted path/query/JSON inputs
//! and returns an [`ApiResponse`](crate::api::ApiResponse)-wrapped body with an appropriate HTTP
//! status. The flow follows the create/match/merge/search pipelines:
//! validation, real-time duplicate detection, persistence via the
//! repository, and search-index synchronization. The `#[utoipa::path]`
//! attributes feed [`ApiDoc`](crate::api::rest::ApiDoc) for OpenAPI/Swagger.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use authentication_verifier::Action;

use super::auth::{MaybeAuthUser, authorize_record, person_resource_attrs, read_visibility};
use super::state::AppState;
use crate::api::ApiResponse;
use crate::compliance::disclosure::{self, AccessContext};
use crate::models::Person;

/// Body returned by the health-check endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Liveness status string (always `"healthy"` when reachable).
    pub status: String,
    /// Service name (`"person-service"`).
    pub service: String,
    /// Crate version from `CARGO_PKG_VERSION`.
    pub version: String,
}

/// Liveness probe; returns a static [`HealthResponse`].
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "person-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Prometheus metrics endpoint (text-exposition format).
///
/// Renders [`crate::metrics::METRICS`] for scraping. Mounted at the
/// root (`/metrics.prom`) — not under `/api` — so a default
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

/// Create-person request body (a flattened [`Person`]).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePersonRequest {
    /// The person to create; flattened so the JSON is a bare Person.
    #[serde(flatten)]
    pub person: Person,
}

/// Validate, duplicate-check, persist, and index a new person.
///
/// Returns `422` on validation failure, `409` with candidate matches if
/// real-time duplicate detection fires, `201` on success.
#[utoipa::path(
    post,
    path = "/api/persons",
    tag = "persons",
    request_body = Person,
    responses(
        (status = 201, description = "Person created successfully"),
        (status = 409, description = "Potential duplicates detected"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_person(
    State(state): State<AppState>,
    Json(mut payload): Json<Person>,
) -> impl IntoResponse {
    // Validate person data
    let validation_errors = crate::validation::validate_person(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Person>::error(
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

    // Ensure person has a UUID
    if payload.id == Uuid::nil() {
        payload.id = Uuid::new_v4();
    }

    // Real-time duplicate detection before creation
    let duplicates = check_duplicates_internal(&state, &payload).await;
    if !duplicates.is_empty() {
        let dup_response = DuplicateCheckResponse {
            has_duplicates: true,
            potential_matches: duplicates,
        };
        let details = serde_json::to_value(&dup_response).ok();
        let mut error = ApiResponse::<Person>::error(
            "DUPLICATE_DETECTED",
            "Potential duplicate persons found. Review matches before proceeding.",
        );
        if let Some(ref mut err) = error.error {
            err.details = details;
        }
        return (StatusCode::CONFLICT, Json(error));
    }

    // Insert into database
    match state.person_repository.create(&payload).await {
        Ok(person) => {
            // Index in search engine
            if let Err(e) = state.search_engine.index_person(&person) {
                tracing::warn!("Failed to index person in search engine: {}", e);
            }

            (StatusCode::CREATED, Json(ApiResponse::success(person)))
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to create person: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get a person by ID
#[utoipa::path(
    get,
    path = "/api/persons/{id}",
    tag = "persons",
    params(
        ("id" = Uuid, Path, description = "Person UUID")
    ),
    responses(
        (status = 200, description = "Person found"),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            // Record-level ABAC: gate the read on the specific record's
            // attributes; honour a `mask` obligation by masking.
            match authorize_record(&caller, Action::Read, &person_resource_attrs(&person)) {
                Ok(obligations) => {
                    // Audited only once authorization has allowed the
                    // read: a denied request disclosed nothing, and
                    // recording it would pollute the §164.528 accounting
                    // with accesses that never happened.
                    if disclosure::record_access(
                        &state.audit_log,
                        "Person",
                        id,
                        disclosure::action::READ,
                        caller.claims().map(|c| c.sub.as_str()),
                        &access,
                    )
                    .await
                    .is_err()
                    {
                        return audit_unavailable::<Person>();
                    }
                    let body = if obligations.iter().any(|o| o == "mask") {
                        crate::privacy::mask_person(&person)
                    } else {
                        person
                    };
                    (StatusCode::OK, Json(ApiResponse::success(body)))
                }
                Err((status, reason)) => (
                    status,
                    Json(ApiResponse::<Person>::error("FORBIDDEN", reason)),
                ),
            }
        }
        Ok(None) => {
            let error = ApiResponse::<Person>::error(
                "NOT_FOUND",
                format!("Person with id '{id}' not found"),
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Update a person
#[utoipa::path(
    put,
    path = "/api/persons/{id}",
    tag = "persons",
    params(
        ("id" = Uuid, Path, description = "Person UUID")
    ),
    request_body = Person,
    responses(
        (status = 200, description = "Person updated successfully"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    Json(mut payload): Json<Person>,
) -> impl IntoResponse {
    // Validate
    let validation_errors = crate::validation::validate_person(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Person>::error(
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

    // Ensure ID in path matches payload
    payload.id = id;

    // Record-level ABAC on the *stored* record (e.g. deny modifying a
    // deceased person's record). Load it first for the resource attrs.
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(existing)) => {
            if let Err((status, reason)) =
                authorize_record(&caller, Action::Write, &person_resource_attrs(&existing))
            {
                return (
                    status,
                    Json(ApiResponse::<Person>::error("FORBIDDEN", reason)),
                );
            }
        }
        Ok(None) => {
            let error = ApiResponse::<Person>::error(
                "NOT_FOUND",
                format!("Person with id '{id}' not found"),
            );
            return (StatusCode::NOT_FOUND, Json(error));
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {e}"),
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error));
        }
    }

    match state.person_repository.update(&payload).await {
        Ok(person) => {
            // Update search index
            if let Err(e) = state.search_engine.index_person(&person) {
                tracing::warn!("Failed to update person in search engine: {}", e);
            }

            (StatusCode::OK, Json(ApiResponse::success(person)))
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to update person: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Delete a person (soft delete)
#[utoipa::path(
    delete,
    path = "/api/persons/{id}",
    tag = "persons",
    params(
        ("id" = Uuid, Path, description = "Person UUID")
    ),
    responses(
        (status = 204, description = "Person deleted successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
) -> impl IntoResponse {
    // Record-level ABAC on the stored record before deleting.
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            if let Err((status, reason)) =
                authorize_record(&caller, Action::Delete, &person_resource_attrs(&person))
            {
                return (status, Json(ApiResponse::<()>::error("FORBIDDEN", reason)));
            }
        }
        Ok(None) => {
            let error =
                ApiResponse::<()>::error("NOT_FOUND", format!("Person with id '{id}' not found"));
            return (StatusCode::NOT_FOUND, Json(error));
        }
        Err(e) => {
            let error = ApiResponse::<()>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {e}"),
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(error));
        }
    }

    match state.person_repository.delete(&id).await {
        Ok(()) => {
            // Remove from search index
            if let Err(e) = state.search_engine.delete_person(&id.to_string()) {
                tracing::warn!("Failed to delete person from search engine: {}", e);
            }

            (StatusCode::NO_CONTENT, Json(ApiResponse::<()>::success(())))
        }
        Err(e) => {
            let error =
                ApiResponse::<()>::error("DATABASE_ERROR", format!("Failed to delete person: {e}"));
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Search query parameters
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

/// Default search result limit (serde default for [`SearchQuery::limit`]).
fn default_limit() -> usize {
    10
}

/// SEC-G7 — the maximum accepted pagination `offset` for
/// [`search_persons`]. The search engine is asked for `offset + limit`
/// hits, so an unbounded `offset` would force the index to collect
/// arbitrarily many results (a CPU/memory `DoS`). Deep pagination beyond
/// this is unsupported — a caller that needs it should narrow the query
/// (cursor pagination is the correct tool). 10 000 is far past any real UI
/// page depth.
const MAX_SEARCH_OFFSET: usize = 10_000;

/// SEC-G7 — is a pagination `offset` within the accepted bound? Pure, so
/// the handler and its tests share one definition.
fn search_offset_within_bound(offset: usize) -> bool {
    offset <= MAX_SEARCH_OFFSET
}

/// SEC-G3 — how a single search hit should appear in the page, given the
/// caller's record-level read visibility and the client's `mask_sensitive`
/// request. Pure, so the concealment/masking decision is unit-tested apart
/// from the DB/search machinery.
#[derive(Debug, PartialEq, Eq)]
enum ResultDisposition {
    /// The caller may not read this record: omit it from the page so its
    /// existence is never revealed (concealment).
    Omit,
    /// Include the record, but masked (a `mask` obligation or the client's
    /// `mask_sensitive` request).
    Masked,
    /// Include the full record.
    Full,
}

/// Decide a search hit's disposition. `visibility` is
/// [`read_visibility`]'s result: `None` ⇒ denied (omit); `Some(obligations)`
/// ⇒ readable, masked if an obligation is `mask` **or** the client asked to
/// mask. When `PERSON_REQUIRE_AUTH` is off, `read_visibility` yields
/// `Some(vec![])`, so this collapses to "mask iff the client asked" —
/// exactly the pre-SEC-G3 behaviour.
fn search_result_disposition(
    visibility: Option<&[String]>,
    mask_sensitive: bool,
) -> ResultDisposition {
    match visibility {
        None => ResultDisposition::Omit,
        Some(obligations) => {
            if mask_sensitive || obligations.iter().any(|o| o == "mask") {
                ResultDisposition::Masked
            } else {
                ResultDisposition::Full
            }
        }
    }
}

/// Paginated search results body.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    /// The matched persons for this page (possibly masked).
    pub persons: Vec<Person>,
    /// Count of persons in this page (not the global total).
    pub total: usize,
    /// Echo of the query string searched.
    pub query: String,
    /// Offset applied for pagination.
    pub offset: usize,
    /// Page size applied (capped at 100).
    pub limit: usize,
}

/// Search for persons
#[utoipa::path(
    get,
    path = "/api/persons/search",
    tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 500, description = "Search error")
    )
)]
pub async fn search_persons(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> impl IntoResponse {
    // Limit to max 100 results
    let limit = params.limit.min(100);

    // SEC-G7: reject an out-of-bound pagination offset before asking the
    // search engine for `offset + limit` hits — an unbounded offset would
    // force the index to materialise arbitrarily many results (CPU/memory
    // DoS), and the addition below could also overflow.
    if !search_offset_within_bound(params.offset) {
        let error = ApiResponse::<SearchResponse>::error(
            "OFFSET_TOO_LARGE",
            format!("offset must not exceed {MAX_SEARCH_OFFSET}; narrow the query instead"),
        );
        return (StatusCode::BAD_REQUEST, Json(error));
    }

    // A search is a collection read: recorded against the nil id, because
    // it disclosed many records rather than one, and attributing it to any
    // single person would corrupt that person's §164.528 accounting.
    if disclosure::record_access(
        &state.audit_log,
        "Person",
        Uuid::nil(),
        disclosure::action::SEARCH,
        caller.claims().map(|c| c.sub.as_str()),
        &access,
    )
    .await
    .is_err()
    {
        return audit_unavailable::<SearchResponse>();
    }

    // Perform search using search engine
    // Request more results to handle pagination offset. `saturating_add`
    // belts-and-braces the (already bounded) offset against overflow.
    let total_needed = params.offset.saturating_add(limit);
    let person_ids = if params.fuzzy {
        state.search_engine.fuzzy_search(&params.q, total_needed)
    } else {
        state.search_engine.search(&params.q, total_needed)
    };

    match person_ids {
        Ok(ids) => {
            // Apply offset and limit
            let paginated_ids: Vec<_> = ids.into_iter().skip(params.offset).take(limit).collect();

            // Fetch full person records from database
            let mut persons = Vec::new();
            for person_id_str in paginated_ids {
                let person_id = match Uuid::parse_str(&person_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse person ID {}: {}", person_id_str, e);
                        continue;
                    }
                };

                match state.person_repository.get_by_id(&person_id).await {
                    Ok(Some(person)) => {
                        // SEC-G3: record-level read authz on every result, so
                        // an aggregate read never reveals more than the
                        // equivalent single `GET`. A denied record is
                        // **omitted** (concealed) — an unauthorised caller
                        // never learns it exists; a `mask` obligation (or the
                        // client's `mask_sensitive` param) returns the masked
                        // view. No-op when `PERSON_REQUIRE_AUTH` is off, so
                        // this preserves today's behaviour until enforcement
                        // is switched on.
                        let visibility = read_visibility(&caller, &person);
                        match search_result_disposition(
                            visibility.as_deref(),
                            params.mask_sensitive,
                        ) {
                            ResultDisposition::Omit => {} // concealed
                            ResultDisposition::Masked => {
                                persons.push(crate::privacy::mask_person(&person));
                            }
                            ResultDisposition::Full => persons.push(person),
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            "Person {} found in search index but not in database",
                            person_id
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch person {}: {}", person_id, e);
                    }
                }
            }

            let response = SearchResponse {
                total: persons.len(),
                persons,
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

/// Match request payload: a probe person plus filter options.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MatchRequest {
    /// Person to match against existing records
    #[serde(flatten)]
    pub person: Person,

    /// Minimum match score threshold (0.0 to 1.0)
    #[serde(default)]
    pub threshold: Option<f64>,

    /// Maximum number of matches to return
    #[serde(default = "default_match_limit")]
    pub limit: usize,
}

/// Default match-result limit (serde default for [`MatchRequest::limit`]).
fn default_match_limit() -> usize {
    10
}

/// One scored candidate in a match/duplicate-check response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MatchResponse {
    /// The candidate person record.
    pub person: Person,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Human label: `"certain"` / `"probable"` / `"possible"`.
    pub quality: String,
    /// How the match was produced (e.g. `"probabilistic"`).
    pub detection_method: String,
    /// Optional per-component score breakdown as JSON.
    pub score_breakdown: Option<serde_json::Value>,
}

/// Wrapper holding all matches for a match request.
#[derive(Debug, Serialize, ToSchema)]
pub struct MatchResultsResponse {
    /// Scored candidates above the threshold, capped at the limit.
    pub matches: Vec<MatchResponse>,
    /// Count of returned matches.
    pub total: usize,
}

/// Match a person against existing records
#[utoipa::path(
    post,
    path = "/api/persons/match",
    tag = "matching",
    request_body = MatchRequest,
    responses(
        (status = 200, description = "Match results", body = MatchResultsResponse),
        (status = 500, description = "Matching error")
    )
)]
pub async fn match_person(
    State(state): State<AppState>,
    Json(payload): Json<MatchRequest>,
) -> impl IntoResponse {
    // Use search engine to get candidate persons (blocking)
    let family_name = &payload.person.name.family;
    let birth_year = payload.person.birth_date.map(|d| d.year());

    let candidate_ids = state
        .search_engine
        .search_by_name_and_year(family_name, birth_year, 100);

    match candidate_ids {
        Ok(ids) => {
            // Fetch full person records from database
            let mut candidates = Vec::new();
            for person_id_str in ids {
                let person_id = match Uuid::parse_str(&person_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse person ID {}: {}", person_id_str, e);
                        continue;
                    }
                };

                match state.person_repository.get_by_id(&person_id).await {
                    Ok(Some(person)) => candidates.push(person),
                    Ok(None) => {
                        tracing::warn!(
                            "Person {} found in search index but not in database",
                            person_id
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch person {}: {}", person_id, e);
                    }
                }
            }

            // Run matcher on candidates
            let match_results = match state.matcher.find_matches(&payload.person, &candidates) {
                Ok(results) => results,
                Err(e) => {
                    let error = ApiResponse::<MatchResultsResponse>::error(
                        "MATCH_ERROR",
                        format!("Matching failed: {e}"),
                    );
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(error));
                }
            };

            // Filter by threshold if provided
            let threshold = payload.threshold.unwrap_or(0.5);
            let matches: Vec<MatchResponse> = match_results
                .into_iter()
                .filter(|m| m.score >= threshold)
                .take(payload.limit)
                .map(|m| {
                    let quality = if m.score >= 0.95 {
                        "certain"
                    } else if m.score >= 0.7 {
                        "probable"
                    } else {
                        "possible"
                    };

                    let breakdown_json = serde_json::to_value(&m.breakdown).ok();

                    MatchResponse {
                        person: m.person.clone(),
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

/// Result of a duplicate check.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateCheckResponse {
    /// `true` if any candidate scored above the review threshold.
    pub has_duplicates: bool,
    /// The candidates that triggered the flag (score ≥ 0.7).
    pub potential_matches: Vec<MatchResponse>,
}

/// Shared duplicate-detection core for `create_person` and
/// `check_duplicates`.
///
/// Blocks on the search index by family name + birth year, excludes the
/// probe's own id, scores candidates with the matcher, and returns those
/// at or above the 0.7 review threshold (max 10). Errors degrade to an
/// empty result rather than failing the caller.
async fn check_duplicates_internal(state: &AppState, person: &Person) -> Vec<MatchResponse> {
    let family_name = &person.name.family;
    let birth_year = person.birth_date.map(|d| d.year());

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
            if pid == person.id {
                continue; // Skip self
            }
            if let Ok(Some(p)) = state.person_repository.get_by_id(&pid).await {
                candidates.push(p);
            }
        }
    }

    let Ok(match_results) = state.matcher.find_matches(person, &candidates) else {
        return Vec::new();
    };

    // Return matches above the auto-review threshold (0.7)
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
                person: m.person.clone(),
                score: m.score,
                quality: quality.to_string(),
                detection_method: "duplicate_detection".to_string(),
                score_breakdown: serde_json::to_value(&m.breakdown).ok(),
            }
        })
        .collect()
}

/// Check for duplicates without creating a person
#[utoipa::path(
    post,
    path = "/api/persons/check-duplicates",
    tag = "deduplication",
    request_body = Person,
    responses(
        (status = 200, description = "Duplicate check results", body = DuplicateCheckResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(person): Json<Person>,
) -> impl IntoResponse {
    let matches = check_duplicates_internal(&state, &person).await;
    let response = DuplicateCheckResponse {
        has_duplicates: !matches.is_empty(),
        potential_matches: matches,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Record Merging ─────────────────────────────────────────────────────────

/// Fold the `duplicate` record's data into a clone of `main`.
///
/// Transfers non-duplicate identifiers, additional names (plus the
/// duplicate's primary name as an `Old` alias), addresses, contacts,
/// documents, emergency contacts, and `tax_id`, and appends a `Replaces`
/// link. Returns the merged person and a JSON map snapshot of the
/// transferred identifiers / tax id.
fn merge_duplicate_into_main(
    main: &Person,
    duplicate: &Person,
) -> (Person, serde_json::Map<String, serde_json::Value>) {
    let mut merged = main.clone();
    let mut transferred = serde_json::Map::new();

    // Transfer identifiers not already present
    for id in &duplicate.identifiers {
        if !merged.identifiers.iter().any(|existing| {
            existing.value == id.value && existing.identifier_type == id.identifier_type
        }) {
            merged.identifiers.push(id.clone());
            transferred
                .entry("identifiers".to_string())
                .or_insert_with(|| serde_json::Value::Array(vec![]))
                .as_array_mut()
                .expect("identifiers entry was just inserted as an array")
                .push(serde_json::to_value(id).unwrap_or_default());
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
    merged.links.push(crate::models::PersonLink {
        other_person_id: duplicate.id,
        link_type: crate::models::LinkType::Replaces,
    });

    (merged, transferred)
}

/// Merge two person records
#[utoipa::path(
    post,
    path = "/api/persons/merge",
    tag = "deduplication",
    request_body = crate::models::MergeRequest,
    responses(
        (status = 200, description = "Merge completed", body = crate::models::MergeResponse),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Merge error")
    )
)]
/// Fold a duplicate record into a main record.
///
/// Transfers non-duplicate identifiers, names (the duplicate's primary
/// name becomes an `Old` alias), addresses, contacts, documents,
/// emergency contacts, and tax id; adds a `Replaces` link; updates and
/// re-indexes main; soft-deletes and de-indexes the duplicate; publishes
/// a `Merged` event; and returns a [`MergeRecord`](crate::models::MergeRecord)
/// snapshot of what was transferred. `404` if either id is missing.
pub async fn merge_persons(
    State(state): State<AppState>,
    Json(req): Json<crate::models::MergeRequest>,
) -> impl IntoResponse {
    // SEC-B5: a record cannot be merged into itself. Without this guard,
    // `main == duplicate` applies the survivor and then soft-deletes the
    // *same* id, tombstoning the record and destroying its data. Reject
    // before any fetch (mirrors the case service's equal-pid `422`).
    if req.main_person_id == req.duplicate_person_id {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::<crate::models::MergeResponse>::error(
                "INVALID_MERGE",
                "main_person_id and duplicate_person_id must differ".to_string(),
            )),
        );
    }

    // Fetch both persons
    let main = match state.person_repository.get_by_id(&req.main_person_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "NOT_FOUND",
                    format!("Main person {} not found", req.main_person_id),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to fetch main person: {e}"),
                )),
            );
        }
    };

    let duplicate = match state
        .person_repository
        .get_by_id(&req.duplicate_person_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "NOT_FOUND",
                    format!("Duplicate person {} not found", req.duplicate_person_id),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<crate::models::MergeResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to fetch duplicate person: {e}"),
                )),
            );
        }
    };

    // Merge data from duplicate into main
    let (merged, transferred) = merge_duplicate_into_main(&main, &duplicate);

    // Apply the merge atomically: the survivor's row updates and the
    // duplicate's soft-delete commit in one transaction, and (under the
    // outbox transport) a `Merged` + `Deleted` outbox row are enqueued on
    // that same transaction. The repository also publishes the in-memory
    // `Merged`/`Deleted` events, so the handler no longer publishes them.
    if let Err(e) = state.person_repository.merge(&merged, &duplicate.id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR",
                format!("Failed to merge persons: {e}"),
            )),
        );
    }

    // Remove duplicate from search index
    if let Err(e) = state.search_engine.delete_person(&duplicate.id.to_string()) {
        tracing::warn!("Failed to remove duplicate from search index: {}", e);
    }

    // Update search index for main
    if let Err(e) = state.search_engine.index_person(&merged) {
        tracing::warn!("Failed to update search index for merged person: {}", e);
    }

    // Create merge record
    let merge_record = crate::models::MergeRecord {
        id: Uuid::new_v4(),
        main_person_id: merged.id,
        duplicate_person_id: duplicate.id,
        status: crate::models::MergeStatus::Completed,
        merged_by: req.merged_by,
        merge_reason: req.merge_reason,
        match_score: None,
        transferred_data: Some(serde_json::Value::Object(transferred)),
        merged_at: Utc::now(),
    };

    let response = crate::models::MergeResponse {
        merge_record,
        main_person: merged,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Batch Deduplication ────────────────────────────────────────────────────

/// Run batch deduplication across all persons
#[utoipa::path(
    post,
    path = "/api/persons/deduplicate",
    tag = "deduplication",
    request_body = crate::models::BatchDeduplicationRequest,
    responses(
        (status = 200, description = "Deduplication results", body = crate::models::BatchDeduplicationResponse),
        (status = 500, description = "Internal server error")
    )
)]
/// Scan active persons pairwise and queue likely duplicates for review.
///
/// For each person, compares against the subsequent records (upper
/// triangle, so each unordered pair is scored once via `seen_pairs`).
/// Pairs at/above `auto_merge_threshold` are marked `AutoMerged`; those
/// at/above `threshold` are queued `Pending`. Does not itself merge —
/// it only produces review-queue items.
#[allow(clippy::too_many_lines)] // linear scan + persist + report walk
pub async fn batch_deduplicate(
    State(state): State<AppState>,
    Json(req): Json<crate::models::BatchDeduplicationRequest>,
) -> impl IntoResponse {
    // Get all active persons
    let persons = match state.person_repository.list_active(1000, 0).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    ApiResponse::<crate::models::BatchDeduplicationResponse>::error(
                        "DATABASE_ERROR",
                        format!("Failed to list persons: {e}"),
                    ),
                ),
            );
        }
    };

    let persons_scanned = persons.len();
    let mut review_items = Vec::new();
    let mut seen_pairs: std::collections::HashSet<(Uuid, Uuid)> = std::collections::HashSet::new();

    for (i, person) in persons.iter().enumerate() {
        // Compare with subsequent persons to avoid duplicate pairs
        let candidates: Vec<_> = persons[i + 1..]
            .iter()
            .take(req.max_candidates)
            .cloned()
            .collect();

        if candidates.is_empty() {
            continue;
        }

        let Ok(matches) = state.matcher.find_matches(person, &candidates) else {
            continue;
        };

        for m in matches {
            if m.score < req.threshold {
                continue;
            }

            // Normalize pair order to avoid duplicates
            let pair = if person.id < m.person.id {
                (person.id, m.person.id)
            } else {
                (m.person.id, person.id)
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

            let status = if m.score >= req.auto_merge_threshold {
                crate::models::ReviewStatus::AutoMerged
            } else {
                crate::models::ReviewStatus::Pending
            };

            review_items.push(crate::models::ReviewQueueItem {
                id: Uuid::new_v4(),
                person_id_a: person.id,
                person_id_b: m.person.id,
                match_score: m.score,
                match_quality: quality.to_string(),
                detection_method: "batch_deduplication".to_string(),
                score_breakdown: serde_json::to_value(&m.breakdown).ok(),
                status,
                reviewed_by: None,
                created_at: Utc::now(),
                reviewed_at: None,
            });
        }
    }

    // Persist the scan into the stored review queue: normalized-pair
    // upsert refreshes scores on re-scan while a decided row keeps its
    // decision (`status` is never touched on conflict). The response
    // reports the STORED rows, so item ids are stable across scans and
    // prior decisions show through.
    let new_items: Vec<crate::db::review_queue::NewReviewItem> = review_items
        .iter()
        .map(|r| crate::db::review_queue::NewReviewItem {
            record_id_a: r.person_id_a,
            record_id_b: r.person_id_b,
            match_score: r.match_score,
            match_quality: r.match_quality.clone(),
            detection_method: r.detection_method.clone(),
            score_breakdown: r.score_breakdown.clone(),
            status: review_status_token(&r.status).to_string(),
        })
        .collect();
    let rows = match crate::db::review_queue::upsert(&state.db, &new_items).await {
        Ok(rows) => rows,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    ApiResponse::<crate::models::BatchDeduplicationResponse>::error(
                        "DATABASE_ERROR",
                        format!("Failed to persist review queue: {e}"),
                    ),
                ),
            );
        }
    };
    let review_items: Vec<crate::models::ReviewQueueItem> =
        rows.iter().map(review_row_to_item).collect();
    let auto_merged = review_items
        .iter()
        .filter(|r| r.status == crate::models::ReviewStatus::AutoMerged)
        .count();
    let queued = review_items
        .iter()
        .filter(|r| r.status == crate::models::ReviewStatus::Pending)
        .count();

    let response = crate::models::BatchDeduplicationResponse {
        persons_scanned,
        duplicates_found: review_items.len(),
        auto_merged,
        queued_for_review: queued,
        review_items,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Review queue (stored) ──────────────────────────────────────────────────

/// The lowercase wire token for a review status.
fn review_status_token(status: &crate::models::ReviewStatus) -> &'static str {
    match status {
        crate::models::ReviewStatus::Pending => "pending",
        crate::models::ReviewStatus::Confirmed => "confirmed",
        crate::models::ReviewStatus::Rejected => "rejected",
        crate::models::ReviewStatus::AutoMerged => "automerged",
    }
}

/// Parse a stored status token (unknown tokens read as `pending`).
fn parse_review_status(token: &str) -> crate::models::ReviewStatus {
    match token {
        "confirmed" => crate::models::ReviewStatus::Confirmed,
        "rejected" => crate::models::ReviewStatus::Rejected,
        "automerged" => crate::models::ReviewStatus::AutoMerged,
        _ => crate::models::ReviewStatus::Pending,
    }
}

/// Map a stored review-queue row onto the wire item shape.
fn review_row_to_item(
    row: &crate::db::review_queue::ReviewQueueRow,
) -> crate::models::ReviewQueueItem {
    crate::models::ReviewQueueItem {
        id: row.id,
        person_id_a: row.record_id_a,
        person_id_b: row.record_id_b,
        match_score: row.match_score,
        match_quality: row.match_quality.clone(),
        detection_method: row.detection_method.clone(),
        score_breakdown: row.score_breakdown.clone(),
        status: parse_review_status(&row.status),
        reviewed_by: row.reviewed_by.clone(),
        created_at: crate::db::convert::offset_to_ts(row.created_at),
        reviewed_at: row.reviewed_at.map(crate::db::convert::offset_to_ts),
    }
}

/// Query parameters for the review-queue list endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct ReviewQueueListQuery {
    /// Optional status-token filter (`pending` / `confirmed` /
    /// `rejected` / `automerged`).
    pub status: Option<String>,
    /// Maximum items to return (default 100, capped at 500).
    pub limit: Option<u64>,
}

/// List the stored deduplication review queue (newest first).
#[utoipa::path(
    get,
    path = "/api/persons/review-queue",
    tag = "deduplication",
    responses(
        (status = 200, description = "Stored review-queue items", body = crate::models::ReviewQueueListResponse),
        (status = 422, description = "Unknown status token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_review_queue(
    State(state): State<AppState>,
    Query(query): Query<ReviewQueueListQuery>,
) -> axum::response::Response {
    if let Some(status) = query.status.as_deref()
        && !matches!(status, "pending" | "confirmed" | "rejected" | "automerged")
    {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                ApiResponse::<crate::models::ReviewQueueListResponse>::error(
                    "INVALID_STATUS",
                    format!("unknown review status `{status}`"),
                ),
            ),
        )
            .into_response();
    }
    match crate::db::review_queue::list(
        &state.db,
        query.status.as_deref(),
        query.limit.unwrap_or(100),
    )
    .await
    {
        Ok(rows) => {
            let items: Vec<crate::models::ReviewQueueItem> =
                rows.iter().map(review_row_to_item).collect();
            let total = items.len();
            (
                StatusCode::OK,
                Json(ApiResponse::success(
                    crate::models::ReviewQueueListResponse { items, total },
                )),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                ApiResponse::<crate::models::ReviewQueueListResponse>::error(
                    "DATABASE_ERROR",
                    format!("Failed to list review queue: {e}"),
                ),
            ),
        )
            .into_response(),
    }
}

/// Decide one `pending` review item (`confirmed` or `rejected`).
///
/// The transition guard is first-writer-wins in SQL: only a `pending`
/// item can be decided; an already-decided item returns `422`, an
/// unknown id `404`.
#[utoipa::path(
    post,
    path = "/api/persons/review-queue/{id}/decision",
    tag = "deduplication",
    request_body = crate::models::ReviewDecisionRequest,
    responses(
        (status = 200, description = "The decided item", body = crate::models::ReviewQueueItem),
        (status = 404, description = "No review item with that id"),
        (status = 422, description = "Item already decided"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn review_decision(
    State(state): State<AppState>,
    caller: MaybeAuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<crate::models::ReviewDecisionRequest>,
) -> axum::response::Response {
    let token = match req.status {
        crate::models::ReviewDecision::Confirmed => "confirmed",
        crate::models::ReviewDecision::Rejected => "rejected",
    };
    let reviewed_by = caller.claims().map(|c| c.sub.clone());
    match crate::db::review_queue::decide(&state.db, id, token, reviewed_by.as_deref()).await {
        Ok(crate::db::review_queue::DecideOutcome::Decided(row)) => {
            // A decision is a review-state mutation: record it on the
            // audit trail (actor = the verified caller, else "system").
            let ctx = crate::db::AuditContext {
                user_id: reviewed_by.clone().or_else(|| Some("system".to_string())),
                ip_address: None,
                user_agent: None,
            };
            if let Err(e) = state
                .audit_log
                .log_action_on(
                    &state.db,
                    "review_decision",
                    "review_queue",
                    id,
                    None,
                    Some(serde_json::json!({ "status": token })),
                    &ctx,
                )
                .await
            {
                tracing::warn!("review-decision audit write failed: {e}");
            }
            (
                StatusCode::OK,
                Json(ApiResponse::success(review_row_to_item(&row))),
            )
                .into_response()
        }
        Ok(crate::db::review_queue::DecideOutcome::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<crate::models::ReviewQueueItem>::error(
                "NOT_FOUND",
                format!("no review item {id}"),
            )),
        )
            .into_response(),
        Ok(crate::db::review_queue::DecideOutcome::AlreadyDecided(current)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::<crate::models::ReviewQueueItem>::error(
                "INVALID_REVIEW_TRANSITION",
                format!("item is `{current}`; only `pending` items can be decided"),
            )),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<crate::models::ReviewQueueItem>::error(
                "DATABASE_ERROR",
                format!("Failed to decide review item: {e}"),
            )),
        )
            .into_response(),
    }
}

// ─── Data Export (GDPR Right of Access) ─────────────────────────────────────

/// Export all data for a person (GDPR right of access)
#[utoipa::path(
    get,
    path = "/api/persons/{id}/export",
    tag = "privacy",
    params(
        ("id" = Uuid, Path, description = "Person UUID")
    ),
    responses(
        (status = 200, description = "Person data export"),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn export_person_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            // An Art. 15 export hands over the whole record — the single
            // most consequential read this service serves, so it is
            // audited whatever the caller declared.
            if disclosure::record_access(
                &state.audit_log,
                "Person",
                id,
                disclosure::action::EXPORT,
                caller.claims().map(|c| c.sub.as_str()),
                &access,
            )
            .await
            .is_err()
            {
                return audit_unavailable::<serde_json::Value>();
            }
            let export = crate::privacy::export_person_data(&person);
            (StatusCode::OK, Json(ApiResponse::success(export)))
        }
        Ok(None) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "NOT_FOUND",
                format!("Person with id '{id}' not found"),
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get a person with sensitive data masked
#[utoipa::path(
    get,
    path = "/api/persons/{id}/masked",
    tag = "privacy",
    params(
        ("id" = Uuid, Path, description = "Person UUID")
    ),
    responses(
        (status = 200, description = "Masked person data"),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_person_masked(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            // A masked read is still a read: §164.312(b) records activity,
            // not just full disclosure.
            if disclosure::record_access(
                &state.audit_log,
                "Person",
                id,
                disclosure::action::READ,
                caller.claims().map(|c| c.sub.as_str()),
                &access,
            )
            .await
            .is_err()
            {
                return audit_unavailable::<Person>();
            }
            let masked = crate::privacy::mask_person(&person);
            (StatusCode::OK, Json(ApiResponse::success(masked)))
        }
        Ok(None) => {
            let error = ApiResponse::<Person>::error(
                "NOT_FOUND",
                format!("Person with id '{id}' not found"),
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

// ─── Audit Log Endpoints ────────────────────────────────────────────────────

/// Audit log query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AuditLogQuery {
    /// Maximum number of results (default: 50, max: 500)
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
}

/// Default audit-log result limit (serde default for the `limit` fields).
fn default_audit_limit() -> i64 {
    50
}

/// Get audit logs for a specific person
#[utoipa::path(
    get,
    path = "/api/persons/{id}/audit",
    tag = "audit",
    params(
        ("id" = Uuid, Path, description = "Person UUID"),
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "Audit logs retrieved successfully"),
        (status = 500, description = "Database error")
    )
)]
pub async fn get_person_audit_logs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);

    match state
        .audit_log
        .get_logs_for_entity("Person", id, u64::try_from(limit).unwrap_or(0))
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

/// Erase a person under GDPR Art. 17.
///
/// `POST /api/persons/{id}/erase` — destroys the person's child rows
/// (names, identifiers, addresses, contacts, documents, emergency
/// contacts, photos, links, match scores), scrubs the parent row's own
/// personal fields, retires the record, withdraws its cross-service
/// links, destroys the content of every audit row about it, and appends a
/// chained `erased` accountability row. The audit hash chain keeps
/// verifying, because redaction preserves each row's stored hash and
/// linkage (see [`crate::compliance::erasure`]).
///
/// This is **not** the soft delete. `DELETE /{id}` retires a record and
/// keeps its data; this destroys the data and is irreversible — which is
/// why it is a **destructive** action under ABAC
/// ([`super::auth::DESTRUCTIVE_POST_SUFFIXES`]) and requires
/// `access=admin`.
///
/// Runs in a transaction. The child deletes, the parent scrub, and the
/// tombstone-name insert are separate statements, and a failure between
/// them would leave a record with no names and un-scrubbed demographics —
/// worse than either outcome on its own.
///
/// Idempotent, and answers for an unknown id rather than `404`: a
/// subject's right to erasure does not lapse once the record is
/// soft-deleted (the audit content held about it is still personal data),
/// and a `404` would confirm to a prober which ids are unknown.
#[utoipa::path(
    post,
    path = "/api/persons/{id}/erase",
    tag = "privacy",
    params(("id" = Uuid, Path, description = "Person UUID")),
    responses(
        (status = 200, description = "Erasure outcome"),
        (status = 403, description = "Policy denied this destructive action"),
        (status = 500, description = "Database error")
    )
)]
pub async fn erase_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> impl IntoResponse {
    use sea_orm::TransactionTrait as _;

    let actor = caller.claims().map(|c| c.sub.as_str());
    let txn = match state.db.begin().await {
        Ok(txn) => txn,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DATABASE_ERROR",
                    format!("Failed to start the erasure transaction: {e}"),
                )),
            );
        }
    };
    let outcome =
        match crate::compliance::erasure::erase(&txn, id, actor, &access, &state.audit_log).await {
            Ok(outcome) => outcome,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<serde_json::Value>::error(
                        "DATABASE_ERROR",
                        format!("Failed to erase: {e}"),
                    )),
                );
            }
        };
    if let Err(e) = txn.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to commit the erasure: {e}"),
            )),
        );
    }

    // The search index holds a copy of the personal data, so an erasure
    // that leaves it indexed has not erased anything a search can reach.
    // A failure here is logged rather than fatal: the durable data is
    // already gone and the transaction has committed, so refusing the
    // response would misreport a completed erasure as a failure.
    if let Err(e) = state.search_engine.delete_person(&id.to_string()) {
        tracing::error!(%e, person_id = %id, "erased the record but failed to drop it from the search index");
    }

    let value = serde_json::to_value(&outcome).unwrap_or_else(|_| serde_json::json!({}));
    (StatusCode::OK, Json(ApiResponse::success(value)))
}

/// Accounting of disclosures for one person (HIPAA §164.528).
///
/// `GET /api/persons/{id}/audit/disclosures` — every audit row for this
/// person classified as an outward **disclosure** rather than an internal
/// access, newest first.
///
/// Gated by the same record-level authorization as reading the record:
/// learning who a record was disclosed to reveals that the record
/// exists, so the accounting cannot be more open than the record it
/// describes.
///
/// The response states whether read-auditing is switched on. Without
/// that caveat the endpoint is actively misleading: with
/// `PERSON_AUDIT_READS` off an empty list means "reads are not being
/// recorded", not "this record was never disclosed", and §164.528 is a
/// question a patient is entitled to a truthful answer to.
#[utoipa::path(
    get,
    path = "/api/persons/{id}/audit/disclosures",
    tag = "audit",
    params(
        ("id" = Uuid, Path, description = "Person UUID"),
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "Accounting of disclosures"),
        (status = 403, description = "Policy denied reading this record"),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Database error")
    )
)]
pub async fn get_person_disclosures(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<AuditLogQuery>,
    caller: MaybeAuthUser,
) -> impl IntoResponse {
    let limit = params.limit.min(500);

    // Concealment first: an unauthorised caller must not learn the record
    // exists by asking for its disclosure history.
    let record = match state.person_repository.get_by_id(&id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<serde_json::Value>::error(
                    "NOT_FOUND",
                    format!("Person with id '{id}' not found"),
                )),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DATABASE_ERROR",
                    format!("Failed to retrieve person: {e}"),
                )),
            );
        }
    };
    if let Err((status, reason)) =
        authorize_record(&caller, Action::Read, &person_resource_attrs(&record))
    {
        return (
            status,
            Json(ApiResponse::<serde_json::Value>::error("FORBIDDEN", reason)),
        );
    }

    match state
        .audit_log
        .disclosures_for_entity(id, u64::try_from(limit).unwrap_or(0))
        .await
    {
        Ok(rows) => {
            let auditing = crate::compliance::audit_reads();
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "id": id,
                    "read_auditing_enabled": auditing,
                    "count": rows.len(),
                    "caveat": if auditing {
                        "complete for the period read-auditing has been enabled"
                    } else {
                        "INCOMPLETE — PERSON_AUDIT_READS is off, so read disclosures are not \
                         being recorded; only disclosure-flagged mutations appear here"
                    },
                    "disclosures": rows,
                }))),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve the accounting of disclosures: {e}"),
            )),
        ),
    }
}

/// Verify row-level record integrity across a page of person records.
///
/// `GET /api/records/verify?limit=100` — recomputes each record's content
/// hash and reports every mismatch (HIPAA §164.312(c)).
///
/// This is the **complement** to `/api/audit/verify`, not a duplicate of
/// it. The audit chain proves the *trail* was not rewritten; this proves
/// the *records* were not edited out of band. An attacker with SQL access
/// who edits a stored name or identifier and writes no audit row defeats
/// the first control and is caught by this one.
///
/// Rows written before the `content_hash` column existed report as
/// `unhashed` rather than as mismatches: adopting the control on a
/// populated table must not produce a wall of false positives, and they
/// are hashed on their next write. They are *not* back-filled, because
/// computing a hash from the current content would certify whatever is
/// there — which is the claim the hash exists to test.
#[utoipa::path(
    get,
    path = "/api/records/verify",
    tag = "audit",
    params(AuditLogQuery),
    responses(
        (status = 200, description = "Record integrity report"),
        (status = 500, description = "Database error")
    )
)]
pub async fn verify_record_integrity(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    use sea_orm::ConnectionTrait as _;

    // Verification is O(rows) with a SHA-256 and a record assembly each,
    // so an unbounded limit is a CPU denial-of-service (SEC-M1).
    let limit = params.limit.clamp(1, 500);

    let rows = match state
        .db
        .query_all(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, content_hash, content_hash_blake3, content_hash_sha3, deleted_at \
             FROM persons \
             ORDER BY updated_at DESC LIMIT $1",
            [limit.into()],
        ))
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
            );
        }
    };

    // Assemble each record so the digest covers the child tables too —
    // which is the whole point, since a name or identifier edit lives
    // there. One query per record, which is why the limit is capped.
    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let Ok(id) = row.try_get::<Uuid>("", "id") else {
            continue;
        };
        let stored: Option<String> = row.try_get("", "content_hash").unwrap_or(None);
        let stored_b3: Option<String> = row.try_get("", "content_hash_blake3").unwrap_or(None);
        let stored_sha3: Option<String> = row.try_get("", "content_hash_sha3").unwrap_or(None);
        let deleted_at: Option<time::OffsetDateTime> =
            row.try_get("", "deleted_at").unwrap_or(None);
        let deleted_micros =
            deleted_at.and_then(|d| i64::try_from(d.unix_timestamp_nanos() / 1_000).ok());
        match state.person_repository.get_by_id(&id).await {
            Ok(Some(person)) => {
                records.push((person, stored, stored_b3, stored_sha3, deleted_micros));
            }
            // A row that vanished between the two queries is not a
            // mismatch; skipping it is honest, and the count reflects it.
            Ok(None) => {}
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<serde_json::Value>::error(
                        "DATABASE_ERROR",
                        format!("Failed to assemble record {id} for verification: {e}"),
                    )),
                );
            }
        }
    }

    let report = crate::compliance::record_integrity::verify(&records);
    let interpretation = if report.verified {
        "no record in the verified window differs from its stored hash; this attests to the \
         person records, not to the audit trail — see /api/audit/verify for that"
    } else {
        "a mismatch means the record's content changed without the service rehashing it — \
         either an out-of-band SQL edit, or a write path that forgot to rehash; investigate \
         the named ids against the audit trail"
    };
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "limit": limit,
            "verified": report.verified,
            "records": report.records,
            "intact": report.intact,
            "unhashed": report.unhashed,
            "mismatched": report.mismatched,
            "interpretation": interpretation,
        }))),
    )
}

/// Verify the tamper-evident audit hash chain.
///
/// `GET /api/audit/verify?limit=1000` — recomputes the trailing rows and
/// reports every linkage or content break (HIPAA §164.312(c)).
///
/// Attests to the **audit trail** only, not to the `persons` rows; the
/// response says so, because the difference matters to whoever reads it.
#[utoipa::path(
    get,
    path = "/api/audit/verify",
    tag = "audit",
    params(AuditLogQuery),
    responses(
        (status = 200, description = "Chain verification report"),
        (status = 500, description = "Database error")
    )
)]
pub async fn verify_audit_chain(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    // Verification is O(rows) with a SHA-256 each, so an unbounded limit
    // is a CPU denial-of-service (the SEC-M1 bound-every-input rule).
    let limit = u64::try_from(params.limit).unwrap_or(1000).clamp(1, 10_000);
    match state.audit_log.verify_chain(limit).await {
        Ok(report) => {
            let interpretation = if report.verified {
                "no break detected in the verified window; this attests to the audit trail \
                 only, not to the person records"
            } else {
                "a break means rows were inserted, deleted, reordered, or edited since they \
                 were written — investigate the named seq/id; a concurrent audit write on a \
                 pooled connection can also fork the chain, which reports as a linkage break"
            };
            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "limit": limit,
                    "verified": report.verified,
                    "rows": report.rows,
                    "intact": report.intact,
                    "redacted": report.redacted,
                    "unchained": report.unchained,
                    "head": report.head,
                    "breaks": report.breaks,
                    "interpretation": interpretation,
                }))),
            )
        }
        Err(e) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to verify the audit chain: {e}"),
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// A refused read-audit write, as a `503 Service Unavailable` response.
///
/// `503` rather than `500`: nothing is wrong with the request, and nothing
/// was disclosed — the service is temporarily unable to account for a read
/// of personal data, so it declines to serve one, and the status is
/// retryable. Only reachable when `PERSON_AUDIT_FAIL_CLOSED` is on.
fn audit_unavailable<T: serde::Serialize>() -> (StatusCode, Json<ApiResponse<T>>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::error(
            "AUDIT_UNAVAILABLE",
            "the access could not be recorded in the audit trail, so the read was refused",
        )),
    )
}

/// Get recent audit logs
#[utoipa::path(
    get,
    path = "/api/audit/recent",
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

/// User audit log query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct UserAuditLogQuery {
    /// User ID to filter by
    pub user_id: String,

    /// Maximum number of results (default: 50, max: 500)
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
}

/// Get audit logs by user
#[utoipa::path(
    get,
    path = "/api/audit/user",
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

#[cfg(test)]
mod tests {
    use super::{
        MAX_SEARCH_OFFSET, ResultDisposition, search_offset_within_bound, search_result_disposition,
    };

    /// SEC-G7: a pagination offset at or under the cap is accepted; anything
    /// past it (up to `usize::MAX`) is rejected, so the search engine is
    /// never asked to materialise an unbounded number of hits.
    #[test]
    fn search_offset_bound_accepts_only_up_to_the_cap() {
        assert!(search_offset_within_bound(0));
        assert!(search_offset_within_bound(MAX_SEARCH_OFFSET));
        assert!(!search_offset_within_bound(MAX_SEARCH_OFFSET + 1));
        assert!(!search_offset_within_bound(usize::MAX));
    }

    /// SEC-G3: the per-result concealment/masking decision.
    #[test]
    fn search_result_disposition_conceals_masks_and_preserves_client_param() {
        // Denied read ⇒ omit the record entirely (concealment), regardless
        // of the client's mask request.
        assert_eq!(
            search_result_disposition(None, false),
            ResultDisposition::Omit
        );
        assert_eq!(
            search_result_disposition(None, true),
            ResultDisposition::Omit
        );

        // Readable with no obligation:
        //  - flag off / no mask policy + client didn't ask ⇒ full record
        //    (pre-SEC-G3 behaviour preserved).
        assert_eq!(
            search_result_disposition(Some(&[]), false),
            ResultDisposition::Full
        );
        //  - client asked to mask ⇒ masked (the existing convenience).
        assert_eq!(
            search_result_disposition(Some(&[]), true),
            ResultDisposition::Masked
        );

        // A `mask` obligation masks even when the client did NOT ask — the
        // SEC-G3 bypass being closed (a mask-only policy can no longer be
        // defeated by omitting `mask_sensitive`).
        let mask = ["mask".to_string()];
        assert_eq!(
            search_result_disposition(Some(&mask), false),
            ResultDisposition::Masked
        );
        assert_eq!(
            search_result_disposition(Some(&mask), true),
            ResultDisposition::Masked
        );

        // An unrelated obligation does not force masking on its own.
        let other = ["audit".to_string()];
        assert_eq!(
            search_result_disposition(Some(&other), false),
            ResultDisposition::Full
        );
    }
}
