//! Axum controllers (loco `Routes`), one module per CRM module, plus
//! the shared error helpers.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;

pub mod audits;
pub mod dashboards;
pub mod docs;
pub mod engagement;
pub mod insights;
pub mod marketing;
pub mod metrics;
pub mod privacy;
pub mod relationships;
pub mod sales;
pub mod support;

/// Map a non-empty problem list to the family's `422 Unprocessable
/// Entity` validation error (every problem in one response).
///
/// # Errors
///
/// Returns the `422` error when `problems` is non-empty.
pub fn ensure_valid(problems: &[String]) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", problems.join("; ")),
    ))
}

/// A `422` with a single message (state-machine refusals etc.).
#[must_use]
pub fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

/// Map a record-level authorization rejection (`(status, reason)`) to a
/// loco error response: `403` = policy denied; `401` = fail-safe when
/// claims are missing behind the guard.
#[must_use]
pub fn record_rejection((status, reason): (StatusCode, String)) -> Error {
    let code = if status == StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    Error::CustomError(status, ErrorDetail::new(code, &reason))
}
