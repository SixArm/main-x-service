//! API plumbing — `ApiResponse<T>` envelope shared by REST handlers.

pub mod rest;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Standard JSON envelope. Exactly one of `data` / `error` is set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// `true` when `data` is set, `false` when `error` is set.
    pub success: bool,
    /// Payload on success; omitted from JSON when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error detail on failure; omitted from JSON when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    /// Build a successful envelope wrapping `data`.
    pub fn success(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    /// Build a failure envelope from a code + message (no details).
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError { code: code.into(), message: message.into(), details: None }),
        }
    }
    /// Build a failure envelope with a serialisable `details` payload.
    pub fn error_with_details<D: Serialize>(
        code: impl Into<String>,
        message: impl Into<String>,
        details: D,
    ) -> Self {
        let details = serde_json::to_value(details).ok();
        Self {
            success: false,
            data: None,
            error: Some(ApiError { code: code.into(), message: message.into(), details }),
        }
    }
}

/// Machine- and human-readable error detail carried by a failed
/// [`ApiResponse`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Short stable error code (e.g. `"validation_error"`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional structured detail (e.g. per-field validation errors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
