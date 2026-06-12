//! API layer: REST (Axum), gRPC (Tonic stub), and FHIR (stub).
//!
//! This module defines the uniform JSON response envelope used by the
//! REST handlers — [`ApiResponse`](crate::api::ApiResponse) (success or
//! error) wrapping an [`ApiError`](crate::api::ApiError) payload — and a
//! `From<Error>` bridge so any crate error can be returned as a
//! `500`-style envelope.
//!
//! # Examples
//!
//! ```
//! use event_service::api::ApiResponse;
//!
//! let ok = ApiResponse::success(42);
//! assert!(ok.success);
//! assert_eq!(ok.data, Some(42));
//!
//! let err: ApiResponse<i32> = ApiResponse::error("NOT_FOUND", "missing");
//! assert!(!err.success);
//! assert_eq!(err.error.unwrap().code, "NOT_FOUND");
//! ```

/// FHIR R5 API — currently a stub returning `OperationOutcome`.
pub mod fhir;
/// gRPC API (Tonic) — currently a stub.
pub mod grpc;
/// REST API (Axum): router, handlers, routes, shared state.
pub mod rest;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Uniform success/error envelope returned by REST handlers.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// `true` for a success response, `false` for an error.
    pub success: bool,
    /// The payload on success; `None` on error.
    pub data: Option<T>,
    /// The error detail on failure; `None` on success.
    pub error: Option<ApiError>,
}

/// Machine-readable error detail carried inside an [`ApiResponse`].
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Stable error code (e.g. `NOT_FOUND`, `VALIDATION_ERROR`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details (field errors, etc.).
    pub details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    /// Wrap `data` in a successful response (`success = true`).
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Build an error response from a code + message (no `data`).
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
    /// Convert any crate [`Error`](crate::Error) into an error envelope
    /// with code `INTERNAL_ERROR` and the error's `Display` as message.
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
