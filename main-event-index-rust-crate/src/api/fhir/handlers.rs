//! FHIR R5 API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::rest::AppState;
use super::{FhirEvent, FhirOperationOutcome, to_fhir_event, from_fhir_event};

/// FHIR search parameters
#[derive(Debug, Deserialize)]
pub struct FhirSearchParams {
    /// Event name (any part)
    #[serde(rename = "name")]
    pub name: Option<String>,

    /// Event family name
    #[serde(rename = "family")]
    pub family: Option<String>,

    /// Event given name
    #[serde(rename = "given")]
    pub given: Option<String>,

    /// Event identifier
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

/// Get FHIR Event by ID
pub async fn get_fhir_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.event_repository.get_by_id(&id).await {
        Ok(Some(event)) => {
            let fhir_event = to_fhir_event(&event);
            (StatusCode::OK, Json(serde_json::to_value(fhir_event).unwrap()))
        }
        Ok(None) => {
            let outcome = FhirOperationOutcome::not_found("Event", &id.to_string());
            (StatusCode::NOT_FOUND, Json(serde_json::to_value(outcome).unwrap()))
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}

/// Create FHIR Event
pub async fn create_fhir_event(
    State(state): State<AppState>,
    Json(fhir_event): Json<FhirEvent>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_event(&fhir_event) {
        Ok(mut event) => {
            // Ensure event has a UUID
            if event.id == Uuid::nil() {
                event.id = Uuid::new_v4();
            }

            // Insert into database
            match state.event_repository.create(&event).await {
                Ok(created_event) => {
                    // Index in search engine
                    if let Err(e) = state.search_engine.index_event(&created_event) {
                        tracing::warn!("Failed to index event in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_event(&created_event);
                    (StatusCode::CREATED, Json(serde_json::to_value(fhir_response).unwrap()))
                }
                Err(e) => {
                    let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
                }
            }
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::invalid(&e.to_string());
            (StatusCode::BAD_REQUEST, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}

/// Update FHIR Event
pub async fn update_fhir_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(fhir_event): Json<FhirEvent>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_event(&fhir_event) {
        Ok(mut event) => {
            // Ensure ID in path matches payload
            event.id = id;

            // Update in database
            match state.event_repository.update(&event).await {
                Ok(updated_event) => {
                    // Update in search index
                    if let Err(e) = state.search_engine.index_event(&updated_event) {
                        tracing::warn!("Failed to update event in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_event(&updated_event);
                    (StatusCode::OK, Json(serde_json::to_value(fhir_response).unwrap()))
                }
                Err(e) => {
                    let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
                }
            }
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::invalid(&e.to_string());
            (StatusCode::BAD_REQUEST, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}

/// Delete FHIR Event
pub async fn delete_fhir_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.event_repository.delete(&id).await {
        Ok(()) => {
            (StatusCode::NO_CONTENT, Json(serde_json::json!({})))
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}

/// Search FHIR Events
pub async fn search_fhir_events(
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
        return (StatusCode::BAD_REQUEST, Json(serde_json::to_value(outcome).unwrap()));
    };

    let limit = params.count.unwrap_or(10).min(100);

    // Search using search engine
    match state.search_engine.search(&search_query, limit) {
        Ok(event_ids) => {
            // Fetch events from database and convert to FHIR
            let mut fhir_entries = Vec::new();
            for event_id_str in &event_ids {
                // Parse string ID to UUID
                let event_id = match Uuid::parse_str(event_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse event ID {}: {}", event_id_str, e);
                        continue;
                    }
                };

                match state.event_repository.get_by_id(&event_id).await {
                    Ok(Some(event)) => {
                        let fhir_event = to_fhir_event(&event);
                        fhir_entries.push(serde_json::json!({
                            "fullUrl": format!("Event/{}", event.id),
                            "resource": fhir_event
                        }));
                    }
                    Ok(None) => {
                        tracing::warn!("Event {} found in search index but not in database", event_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch event {}: {}", event_id, e);
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
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}
