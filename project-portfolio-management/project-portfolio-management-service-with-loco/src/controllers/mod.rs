//! HTTP controllers for the portfolio service.

pub mod docs;
pub mod governance;
pub mod metrics;
pub mod work_items;

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
pub fn record_rejection(
    (status, reason): (axum::http::StatusCode, String),
) -> loco_rs::Error {
    let code = if status == axum::http::StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    loco_rs::Error::CustomError(status, loco_rs::controller::ErrorDetail::new(code, &reason))
}
