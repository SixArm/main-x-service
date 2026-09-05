//! Axum controllers (loco `Routes`), one module per pillar, plus the
//! shared error helpers.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;

pub mod acquisition;
pub mod adjustments;
pub mod appraisals;
pub mod assessments;
pub mod audits;
pub mod development;
pub mod docs;
pub mod ergonomics;
pub mod hr_core;
pub mod intelligence;
pub mod learning;
pub mod metrics;
pub mod notifications;
pub mod payroll;
pub mod privacy;
pub mod talent;
pub mod wellbeing;
pub mod workforce;

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

// ─── Pagination (WPM-T40; agents/share/restful.md) ──────────────────────────

/// Largest page any collection read will serve. A bigger `limit` is
/// **clamped** to this rather than refused.
pub const MAX_LIMIT: u64 = 500;

/// Largest accepted `offset`; past this a request is a `400` — an
/// unbounded offset would otherwise make the database materialise
/// arbitrarily many rows only to discard them (SEC-G7). Deep paging
/// past this bound wants a cursor, not a bigger number.
pub const MAX_OFFSET: u64 = 10_000;

/// `?limit=` / `?offset=` for a collection read.
///
/// Declared as its own type (not `#[serde(flatten)]`-ed onto a wider
/// query struct): a flattened struct deserializes from a string-keyed
/// map, so `limit=2` would arrive as the string `"2"` and fail to parse
/// as a `u64` — a spurious `400` on a valid request.
#[derive(Debug, Default, Clone, Copy, serde::Deserialize)]
pub struct Page {
    /// Page size; absent, zero, or unparseable ⇒ the endpoint default.
    #[serde(default)]
    pub limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    pub offset: Option<u64>,
}

impl Page {
    /// The clamped `(limit, offset)` this request will actually use. A
    /// zero `limit` falls back to `default_limit`: an empty page and an
    /// empty collection look identical to a client, and only one is an
    /// answer.
    #[must_use]
    pub fn resolve(self, default_limit: u64) -> (u64, u64) {
        let limit = self
            .limit
            .filter(|l| *l > 0)
            .unwrap_or(default_limit)
            .min(MAX_LIMIT);
        (limit, self.offset.unwrap_or(0))
    }

    /// Reject an out-of-bound offset before it reaches the database.
    ///
    /// # Errors
    ///
    /// Returns a `400` when the offset exceeds [`MAX_OFFSET`].
    pub fn check_offset(self) -> Result<()> {
        if self.offset.unwrap_or(0) > MAX_OFFSET {
            return Err(Error::CustomError(
                StatusCode::BAD_REQUEST,
                ErrorDetail::new(
                    "offset_too_large",
                    format!("offset must not exceed {MAX_OFFSET}; narrow the query instead"),
                ),
            ));
        }
        Ok(())
    }
}

/// Stamp `X-Total-Count` / `X-Limit` / `X-Offset` onto a response
/// (`agents/share/restful.md`).
#[must_use]
pub fn with_page_headers(mut response: Response, total: u64, limit: u64, offset: u64) -> Response {
    let headers = response.headers_mut();
    for (name, value) in [
        ("x-total-count", total),
        ("x-limit", limit),
        ("x-offset", offset),
    ] {
        if let Ok(value) = axum::http::HeaderValue::from_str(&value.to_string()) {
            headers.insert(name, value);
        }
    }
    response
}
