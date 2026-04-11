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

use crate::models::Patient;
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
        service: "main-patient-index".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create patient request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePatientRequest {
    #[serde(flatten)]
    pub patient: Patient,
}

/// Create a new patient
#[utoipa::path(
    post,
    path = "/api/v1/patients",
    tag = "patients",
    request_body = Patient,
    responses(
        (status = 201, description = "Patient created successfully"),
        (status = 409, description = "Potential duplicates detected"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_patient(
    State(state): State<AppState>,
    Json(mut payload): Json<Patient>,
) -> impl IntoResponse {
    // Validate patient data
    let validation_errors = crate::validation::validate_patient(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Patient>::error(
            "VALIDATION_ERROR",
            format!("Validation failed: {}", validation_errors.iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect::<Vec<_>>()
                .join("; "))
        );
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error));
    }

    // Ensure patient has a UUID
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
        let mut error = ApiResponse::<Patient>::error(
            "DUPLICATE_DETECTED",
            "Potential duplicate patients found. Review matches before proceeding."
        );
        if let Some(ref mut err) = error.error {
            err.details = details;
        }
        return (StatusCode::CONFLICT, Json(error));
    }

    // Insert into database
    match state.patient_repository.create(&payload).await {
        Ok(patient) => {
            // Index in search engine
            if let Err(e) = state.search_engine.index_patient(&patient) {
                tracing::warn!("Failed to index patient in search engine: {}", e);
            }

            (StatusCode::CREATED, Json(ApiResponse::success(patient)))
        }
        Err(e) => {
            let error = ApiResponse::<Patient>::error(
                "DATABASE_ERROR",
                format!("Failed to create patient: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get a patient by ID
#[utoipa::path(
    get,
    path = "/api/v1/patients/{id}",
    tag = "patients",
    params(
        ("id" = Uuid, Path, description = "Patient UUID")
    ),
    responses(
        (status = 200, description = "Patient found"),
        (status = 404, description = "Patient not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(patient)) => {
            (StatusCode::OK, Json(ApiResponse::success(patient)))
        }
        Ok(None) => {
            let error = ApiResponse::<Patient>::error(
                "NOT_FOUND",
                format!("Patient with id '{}' not found", id)
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Patient>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve patient: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Update a patient
#[utoipa::path(
    put,
    path = "/api/v1/patients/{id}",
    tag = "patients",
    params(
        ("id" = Uuid, Path, description = "Patient UUID")
    ),
    request_body = Patient,
    responses(
        (status = 200, description = "Patient updated successfully"),
        (status = 422, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(mut payload): Json<Patient>,
) -> impl IntoResponse {
    // Validate
    let validation_errors = crate::validation::validate_patient(&payload);
    if !validation_errors.is_empty() {
        let error = ApiResponse::<Patient>::error(
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

    match state.patient_repository.update(&payload).await {
        Ok(patient) => {
            // Update search index
            if let Err(e) = state.search_engine.index_patient(&patient) {
                tracing::warn!("Failed to update patient in search engine: {}", e);
            }

            (StatusCode::OK, Json(ApiResponse::success(patient)))
        }
        Err(e) => {
            let error = ApiResponse::<Patient>::error(
                "DATABASE_ERROR",
                format!("Failed to update patient: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Delete a patient (soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/patients/{id}",
    tag = "patients",
    params(
        ("id" = Uuid, Path, description = "Patient UUID")
    ),
    responses(
        (status = 204, description = "Patient deleted successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.patient_repository.delete(&id).await {
        Ok(()) => {
            // Remove from search index
            if let Err(e) = state.search_engine.delete_patient(&id.to_string()) {
                tracing::warn!("Failed to delete patient from search engine: {}", e);
            }

            (StatusCode::NO_CONTENT, Json(ApiResponse::<()>::success(())))
        }
        Err(e) => {
            let error = ApiResponse::<()>::error(
                "DATABASE_ERROR",
                format!("Failed to delete patient: {}", e)
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
    pub patients: Vec<Patient>,
    pub total: usize,
    pub query: String,
    pub offset: usize,
    pub limit: usize,
}

/// Search for patients
#[utoipa::path(
    get,
    path = "/api/v1/patients/search",
    tag = "search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 500, description = "Search error")
    )
)]
pub async fn search_patients(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    // Limit to max 100 results
    let limit = params.limit.min(100);

    // Perform search using search engine
    // Request more results to handle pagination offset
    let total_needed = params.offset + limit;
    let patient_ids = if params.fuzzy {
        state.search_engine.fuzzy_search(&params.q, total_needed)
    } else {
        state.search_engine.search(&params.q, total_needed)
    };

    match patient_ids {
        Ok(ids) => {
            // Apply offset and limit
            let paginated_ids: Vec<_> = ids.into_iter()
                .skip(params.offset)
                .take(limit)
                .collect();

            // Fetch full patient records from database
            let mut patients = Vec::new();
            for patient_id_str in paginated_ids {
                let patient_id = match Uuid::parse_str(&patient_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse patient ID {}: {}", patient_id_str, e);
                        continue;
                    }
                };

                match state.patient_repository.get_by_id(&patient_id).await {
                    Ok(Some(patient)) => {
                        if params.mask_sensitive {
                            patients.push(crate::privacy::mask_patient(&patient));
                        } else {
                            patients.push(patient);
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("Patient {} found in search index but not in database", patient_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch patient {}: {}", patient_id, e);
                    }
                }
            }

            let response = SearchResponse {
                total: patients.len(),
                patients,
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
    /// Patient to match against existing records
    #[serde(flatten)]
    pub patient: Patient,

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
    pub patient: Patient,
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

/// Match a patient against existing records
#[utoipa::path(
    post,
    path = "/api/v1/patients/match",
    tag = "matching",
    request_body = MatchRequest,
    responses(
        (status = 200, description = "Match results", body = MatchResultsResponse),
        (status = 500, description = "Matching error")
    )
)]
pub async fn match_patient(
    State(state): State<AppState>,
    Json(payload): Json<MatchRequest>,
) -> impl IntoResponse {
    // Use search engine to get candidate patients (blocking)
    let family_name = &payload.patient.name.family;
    let birth_year = payload.patient.birth_date.map(|d| d.year());

    let candidate_ids = state.search_engine
        .search_by_name_and_year(family_name, birth_year, 100);

    match candidate_ids {
        Ok(ids) => {
            // Fetch full patient records from database
            let mut candidates = Vec::new();
            for patient_id_str in ids {
                let patient_id = match Uuid::parse_str(&patient_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse patient ID {}: {}", patient_id_str, e);
                        continue;
                    }
                };

                match state.patient_repository.get_by_id(&patient_id).await {
                    Ok(Some(patient)) => candidates.push(patient),
                    Ok(None) => {
                        tracing::warn!("Patient {} found in search index but not in database", patient_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch patient {}: {}", patient_id, e);
                    }
                }
            }

            // Run matcher on candidates
            let match_results = match state.matcher.find_matches(&payload.patient, &candidates) {
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
                        patient: m.patient.clone(),
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

/// Internal duplicate detection logic shared by create_patient and the explicit endpoint.
async fn check_duplicates_internal(state: &AppState, patient: &Patient) -> Vec<MatchResponse> {
    let family_name = &patient.name.family;
    let birth_year = patient.birth_date.map(|d| d.year());

    let candidate_ids = match state.search_engine.search_by_name_and_year(family_name, birth_year, 50) {
        Ok(ids) => ids,
        Err(_) => return Vec::new(),
    };

    let mut candidates = Vec::new();
    for id_str in candidate_ids {
        if let Ok(pid) = Uuid::parse_str(&id_str) {
            if pid == patient.id {
                continue; // Skip self
            }
            if let Ok(Some(p)) = state.patient_repository.get_by_id(&pid).await {
                candidates.push(p);
            }
        }
    }

    let match_results = match state.matcher.find_matches(patient, &candidates) {
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
                patient: m.patient.clone(),
                score: m.score,
                quality: quality.to_string(),
                detection_method: "duplicate_detection".to_string(),
                score_breakdown: serde_json::to_value(&m.breakdown).ok(),
            }
        })
        .collect()
}

/// Check for duplicates without creating a patient
#[utoipa::path(
    post,
    path = "/api/v1/patients/check-duplicates",
    tag = "deduplication",
    request_body = Patient,
    responses(
        (status = 200, description = "Duplicate check results", body = DuplicateCheckResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn check_duplicates(
    State(state): State<AppState>,
    Json(patient): Json<Patient>,
) -> impl IntoResponse {
    let matches = check_duplicates_internal(&state, &patient).await;
    let response = DuplicateCheckResponse {
        has_duplicates: !matches.is_empty(),
        potential_matches: matches,
    };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Record Merging ─────────────────────────────────────────────────────────

/// Merge two patient records
#[utoipa::path(
    post,
    path = "/api/v1/patients/merge",
    tag = "deduplication",
    request_body = crate::models::MergeRequest,
    responses(
        (status = 200, description = "Merge completed", body = crate::models::MergeResponse),
        (status = 404, description = "Patient not found"),
        (status = 500, description = "Merge error")
    )
)]
pub async fn merge_patients(
    State(state): State<AppState>,
    Json(req): Json<crate::models::MergeRequest>,
) -> impl IntoResponse {
    // Fetch both patients
    let master = match state.patient_repository.get_by_id(&req.main_patient_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "NOT_FOUND", format!("Master patient {} not found", req.main_patient_id)
            )));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR", format!("Failed to fetch main patient: {}", e)
            )));
        }
    };

    let duplicate = match state.patient_repository.get_by_id(&req.duplicate_patient_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "NOT_FOUND", format!("Duplicate patient {} not found", req.duplicate_patient_id)
            )));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::MergeResponse>::error(
                "DATABASE_ERROR", format!("Failed to fetch duplicate patient: {}", e)
            )));
        }
    };

    // Merge data from duplicate into master
    let mut merged = master.clone();
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

    // Transfer tax_id if master doesn't have one
    if merged.tax_id.is_none() && duplicate.tax_id.is_some() {
        merged.tax_id = duplicate.tax_id.clone();
        transferred.insert("tax_id".into(), serde_json::to_value(&duplicate.tax_id).unwrap_or_default());
    }

    // Add a link from master → replaces duplicate
    merged.links.push(crate::models::PatientLink {
        other_patient_id: duplicate.id,
        link_type: crate::models::LinkType::Replaces,
    });

    // Update main patient
    if let Err(e) = state.patient_repository.update(&merged).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::MergeResponse>::error(
            "DATABASE_ERROR", format!("Failed to update main patient: {}", e)
        )));
    }

    // Soft-delete the duplicate
    if let Err(e) = state.patient_repository.delete(&duplicate.id).await {
        tracing::error!("Failed to soft-delete duplicate patient: {}", e);
    }

    // Remove duplicate from search index
    if let Err(e) = state.search_engine.delete_patient(&duplicate.id.to_string()) {
        tracing::warn!("Failed to remove duplicate from search index: {}", e);
    }

    // Update search index for master
    if let Err(e) = state.search_engine.index_patient(&merged) {
        tracing::warn!("Failed to update search index for merged patient: {}", e);
    }

    // Publish merge event
    state.event_publisher.publish(crate::streaming::PatientEvent::Merged {
        source_id: duplicate.id,
        target_id: merged.id,
        timestamp: chrono::Utc::now(),
    }).ok();

    // Create merge record
    let merge_record = crate::models::MergeRecord {
        id: Uuid::new_v4(),
        main_patient_id: merged.id,
        duplicate_patient_id: duplicate.id,
        status: crate::models::MergeStatus::Completed,
        merged_by: req.merged_by,
        merge_reason: req.merge_reason,
        match_score: None,
        transferred_data: Some(serde_json::Value::Object(transferred)),
        merged_at: chrono::Utc::now(),
    };

    let response = crate::models::MergeResponse {
        merge_record,
        main_patient: merged,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Batch Deduplication ────────────────────────────────────────────────────

/// Run batch deduplication across all patients
#[utoipa::path(
    post,
    path = "/api/v1/patients/deduplicate",
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
    // Get all active patients
    let patients = match state.patient_repository.list_active(1000, 0).await {
        Ok(p) => p,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<crate::models::BatchDeduplicationResponse>::error(
                "DATABASE_ERROR", format!("Failed to list patients: {}", e)
            )));
        }
    };

    let patients_scanned = patients.len();
    let mut review_items = Vec::new();
    let mut auto_merged = 0usize;
    let mut seen_pairs: std::collections::HashSet<(Uuid, Uuid)> = std::collections::HashSet::new();

    for (i, patient) in patients.iter().enumerate() {
        // Compare with subsequent patients to avoid duplicate pairs
        let candidates: Vec<_> = patients[i+1..].iter()
            .take(req.max_candidates)
            .cloned()
            .collect();

        if candidates.is_empty() {
            continue;
        }

        let matches = match state.matcher.find_matches(patient, &candidates) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for m in matches {
            if m.score < req.threshold {
                continue;
            }

            // Normalize pair order to avoid duplicates
            let pair = if patient.id < m.patient.id {
                (patient.id, m.patient.id)
            } else {
                (m.patient.id, patient.id)
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
                patient_id_a: patient.id,
                patient_id_b: m.patient.id,
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
        patients_scanned,
        duplicates_found: review_items.len(),
        auto_merged,
        queued_for_review: queued,
        review_items,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ─── Data Export (GDPR Right of Access) ─────────────────────────────────────

/// Export all data for a patient (GDPR right of access)
#[utoipa::path(
    get,
    path = "/api/v1/patients/{id}/export",
    tag = "privacy",
    params(
        ("id" = Uuid, Path, description = "Patient UUID")
    ),
    responses(
        (status = 200, description = "Patient data export"),
        (status = 404, description = "Patient not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn export_patient_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(patient)) => {
            let export = crate::privacy::export_patient_data(&patient);
            (StatusCode::OK, Json(ApiResponse::success(export)))
        }
        Ok(None) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "NOT_FOUND",
                format!("Patient with id '{}' not found", id)
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<serde_json::Value>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve patient: {}", e)
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}

/// Get a patient with sensitive data masked
#[utoipa::path(
    get,
    path = "/api/v1/patients/{id}/masked",
    tag = "privacy",
    params(
        ("id" = Uuid, Path, description = "Patient UUID")
    ),
    responses(
        (status = 200, description = "Masked patient data"),
        (status = 404, description = "Patient not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_patient_masked(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(patient)) => {
            let masked = crate::privacy::mask_patient(&patient);
            (StatusCode::OK, Json(ApiResponse::success(masked)))
        }
        Ok(None) => {
            let error = ApiResponse::<Patient>::error(
                "NOT_FOUND",
                format!("Patient with id '{}' not found", id)
            );
            (StatusCode::NOT_FOUND, Json(error))
        }
        Err(e) => {
            let error = ApiResponse::<Patient>::error(
                "DATABASE_ERROR",
                format!("Failed to retrieve patient: {}", e)
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

/// Get audit logs for a specific patient
#[utoipa::path(
    get,
    path = "/api/v1/patients/{id}/audit",
    tag = "audit",
    params(
        ("id" = Uuid, Path, description = "Patient UUID"),
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "Audit logs retrieved successfully"),
        (status = 500, description = "Database error")
    )
)]
pub async fn get_patient_audit_logs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<AuditLogQuery>,
) -> impl IntoResponse {
    let limit = params.limit.min(500);

    match state.audit_log.get_logs_for_entity("Patient", id, limit as u64).await {
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
