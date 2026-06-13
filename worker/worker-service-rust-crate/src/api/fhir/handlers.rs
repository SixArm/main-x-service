//! Axum handlers for the FHIR R5 `/fhir/Worker` endpoints.
//!
//! Each handler bridges the FHIR wire format and the internal model: it
//! converts with [`to_fhir_worker`] / [`from_fhir_worker`], reuses the same
//! repository and search engine as the REST API via the shared [`AppState`],
//! and reports errors as a FHIR [`FhirOperationOutcome`] (rather than the REST
//! [`crate::api::ApiResponse`] envelope) so responses stay FHIR-conformant.
//! Search returns a FHIR `Bundle` of type `searchset`. No doctests — these
//! require a live [`AppState`].

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{FhirOperationOutcome, FhirWorker, from_fhir_worker, to_fhir_worker};
use crate::api::rest::AppState;

/// Query parameters for `GET /fhir/Worker`, named per the FHIR search-parameter
/// spec (`name`, `family`, `given`, `identifier`, `birthdate`, `gender`,
/// `_count`) via `#[serde(rename)]`.
#[derive(Debug, Deserialize)]
pub struct FhirSearchParams {
    /// Worker name (any part)
    #[serde(rename = "name")]
    pub name: Option<String>,

    /// Worker family name
    #[serde(rename = "family")]
    pub family: Option<String>,

    /// Worker given name
    #[serde(rename = "given")]
    pub given: Option<String>,

    /// Worker identifier
    #[serde(rename = "identifier")]
    pub identifier: Option<String>,

    /// Birth date
    #[serde(rename = "birthdate")]
    pub birth_date: Option<String>,

    /// Gender
    #[serde(rename = "gender")]
    pub gender: Option<String>,

    /// Number of results
    #[serde(rename = "_count")]
    pub count: Option<usize>,
}

/// `GET /fhir/Worker/{id}` — returns the worker as a FHIR resource, a
/// `not-found` outcome (`404`), or a `database-error` outcome (`500`).
pub async fn get_fhir_worker(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.worker_repository.get_by_id(&id).await {
        Ok(Some(worker)) => {
            let fhir_worker = to_fhir_worker(&worker);
            (
                StatusCode::OK,
                Json(serde_json::to_value(fhir_worker).unwrap()),
            )
        }
        Ok(None) => {
            let outcome = FhirOperationOutcome::not_found("Worker", &id.to_string());
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::to_value(outcome).unwrap()),
            )
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(outcome).unwrap()),
            )
        }
    }
}

/// `POST /fhir/Worker` — parses the FHIR body into the internal model
/// (`400` invalid outcome on failure), assigns a UUID if absent, persists,
/// indexes, and returns the created resource (`201`).
pub async fn create_fhir_worker(
    State(state): State<AppState>,
    Json(fhir_worker): Json<FhirWorker>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_worker(&fhir_worker) {
        Ok(mut worker) => {
            // Ensure worker has a UUID
            if worker.id == Uuid::nil() {
                worker.id = Uuid::new_v4();
            }

            // Insert into database
            match state.worker_repository.create(&worker).await {
                Ok(created_worker) => {
                    // Index in search engine
                    if let Err(e) = state.search_engine.index_worker(&created_worker) {
                        tracing::warn!("Failed to index worker in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_worker(&created_worker);
                    (
                        StatusCode::CREATED,
                        Json(serde_json::to_value(fhir_response).unwrap()),
                    )
                }
                Err(e) => {
                    let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::to_value(outcome).unwrap()),
                    )
                }
            }
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::invalid(&e.to_string());
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::to_value(outcome).unwrap()),
            )
        }
    }
}

/// `PUT /fhir/Worker/{id}` — parses the body, forces its id to the path id,
/// updates, re-indexes, and returns the updated resource (`200`).
pub async fn update_fhir_worker(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(fhir_worker): Json<FhirWorker>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_worker(&fhir_worker) {
        Ok(mut worker) => {
            // Ensure ID in path matches payload
            worker.id = id;

            // Update in database
            match state.worker_repository.update(&worker).await {
                Ok(updated_worker) => {
                    // Update in search index
                    if let Err(e) = state.search_engine.index_worker(&updated_worker) {
                        tracing::warn!("Failed to update worker in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_worker(&updated_worker);
                    (
                        StatusCode::OK,
                        Json(serde_json::to_value(fhir_response).unwrap()),
                    )
                }
                Err(e) => {
                    let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::to_value(outcome).unwrap()),
                    )
                }
            }
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::invalid(&e.to_string());
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::to_value(outcome).unwrap()),
            )
        }
    }
}

/// `DELETE /fhir/Worker/{id}` — soft-deletes the worker, returning `204`
/// (empty body) or a `database-error` outcome (`500`).
pub async fn delete_fhir_worker(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.worker_repository.delete(&id).await {
        Ok(()) => (StatusCode::NO_CONTENT, Json(serde_json::json!({}))),
        Err(e) => {
            let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(outcome).unwrap()),
            )
        }
    }
}

/// `GET /fhir/Worker?...` — searches by the first provided name parameter
/// (`name`, else `family`, else `given`; `400` if none), hydrates hits, and
/// returns them as a FHIR `searchset` `Bundle`. `_count` caps the page (≤100).
pub async fn search_fhir_workers(
    State(state): State<AppState>,
    Query(params): Query<FhirSearchParams>,
) -> impl IntoResponse {
    // Build search query from FHIR parameters
    let search_query = if let Some(ref name) = params.name {
        name.clone()
    } else if let Some(ref family) = params.family {
        family.clone()
    } else if let Some(ref given) = params.given {
        given.clone()
    } else {
        // No search criteria provided
        let outcome = FhirOperationOutcome::invalid("At least one search parameter is required");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::to_value(outcome).unwrap()),
        );
    };

    let limit = params.count.unwrap_or(10).min(100);

    // Search using search engine
    match state.search_engine.search(&search_query, limit) {
        Ok(worker_ids) => {
            // Fetch workers from database and convert to FHIR
            let mut fhir_entries = Vec::new();
            for worker_id_str in &worker_ids {
                // Parse string ID to UUID
                let worker_id = match Uuid::parse_str(worker_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse worker ID {}: {}", worker_id_str, e);
                        continue;
                    }
                };

                match state.worker_repository.get_by_id(&worker_id).await {
                    Ok(Some(worker)) => {
                        let fhir_worker = to_fhir_worker(&worker);
                        fhir_entries.push(serde_json::json!({
                            "fullUrl": format!("Worker/{}", worker.id),
                            "resource": fhir_worker
                        }));
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

            let bundle = serde_json::json!({
                "resourceType": "Bundle",
                "type": "searchset",
                "total": fhir_entries.len(),
                "entry": fhir_entries
            });
            (StatusCode::OK, Json(bundle))
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::error("search-error", &e.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(outcome).unwrap()),
            )
        }
    }
}
