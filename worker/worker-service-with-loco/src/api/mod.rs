//! API surface for the worker service: REST, gRPC, and FHIR.
//!
//! Submodules: [`rest`](crate::api::rest) (the primary Axum JSON API),
//! [`fhir`](crate::api::fhir) (HL7 FHIR R5 endpoints), and
//! [`grpc`](crate::api::grpc) (a real Tonic server — Create/Get/List/Delete
//! Worker). This module also defines the
//! shared [`ApiResponse`](crate::api::ApiResponse)/[`ApiError`](crate::api::ApiError)
//! envelope every REST handler returns,
//! giving clients a uniform `{ success, data, error }` shape.

/// HL7 FHIR R5 endpoints (the `Worker` resource).
pub mod fhir;
/// gRPC API (Tonic) — Create/Get/List/Delete Worker, real server.
pub mod grpc;
/// Primary REST JSON API (Axum) with OpenAPI/Swagger.
pub mod rest;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Uniform success/error envelope wrapping every REST response body.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// `true` for success responses, `false` when `error` is populated.
    pub success: bool,
    /// The payload on success; `None` on error.
    pub data: Option<T>,
    /// The error detail on failure; `None` on success.
    pub error: Option<ApiError>,
}

/// Machine- and human-readable error detail carried in [`ApiResponse::error`].
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Stable error code (e.g. `INTERNAL_ERROR`, `NOT_FOUND`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details (e.g. per-field validation errors).
    pub details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    /// Wraps `data` in a successful response (`success: true`, no error).
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Builds an error response (`success: false`, no data) with the given
    /// code and message and no structured details.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
        }
    }
}

impl<T> From<crate::Error> for ApiResponse<T> {
    /// Converts any [`crate::Error`] into a generic `INTERNAL_ERROR` response
    /// carrying the error's `Display` text.
    ///
    /// This is the catch-all fallback: it flattens every error variant to the
    /// same code, so handlers that need a more specific code/status (e.g.
    /// `NOT_FOUND` → 404, `VALIDATION_ERROR` → 422) build the [`ApiError`]
    /// explicitly rather than going through this `From`.
    fn from(err: crate::Error) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "INTERNAL_ERROR".to_string(),
                message: err.to_string(),
                details: None,
            }),
        }
    }
}
