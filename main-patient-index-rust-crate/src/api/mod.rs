//! API modules for REST, gRPC, and FHIR

pub mod rest;
pub mod grpc;
pub mod fhir;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

/// API error response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
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
