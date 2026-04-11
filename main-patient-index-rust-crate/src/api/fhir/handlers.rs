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
use super::{FhirPatient, FhirOperationOutcome, to_fhir_patient, from_fhir_patient};

/// FHIR search parameters
#[derive(Debug, Deserialize)]
pub struct FhirSearchParams {
    /// Patient name (any part)
    #[serde(rename = "name")]
    pub name: Option<String>,

    /// Patient family name
    #[serde(rename = "family")]
    pub family: Option<String>,

    /// Patient given name
    #[serde(rename = "given")]
    pub given: Option<String>,

    /// Patient identifier
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

/// Get FHIR Patient by ID
pub async fn get_fhir_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(patient)) => {
            let fhir_patient = to_fhir_patient(&patient);
            (StatusCode::OK, Json(serde_json::to_value(fhir_patient).unwrap()))
        }
        Ok(None) => {
            let outcome = FhirOperationOutcome::not_found("Patient", &id.to_string());
            (StatusCode::NOT_FOUND, Json(serde_json::to_value(outcome).unwrap()))
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}

/// Create FHIR Patient
pub async fn create_fhir_patient(
    State(state): State<AppState>,
    Json(fhir_patient): Json<FhirPatient>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_patient(&fhir_patient) {
        Ok(mut patient) => {
            // Ensure patient has a UUID
            if patient.id == Uuid::nil() {
                patient.id = Uuid::new_v4();
            }

            // Insert into database
            match state.patient_repository.create(&patient).await {
                Ok(created_patient) => {
                    // Index in search engine
                    if let Err(e) = state.search_engine.index_patient(&created_patient) {
                        tracing::warn!("Failed to index patient in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_patient(&created_patient);
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

/// Update FHIR Patient
pub async fn update_fhir_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(fhir_patient): Json<FhirPatient>,
) -> impl IntoResponse {
    // Convert FHIR to internal model
    match from_fhir_patient(&fhir_patient) {
        Ok(mut patient) => {
            // Ensure ID in path matches payload
            patient.id = id;

            // Update in database
            match state.patient_repository.update(&patient).await {
                Ok(updated_patient) => {
                    // Update in search index
                    if let Err(e) = state.search_engine.index_patient(&updated_patient) {
                        tracing::warn!("Failed to update patient in search engine: {}", e);
                    }

                    let fhir_response = to_fhir_patient(&updated_patient);
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

/// Delete FHIR Patient
pub async fn delete_fhir_patient(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.patient_repository.delete(&id).await {
        Ok(()) => {
            (StatusCode::NO_CONTENT, Json(serde_json::json!({})))
        }
        Err(e) => {
            let outcome = FhirOperationOutcome::error("database-error", &e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::to_value(outcome).unwrap()))
        }
    }
}

/// Search FHIR Patients
pub async fn search_fhir_patients(
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
        Ok(patient_ids) => {
            // Fetch patients from database and convert to FHIR
            let mut fhir_entries = Vec::new();
            for patient_id_str in &patient_ids {
                // Parse string ID to UUID
                let patient_id = match Uuid::parse_str(patient_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("Failed to parse patient ID {}: {}", patient_id_str, e);
                        continue;
                    }
                };

                match state.patient_repository.get_by_id(&patient_id).await {
                    Ok(Some(patient)) => {
                        let fhir_patient = to_fhir_patient(&patient);
                        fhir_entries.push(serde_json::json!({
                            "fullUrl": format!("Patient/{}", patient.id),
                            "resource": fhir_patient
                        }));
                    }
                    Ok(None) => {
                        tracing::warn!("Patient {} found in search index but not in database", patient_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch patient {}: {}", patient_id, e);
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
