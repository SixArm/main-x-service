//! The uniform `{ success, data, error }` API envelope (spec §9).
//!
//! Every `/api` response is wrapped so callers get one shape on success
//! and failure. Graph responses additionally carry an `as_of` watermark,
//! which lives inside the per-endpoint `data` payload (spec §6 FR-17).

use axum::response::IntoResponse;
use loco_rs::prelude::*;
use serde::Serialize;

/// The standard response wrapper. `data` is present on success; `error`
/// carries a human-readable message on failure.
#[derive(Debug, Serialize)]
pub struct ApiEnvelope<T> {
    /// Whether the request succeeded.
    pub success: bool,
    /// The payload (present iff `success`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// The error message (present iff `!success`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A `200` success envelope wrapping `data`.
///
/// # Errors
///
/// Propagates serialization errors from [`format::json`].
pub fn ok<T: Serialize>(data: T) -> Result<Response> {
    format::json(ApiEnvelope {
        success: true,
        data: Some(data),
        error: None,
    })
}

/// A `400 Bad Request` error envelope (malformed ref, unknown kind, or
/// depth over the cap — spec §9.2).
///
/// # Errors
///
/// Infallible in practice; the `Result` matches the handler signature.
pub fn bad_request(message: &str) -> Result<Response> {
    let body = ApiEnvelope::<()> {
        success: false,
        data: None,
        error: Some(message.to_string()),
    };
    Ok((axum::http::StatusCode::BAD_REQUEST, axum::Json(body)).into_response())
}
