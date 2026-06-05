//! API plumbing — `ApiResponse<T>` envelope shared by REST handlers.

pub mod rest;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Standard JSON envelope. Always exactly one of `data` / `error` is
/// set. `success = error.is_none()`. Swagger sees the inner `data`
/// type directly via `#[utoipa::path(responses(body = T))]`; the
/// envelope itself is internal plumbing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError { code: code.into(), message: message.into(), details: None }),
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
