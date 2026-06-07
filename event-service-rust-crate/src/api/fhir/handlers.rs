//! Stub FHIR R5 handlers. Each handler returns 501 with an
//! `OperationOutcome`-shaped JSON body explaining that the
//! mapping isn't decided yet. See `mod.rs` for the rationale.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

/// Build a `501 Not Implemented` response with a FHIR
/// `OperationOutcome` body carrying `reason` as the diagnostics.
fn not_implemented(reason: &str) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "resourceType": "OperationOutcome",
            "issue": [{
                "severity": "error",
                "code": "not-supported",
                "diagnostics": reason,
            }],
        })),
    )
}

/// Stub `GET` for a FHIR Event — returns `501`.
pub async fn get_event() -> impl IntoResponse {
    not_implemented("FHIR Event GET not implemented yet — see src/api/fhir/mod.rs")
}

/// Stub `POST` (create) for a FHIR Event — returns `501`.
pub async fn create_event() -> impl IntoResponse {
    not_implemented("FHIR Event POST not implemented yet — see src/api/fhir/mod.rs")
}

/// Stub `PUT` (update) for a FHIR Event — returns `501`.
pub async fn update_event() -> impl IntoResponse {
    not_implemented("FHIR Event PUT not implemented yet — see src/api/fhir/mod.rs")
}

/// Stub `DELETE` for a FHIR Event — returns `501`.
pub async fn delete_event() -> impl IntoResponse {
    not_implemented("FHIR Event DELETE not implemented yet — see src/api/fhir/mod.rs")
}

/// Stub search for FHIR Events — returns `501`.
pub async fn search_events() -> impl IntoResponse {
    not_implemented("FHIR Event search not implemented yet — see src/api/fhir/mod.rs")
}
