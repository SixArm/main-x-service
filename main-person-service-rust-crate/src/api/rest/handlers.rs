//! REST API request handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;
use chrono::Datelike;

use crate::models::Person;
use crate::api::ApiResponse;
use super::state::AppState;

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Health check endpoint
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
        service: "main-person-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create person request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePersonRequest {
    #[serde(flatten)]
    pub person: Person,
}

/// Create a new person
#[utoipa::path(
    post,
    path = "/api/v1/persons",
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
            format!("Validation failed: {}", validation_errors.iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join("; "))
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
            "Potential duplicate persons found. Review matches before proceeding."
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
                format!("Failed to create person: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get a person by ID
#[utoipa::path(
    get,
    path = "/api/v1/persons/{id}",
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
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            (StatusCode::OK, Json(ApiResponse::success(person)))
        }
        Ok(None) => {
            let error = ApiResponse::<Person>::error(
                "NOT_FOUND",
                format!("Person with id '{}' not found", id)
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Update a person
#[utoipa::path(
    put,
    path = "/api/v1/persons/{id}",
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
    Json(mut payload): Json<Person>,
) -> impl IntoResponse {
    // Validate
    let validation_errors = crate::validation::validate_person(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Person>::error(
            "VALIDATION_ERROR",
            format!("Validation failed: {}", validation_errors.iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join("; "))
        );
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error));
    }

    // Ensure ID in path matches payload
    payload.id = id;

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
                format!("Failed to update person: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Delete a person (soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/persons/{id}",
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
) -> impl IntoResponse {
    match state.person_repository.delete(&id).await {
        Ok(()) => {
            // Remove from search index
            if let Err(e) = state.search_engine.delete_person(&id.to_string()) {
                tracing::warn!("Failed to delete person from search engine: {}", e);
            }

            (StatusCode::NO_CONTENT, Json(ApiResponse::<()>::success(())))
        }
        Err(e) => {
            let error = ApiResponse::<()>::error(
                "DATABASE_ERROR",
                format!("Failed to delete person: {}", e)
            );
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

fn default_limit() -> usize {
    10
}

/// Search results response
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    pub persons: Vec<Person>,
    pub total: usize,
    pub query: String,
    pub offset: usize,
    pub limit: usize,
}

/// Search for persons
#[utoipa::path(
    get,
    path = "/api/v1/persons/search",
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
) -> impl IntoResponse {
    // Limit to max 100 results
    let limit = params.limit.min(100);

    // Perform search using search engine
    // Request more results to handle pagination offset
    let total_needed = params.offset + limit;
    let person_ids = if params.fuzzy {
        state.search_engine.fuzzy_search(&params.q, total_needed)
    } else {
        state.search_engine.search(&params.q, total_needed)
    };

    match person_ids {
        Ok(ids) => {
            // Apply offset and limit
            let paginated_ids: Vec<_> = ids.into_iter()
                .skip(params.offset)
                .take(limit)
                .collect();

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
                        if params.mask_sensitive {
                            persons.push(crate::privacy::mask_person(&person));
                        } else {
                            persons.push(person);
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("Person {} found in search index but not in database", person_id);
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
            let error = ApiResponse::<SearchResponse>::error(
                "SEARCH_ERROR",
                format!("Search failed: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Match request payload
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

fn default_match_limit() -> usize {
    10
}

/// Match result with score
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MatchResponse {
    pub person: Person,
    pub score: f64,
    pub quality: String,
    pub detection_method: String,
    pub score_breakdown: Option<serde_json::Value>,
}

/// Match results response
#[derive(Debug, Serialize, ToSchema)]
pub struct MatchResultsResponse {
    pub matches: Vec<MatchResponse>,
    pub total: usize,
}

/// Match a person against existing records
#[utoipa::path(
    post,
    path = "/api/v1/persons/match",
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

    let candidate_ids = state.search_engine
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
                        tracing::warn!("Person {} found in search index but not in database", person_id);
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
                        format!("Matching failed: {}", e)
                    );
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(error));
                }
            };

            // Filter by threshold if provided
            let threshold = payload.threshold.unwrap_or(0.5);
            let matches: Vec<MatchResponse> = match_results.into_iter()
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
                format!("Matching failed: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

// ─── Duplicate Detection ────────────────────────────────────────────────────

/// Response for duplicate checking
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DuplicateCheckResponse {
    pub has_duplicates: bool,
    pub potential_matches: Vec<MatchResponse>,
}

/// Internal duplicate detection logic shared by create_person and the explicit endpoint.
async fn check_duplicates_internal(state: &AppState, person: &Person) -> Vec<MatchResponse> {
    let family_name = &person.name.family;
    let birth_year = person.birth_date.map(|d| d.year());

    let candidate_ids = match state.search_engine.search_by_name_and_year(family_name, birth_year, 50) {
        Ok(ids) => ids,
        Err(_) => return Vec::new(),
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

    let match_results = match state.matcher.find_matches(person, &candidates) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    // Return matches above the auto-review threshold (0.7)
    match_results.into_iter()
        .filter(|m| m.score >= 0.7)
        .take(10)
        .map(|m| {
            let quality = if m.score >= 0.95 { "certain" }
                else if m.score >= 0.7 { "probable" }
                else { "possible" };

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
    path = "/api/v1/persons/check-duplicates",
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

/// Merge two person records
#[utoipa::path(
    post,
    path = "/api/v1/persons/merge",
    tag = "deduplication",
    request_body = crate::models::MergeRequest,
    responses(
        (status = 200, description = "Merge completed", body = crate::models::MergeResponse),
        (status = 404, description = "Person not found"),
        (status = 500, description = "Merge error")
    )
)]
pub async fn merge_persons(
    State(state): State<AppState>,
    Json(req): Json<crate::models::MergeRequest>,
) -> impl IntoResponse {
    // Fetch both persons
    let main = match state.person_repository.get_by_id(&req.main_person_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "NOT_FOUND", format!("Main person {} not found", req.main_person_id)
            )));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR", format!("Failed to fetch main person: {}", e)
            )));
        }
    };

    let duplicate = match state.person_repository.get_by_id(&req.duplicate_person_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "NOT_FOUND", format!("Duplicate person {} not found", req.duplicate_person_id)
            )));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR", format!("Failed to fetch duplicate person: {}", e)
            )));
        }
    };

    // Merge data from duplicate into main
    let mut merged = main.clone();
    let mut transferred = serde_json::Map::new();

    // Transfer identifiers not already present
    for id in &duplicate.identifiers {
        if !merged.identifiers.iter().any(|existing| existing.value == id.value && existing.identifier_type == id.identifier_type) {
            merged.identifiers.push(id.clone());
            transferred.entry("identifiers".to_string())
                .or_insert_with(|| serde_json::Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
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
        if !merged.telecom.iter().any(|existing| existing.value == cp.value) {
            merged.telecom.push(cp.clone());
        }
    }

    // Transfer documents
    for doc in &duplicate.documents {
        if !merged.documents.iter().any(|existing| existing.number == doc.number && existing.document_type == doc.document_type) {
            merged.documents.push(doc.clone());
        }
    }

    // Transfer emergency contacts
    for ec in &duplicate.emergency_contacts {
        if !merged.emergency_contacts.iter().any(|existing| existing.name == ec.name) {
            merged.emergency_contacts.push(ec.clone());
        }
    }

    // Transfer tax_id if main doesn't have one
    if merged.tax_id.is_none() && duplicate.tax_id.is_some() {
        merged.tax_id = duplicate.tax_id.clone();
        transferred.insert("tax_id".into(), serde_json::to_value(&duplicate.tax_id).unwrap_or_default());
    }

    // Add a link from main → replaces duplicate
    merged.links.push(crate::models::PersonLink {
        other_person_id: duplicate.id,
        link_type: crate::models::LinkType::Replaces,
    });

    // Update main person
    if let Err(e) = state.person_repository.update(&merged).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::MergeResponse>::error(
            "DATABASE_ERROR", format!("Failed to update main person: {}", e)
        )));
    }

    // Soft-delete the duplicate
    if let Err(e) = state.person_repository.delete(&duplicate.id).await {
        tracing::error!("Failed to soft-delete duplicate person: {}", e);
    }

    // Remove duplicate from search index
    if let Err(e) = state.search_engine.delete_person(&duplicate.id.to_string()) {
        tracing::warn!("Failed to remove duplicate from search index: {}", e);
    }

    // Update search index for main
    if let Err(e) = state.search_engine.index_person(&merged) {
        tracing::warn!("Failed to update search index for merged person: {}", e);
    }

    // Publish merge event
    state.event_publisher.publish(crate::streaming::PersonEvent::Merged {
        source_id: duplicate.id,
        target_id: merged.id,
        timestamp: chrono::Utc::now(),
    }).ok();

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
        merged_at: chrono::Utc::now(),
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
    path = "/api/v1/persons/deduplicate",
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
    // Get all active persons
    let persons = match state.person_repository.list_active(1000, 0).await {
        Ok(p) => p,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::BatchDeduplicationResponse>::error(
                "DATABASE_ERROR", format!("Failed to list persons: {}", e)
            )));
        }
    };

    let persons_scanned = persons.len();
    let mut review_items = Vec::new();
    let mut auto_merged = 0usize;
    let mut seen_pairs: std::collections::HashSet<(Uuid, Uuid)> = std::collections::HashSet::new();

    for (i, person) in persons.iter().enumerate() {
        // Compare with subsequent persons to avoid duplicate pairs
        let candidates: Vec<_> = persons[i+1..].iter()
            .take(req.max_candidates)
            .cloned()
            .collect();

        if candidates.is_empty() {
            continue;
        }

        let matches = match state.matcher.find_matches(person, &candidates) {
            Ok(m) => m,
            Err(_) => continue,
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

            let quality = if m.score >= 0.95 { "certain" }
                else if m.score >= 0.7 { "probable" }
                else { "possible" };

            let status = if m.score >= req.auto_merge_threshold {
                auto_merged += 1;
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
                created_at: chrono::Utc::now(),
                reviewed_at: None,
            });
        }
    }

    let queued = review_items.iter().filter(|r| r.status == crate::models::ReviewStatus::Pending).count();

    let response = crate::models::BatchDeduplicationResponse {
        persons_scanned,
        duplicates_found: review_items.len(),
        auto_merged,
        queued_for_review: queued,
        review_items,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Data Export (GDPR Right of Access) ─────────────────────────────────────

/// Export all data for a person (GDPR right of access)
#[utoipa::path(
    get,
    path = "/api/v1/persons/{id}/export",
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
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            let export = crate::privacy::export_person_data(&person);
            (StatusCode::OK, Json(ApiResponse::success(export)))
        }
        Ok(None) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "NOT_FOUND",
                format!("Person with id '{}' not found", id)
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get a person with sensitive data masked
#[utoipa::path(
    get,
    path = "/api/v1/persons/{id}/masked",
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
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            let masked = crate::privacy::mask_person(&person);
            (StatusCode::OK, Json(ApiResponse::success(masked)))
        }
        Ok(None) => {
            let error = ApiResponse::<Person>::error(
                "NOT_FOUND",
                format!("Person with id '{}' not found", id)
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Person>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve person: {}", e)
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

fn default_audit_limit() -> i64 {
    50
}

/// Get audit logs for a specific person
#[utoipa::path(
    get,
    path = "/api/v1/persons/{id}/audit",
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

    match state.audit_log.get_logs_for_entity("Person", id, limit as u64).await {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => {
            let error = ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve audit logs: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get recent audit logs
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

    match state.audit_log.get_recent_logs(limit as u64).await {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => {
            let error = ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve audit logs: {}", e)
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

    match state.audit_log.get_logs_by_user(&params.user_id, limit as u64).await {
        Ok(logs) => (StatusCode::OK, Json(ApiResponse::success(logs))),
        Err(e) => {
            let error = ApiResponse::<Vec<crate::db::models::audit_log::Model>>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve audit logs: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}
