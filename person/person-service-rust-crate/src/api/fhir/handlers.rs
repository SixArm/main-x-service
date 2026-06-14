//! FHIR R5 API handlers for the `Person` resource.
//!
//! Axum handlers backing the `/fhir/Person` endpoints (get/create/
//! update/delete/search). Each handler shares the same [`AppState`](crate::api::rest::state::AppState) as
//! the REST API and bridges the wire <-> domain boundary via
//! [`to_fhir_person`](crate::api::fhir::to_fhir_person)/[`from_fhir_person`](crate::api::fhir::from_fhir_person) (in [`super`](crate::api::fhir)). Errors are
//! reported as FHIR [`FhirOperationOutcome`](crate::api::fhir::FhirOperationOutcome) resources with appropriate
//! HTTP status codes (404 not-found, 400 invalid, 500 database/search).
//! Create/update also keep the Tantivy search index in sync, logging a
//! warning (not failing the request) on index errors.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use super::{FhirOperationOutcome, FhirPerson, from_fhir_person, to_fhir_person};
use crate::api::rest::AppState;

/// Query-string parameters for the FHIR `Person` search endpoint.
///
/// Mirrors the FHIR search-parameter names (renamed via serde, since
/// `_count` and `birthdate` are not valid Rust identifiers). Only a
/// subset is supported; see [`super::search_parameters`] for the
/// reserved expansion point.
#[derive(Debug, Deserialize)]
pub struct FhirSearchParams {
    /// Person name (any part)
    #[serde(rename = "name")]
    pub name: Option<String>,

    /// Person family name
    #[serde(rename = "family")]
    pub family: Option<String>,

    /// Person given name
    #[serde(rename = "given")]
    pub given: Option<String>,

    /// Person identifier
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

/// `GET /fhir/Person/{id}` — fetch a person and render it as FHIR.
///
/// Returns `200` with the FHIR Person JSON on success, `404` with a
/// `not-found` `OperationOutcome` if no such person exists, or `500` with
/// a `database-error` `OperationOutcome` on a repository failure.
///
/// # Panics
///
/// Panics only if serializing the FHIR response body to JSON fails,
/// which cannot happen for these statically-typed payloads.
pub async fn get_fhir_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.person_repository.get_by_id(&id).await {
        Ok(Some(person)) => {
            let fhir_person = to_fhir_person(&person);
            (
                StatusCode::OK,
                Json(serde_json::to_value(fhir_person).unwrap()),
            )
        }
        Ok(None) => {
            let outcome = FhirOperationOutcome::not_found("Person", &id.to_string());
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

/// `POST /fhir/Person` — create a person from a FHIR Person payload.
///
/// Converts the FHIR resource to the domain model, assigns a fresh UUID
/// if the payload omits one, persists it, and indexes it for search
/// (a search-index failure is logged but does not fail the request).
/// Returns `201` with the created FHIR Person, `400` for an invalid
/// payload, or `500` on a database error.
///
/// # Panics
///
/// Panics only if serializing the FHIR response body to JSON fails,
/// which cannot happen for these statically-typed payloads.
pub async fn create_fhir_person(
    State(state): State<AppState>,
    Json(fhir_person): Json<FhirPerson>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_person(&fhir_person) {
        Ok(mut person) => {
            // Ensure person has a UUID
            if person.id == Uuid::nil() {
                person.id = Uuid::new_v4();
            }

            // Insert into database
            match state.person_repository.create(&person).await {
                Ok(created_person) => {
                    // Index in search engine
                    if let Err(e) = state.search_engine.index_person(&created_person) {
                        tracing::warn!("Failed to index person in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_person(&created_person);
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

/// `PUT /fhir/Person/{id}` — replace a person from a FHIR payload.
///
/// The path `id` is authoritative and overrides any id in the body.
/// Updates the database record and refreshes the search index (index
/// failures are logged, not fatal). Returns `200` with the updated FHIR
/// Person, `400` for an invalid payload, or `500` on a database error.
///
/// # Panics
///
/// Panics only if serializing the FHIR response body to JSON fails,
/// which cannot happen for these statically-typed payloads.
pub async fn update_fhir_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(fhir_person): Json<FhirPerson>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_person(&fhir_person) {
        Ok(mut person) => {
            // Ensure ID in path matches payload
            person.id = id;

            // Update in database
            match state.person_repository.update(&person).await {
                Ok(updated_person) => {
                    // Update in search index
                    if let Err(e) = state.search_engine.index_person(&updated_person) {
                        tracing::warn!("Failed to update person in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_person(&updated_person);
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

/// `DELETE /fhir/Person/{id}` — soft-delete a person.
///
/// Delegates to the repository's soft delete (the record is retained for
/// audit). Returns `204 No Content` on success or `500` with a
/// `database-error` `OperationOutcome` on failure.
///
/// # Panics
///
/// Panics only if serializing the FHIR response body to JSON fails,
/// which cannot happen for these statically-typed payloads.
pub async fn delete_fhir_person(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.person_repository.delete(&id).await {
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

/// `GET /fhir/Person?...` — search persons and return a FHIR Bundle.
///
/// Picks the first provided text criterion (`name`, then `family`, then
/// `given`) as the search query and caps `_count` at 100 (default 10).
/// Runs the Tantivy search, fetches each hit from the database, and
/// wraps the results in a `searchset` Bundle. Index entries missing
/// from the database are skipped with a warning. Returns `400` if no
/// search parameter is supplied, or `500` on a search-engine error.
///
/// # Panics
///
/// Panics only if serializing the FHIR response body to JSON fails,
/// which cannot happen for these statically-typed payloads.
pub async fn search_fhir_persons(
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
        Ok(person_ids) => {
            // Fetch persons from database and convert to FHIR
            let mut fhir_entries = Vec::new();
            for person_id_str in &person_ids {
                // Parse string ID to UUID
                let person_id = match Uuid::parse_str(person_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse person ID {}: {}", person_id_str, e);
                        continue;
                    }
                };

                match state.person_repository.get_by_id(&person_id).await {
                    Ok(Some(person)) => {
                        let fhir_person = to_fhir_person(&person);
                        fhir_entries.push(serde_json::json!({
                            "fullUrl": format!("Person/{}", person.id),
                            "resource": fhir_person
                        }));
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
