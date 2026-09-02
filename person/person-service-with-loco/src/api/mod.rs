//! API surface: REST (Axum), FHIR R5, and gRPC, plus the shared
//! [`ApiResponse`](crate::api::ApiResponse) envelope.
//!
//! Every REST/FHIR endpoint returns an [`ApiResponse`](crate::api::ApiResponse)`<T>`, a uniform
//! `{ success, data, error }` JSON wrapper, so clients can branch on one
//! shape. [`ApiError`](crate::api::ApiError) is the structured error body. The [`From`] impls
//! map a [`crate::Error`] into a failed response. Submodules: [`rest`](crate::api::rest)
//! (the primary HTTP API + Swagger), [`fhir`](crate::api::fhir) (HL7 FHIR R5 Person), and
//! [`grpc`](crate::api::grpc) (a real Tonic server — Create/Get/List/Delete Person).

/// HL7 FHIR R5 API for the Person resource.
pub mod fhir;
/// gRPC API (Tonic) — Create/Get/List/Delete Person, real server.
pub mod grpc;
/// REST API: router, handlers, state, and OpenAPI/Swagger.
pub mod rest;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Uniform success/error envelope returned by every REST/FHIR endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// `true` for a successful response, `false` for an error.
    pub success: bool,
    /// The payload on success; `None` on error.
    pub data: Option<T>,
    /// The error detail on failure; `None` on success.
    pub error: Option<ApiError>,
}

/// Structured error body carried inside a failed [`ApiResponse`].
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Stable machine-readable error code (e.g. `INTERNAL_ERROR`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured context (e.g. per-field validation details).
    pub details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    /// Wrap `data` in a successful response.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Build a failed response with the given `code` and `message`
    /// (no extra details).
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
    /// Convert any [`crate::Error`] into an `INTERNAL_ERROR` response.
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
