//! REST handlers. STUBS — every route returns `501 Not Implemented`
//! with the standard `ApiResponse` envelope so the front-end can be
//! developed against the actual error shape.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::api::ApiResponse;
use super::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// Health check — always returns `200 healthy` so orchestrators can
/// distinguish "process is up" from "process can talk to DB".
pub async fn health(State(_state): State<AppState>) -> impl IntoResponse {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy",
        service: "course-service",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

/// Catch-all stub handler — the real implementations land alongside
/// each task in `spec.md §13`.
pub async fn not_implemented(State(_state): State<AppState>) -> impl IntoResponse {
    let body: ApiResponse<()> = ApiResponse::error(
        "NOT_IMPLEMENTED",
        "Endpoint not yet implemented — see spec.md §13 for status.",
    );
    (StatusCode::NOT_IMPLEMENTED, Json(body))
}
