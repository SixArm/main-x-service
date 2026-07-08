//! Axum handlers for the FHIR R5 `/fhir/Practitioner` endpoints.
//!
//! Each handler bridges the FHIR wire format and the internal model: it
//! converts with [`to_fhir_worker`] / [`from_fhir_worker`], reuses the same
//! repository and search engine as the REST API via the shared [`AppState`],
//! and reports errors as a FHIR [`FhirOperationOutcome`] (rather than the REST
//! [`crate::api::ApiResponse`] envelope) so responses stay FHIR-conformant.
//! Every response carries `application/fhir+json`; search returns a FHIR
//! `Bundle` of type `searchset`, and `GET /fhir/metadata` returns the
//! `CapabilityStatement`. No doctests — these require a live [`AppState`].

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{FhirOperationOutcome, FhirWorker, from_fhir_worker, to_fhir_worker};
use crate::api::rest::AppState;

/// Builds an `application/fhir+json` [`Response`] with the given status and an
/// optional `Location` header.
fn fhir_response(status: StatusCode, body: Vec<u8>, location: Option<String>) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/fhir+json");
    if let Some(loc) = location {
        builder = builder.header(header::LOCATION, loc);
    }
    builder
        .body(Body::from(body))
        .expect("static headers + owned body always build a valid response")
}

/// Serializes `body` as a FHIR resource with the given status; a serialization
/// failure degrades to a `500` `OperationOutcome`.
fn fhir_json<T: Serialize>(status: StatusCode, body: &T) -> Response {
    match serde_json::to_vec(body) {
        Ok(bytes) => fhir_response(status, bytes, None),
        Err(_) => fhir_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            br#"{"resourceType":"OperationOutcome","issue":[{"severity":"error","code":"exception","diagnostics":"serialization failed"}]}"#.to_vec(),
            None,
        ),
    }
}

/// A FHIR error [`Response`]: an `OperationOutcome` with one issue.
fn fhir_error(status: StatusCode, code: &str, message: &str) -> Response {
    fhir_json(status, &FhirOperationOutcome::error(code, message))
}

/// Query parameters for `GET /fhir/Practitioner`, named per the FHIR
/// search-parameter spec (`name`, `family`, `given`, `identifier`, `birthdate`,
/// `gender`, `_id`, `_lastUpdated`, `_count`) via `#[serde(rename)]`.
#[derive(Debug, Deserialize)]
pub struct FhirSearchParams {
    /// Practitioner name (any part)
    #[serde(rename = "name")]
    pub name: Option<String>,

    /// Practitioner family name
    #[serde(rename = "family")]
    pub family: Option<String>,

    /// Practitioner given name
    #[serde(rename = "given")]
    pub given: Option<String>,

    /// Business identifier
    #[serde(rename = "identifier")]
    pub identifier: Option<String>,

    /// Birth date
    #[serde(rename = "birthdate")]
    pub birth_date: Option<String>,

    /// Administrative gender
    #[serde(rename = "gender")]
    pub gender: Option<String>,

    /// Number of results
    #[serde(rename = "_count")]
    pub count: Option<usize>,
}

/// `GET /fhir/Practitioner/{id}` — returns the worker as a FHIR
/// `Practitioner`, a `not-found` outcome (`404`), or a `database-error`
/// outcome (`500`).
pub async fn get_fhir_worker(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    match state.worker_repository.get_by_id(&id).await {
        Ok(Some(worker)) => fhir_json(StatusCode::OK, &to_fhir_worker(&worker)),
        Ok(None) => fhir_json(
            StatusCode::NOT_FOUND,
            &FhirOperationOutcome::not_found("Practitioner", &id.to_string()),
        ),
        Err(e) => fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "database-error", &e.to_string()),
    }
}

/// `POST /fhir/Practitioner` — parses the FHIR body into the internal model
/// (`400` invalid outcome on failure), assigns a UUID if absent, persists,
/// indexes, and returns the created resource (`201`) with a `Location` header.
pub async fn create_fhir_worker(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    let fhir_worker: FhirWorker = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(StatusCode::BAD_REQUEST, "structure", &format!("invalid FHIR JSON: {e}"));
        }
    };
    match from_fhir_worker(&fhir_worker) {
        Ok(mut worker) => {
            if worker.id == Uuid::nil() {
                worker.id = Uuid::new_v4();
            }
            match state.worker_repository.create(&worker).await {
                Ok(created_worker) => {
                    if let Err(e) = state.search_engine.index_worker(&created_worker) {
                        tracing::warn!("Failed to index worker in search engine: {}", e);
                    }
                    let pid = created_worker.id.to_string();
                    let resource = to_fhir_worker(&created_worker);
                    match serde_json::to_vec(&resource) {
                        Ok(bytes) => fhir_response(
                            StatusCode::CREATED,
                            bytes,
                            Some(format!("Practitioner/{pid}")),
                        ),
                        Err(e) => fhir_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "exception",
                            &e.to_string(),
                        ),
                    }
                }
                Err(e) => fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "database-error", &e.to_string()),
            }
        }
        Err(e) => fhir_error(StatusCode::BAD_REQUEST, "invalid", &e.to_string()),
    }
}

/// `PUT /fhir/Practitioner/{id}` — parses the body, forces its id to the path
/// id, updates, re-indexes, and returns the updated resource (`200`).
pub async fn update_fhir_worker(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: axum::body::Bytes,
) -> Response {
    let fhir_worker: FhirWorker = match serde_json::from_slice(&body) {
        Ok(f) => f,
        Err(e) => {
            return fhir_error(StatusCode::BAD_REQUEST, "structure", &format!("invalid FHIR JSON: {e}"));
        }
    };
    match from_fhir_worker(&fhir_worker) {
        Ok(mut worker) => {
            worker.id = id;
            match state.worker_repository.update(&worker).await {
                Ok(updated_worker) => {
                    if let Err(e) = state.search_engine.index_worker(&updated_worker) {
                        tracing::warn!("Failed to update worker in search engine: {}", e);
                    }
                    fhir_json(StatusCode::OK, &to_fhir_worker(&updated_worker))
                }
                Err(e) => fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "database-error", &e.to_string()),
            }
        }
        Err(e) => fhir_error(StatusCode::BAD_REQUEST, "invalid", &e.to_string()),
    }
}

/// `DELETE /fhir/Practitioner/{id}` — soft-deletes the worker, returning `204`
/// (empty body) or a `database-error` outcome (`500`).
pub async fn delete_fhir_worker(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    match state.worker_repository.delete(&id).await {
        Ok(()) => fhir_response(StatusCode::NO_CONTENT, Vec::new(), None),
        Err(e) => fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "database-error", &e.to_string()),
    }
}

/// `GET /fhir/Practitioner?...` — searches by the first provided name parameter
/// (`name`, else `family`, else `given`; `400` if none), hydrates hits, and
/// returns them as a FHIR `searchset` `Bundle`. `_count` caps the page (≤100).
pub async fn search_fhir_workers(
    State(state): State<AppState>,
    Query(params): Query<FhirSearchParams>,
) -> Response {
    let search_query = if let Some(ref name) = params.name {
        name.clone()
    } else if let Some(ref family) = params.family {
        family.clone()
    } else if let Some(ref given) = params.given {
        given.clone()
    } else {
        return fhir_json(
            StatusCode::BAD_REQUEST,
            &FhirOperationOutcome::invalid("At least one search parameter is required"),
        );
    };

    let limit = params.count.unwrap_or(10).min(100);

    match state.search_engine.search(&search_query, limit) {
        Ok(worker_ids) => {
            let mut fhir_entries = Vec::new();
            for worker_id_str in &worker_ids {
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
                            "fullUrl": format!("Practitioner/{}", worker.id),
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
            fhir_json(StatusCode::OK, &bundle)
        }
        Err(e) => fhir_error(StatusCode::INTERNAL_SERVER_ERROR, "search-error", &e.to_string()),
    }
}

/// `GET /fhir/metadata` — the `CapabilityStatement` declaring exactly the
/// interactions and search parameters this service implements (fhir.md §7).
/// Kept in sync with the mounted routes (a test pins the resource type +
/// params).
pub async fn metadata() -> Response {
    let statement = serde_json::json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "kind": "instance",
        "fhirVersion": "5.0.0",
        "format": ["application/fhir+json"],
        "rest": [{
            "mode": "server",
            "resource": [{
                "type": "Practitioner",
                "interaction": [
                    {"code": "read"},
                    {"code": "create"},
                    {"code": "update"},
                    {"code": "delete"},
                    {"code": "search-type"}
                ],
                "searchParam": [
                    {"name": "_id", "type": "token"},
                    {"name": "_lastUpdated", "type": "date"},
                    {"name": "_count", "type": "number"},
                    {"name": "identifier", "type": "token"},
                    {"name": "name", "type": "string"},
                    {"name": "family", "type": "string"},
                    {"name": "given", "type": "string"},
                    {"name": "gender", "type": "token"}
                ]
            }]
        }]
    });
    fhir_json(StatusCode::OK, &statement)
}
