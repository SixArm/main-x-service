//! HTTP controllers for the portfolio service.

pub mod automation;
pub mod collaboration;
/// Integrity-verification endpoints.
pub mod compliance;
pub mod docs;
pub mod engineering;
pub mod governance;
pub mod insights;
pub mod metrics;
pub mod oversight;
pub mod plans;
pub mod prioritisation;
pub mod strategy;
pub mod visibility;

/// Map a model-layer error to its HTTP shape: a missing record is
/// `404 Not Found`; anything else stays a model error (500-class).
/// loco 0.16 stopped mapping `ModelError::EntityNotFound` itself (its
/// `IntoResponse` catch-all turns it into a 500), so every controller
/// lookup routes through this instead of a bare `?`.
#[must_use]
pub fn model_not_found(err: loco_rs::model::ModelError) -> loco_rs::Error {
    match err {
        loco_rs::model::ModelError::EntityNotFound => loco_rs::Error::NotFound,
        other => loco_rs::Error::Model(other),
    }
}

/// Map a record-level authorization rejection (`(status, reason)`) to
/// a loco error: `403` = policy denied; `401` = fail-safe when claims
/// are missing behind the guard.
#[must_use]
pub fn record_rejection((status, reason): (axum::http::StatusCode, String)) -> loco_rs::Error {
    let code = if status == axum::http::StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    loco_rs::Error::CustomError(status, loco_rs::controller::ErrorDetail::new(code, &reason))
}

/// Weak `ETag` over a serializable view (everything except `as_of`).
#[must_use]
pub fn etag_of<T: serde::Serialize>(value: &T) -> String {
    use std::hash::{Hash, Hasher};
    let json = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    format!("W/\"{:016x}\"", hasher.finish())
}

/// Serve `body` as JSON with `etag`, honouring `If-None-Match` → `304`
/// (the patient-flow conditional-read pattern).
///
/// # Errors
///
/// When the body fails to serialize.
pub fn conditional_json<T: serde::Serialize>(
    headers: &axum::http::HeaderMap,
    etag: &str,
    body: &T,
) -> loco_rs::Result<axum::response::Response> {
    use axum::http::{StatusCode, header};
    use axum::response::IntoResponse;
    let matched = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == etag);
    let etag_value = header::HeaderValue::from_str(etag)
        .unwrap_or_else(|_| header::HeaderValue::from_static("W/\"0\""));
    if matched {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag_value)]).into_response());
    }
    Ok((
        StatusCode::OK,
        [(header::ETAG, etag_value)],
        axum::Json(serde_json::to_value(body).map_err(|e| loco_rs::Error::Any(e.into()))?),
    )
        .into_response())
}
