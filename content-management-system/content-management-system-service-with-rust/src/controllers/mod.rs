//! Axum controllers (loco `Routes`), one module per CMS area, plus the
//! shared error helpers.
//!
//! Mounted so far: [`sites`] (sites + templates), [`types`] (content
//! types), [`entries`] (entries, variants, revisions, diff/restore,
//! usage, publish-check), [`assets`] (the library, renditions,
//! orphans), [`workflow`] (transitions, publishing, scheduling,
//! locks), [`localization`] (locale resolution, the translation
//! workflow, staleness), [`routing`] (addresses, redirects, menus,
//! audience rules), [`delivery`] (the public surface), [`insights`]
//! (content health and editorial throughput), [`audits`], [`docs`],
//! [`preview`] (the one credential that shows unpublished content),
//! [`webhooks`] (the only extension mechanism this service has),
//! [`audits`], [`docs`], and [`metrics`]. Webhooks arrive with
//! CMS-T23.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;

pub mod assets;
pub mod audits;
pub mod delivery;
pub mod docs;
pub mod entries;
pub mod insights;
pub mod localization;
pub mod metrics;
pub mod preview;
pub mod routing;
pub mod sites;
pub mod types;
pub mod webhooks;
pub mod workflow;

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

/// A `422` carrying several problems (the same shape [`ensure_valid`]
/// produces, for callers that already hold a problem list).
#[must_use]
pub fn validation_error(problems: &[String]) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", problems.join("; ")),
    )
}

/// Turn a record-level authorization refusal into an HTTP error.
///
/// `401` and `403` keep their meanings from the family's ABAC contract
/// — missing credential versus valid credential the policy denied —
/// and the reason names the deciding rule so an operator can find it
/// in the policy rather than guessing.
#[must_use]
pub fn authz_error((status, reason): (StatusCode, String)) -> Error {
    Error::CustomError(status, ErrorDetail::new("forbidden", &reason))
}

/// A `422` with a single message (state-machine refusals etc.).
#[must_use]
pub fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

/// A `409` — a uniqueness clash, or a delete refused because something
/// still references the target (CMS-D8). The message names what is in
/// the way, because "conflict" alone tells an operator nothing.
#[must_use]
pub fn conflict(message: &str) -> Error {
    Error::CustomError(StatusCode::CONFLICT, ErrorDetail::new("conflict", message))
}

/// A weak `ETag` over a payload, **excluding `as_of`** so an unchanged
/// answer keeps its tag as the clock moves, plus an optional salt for
/// anything else the response varies by (the personalization context,
/// in delivery).
///
/// Weak rather than strong because the payload is JSON whose key order
/// is stable but whose byte-for-byte identity is not a promise worth
/// making.
#[must_use]
pub fn weak_etag(payload: &serde_json::Value, salt: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let mut copy = payload.clone();
    if let Some(object) = copy.as_object_mut() {
        object.remove("as_of");
    }
    let mut hasher = Sha256::new();
    hasher.update(copy.to_string().as_bytes());
    hasher.update(salt.as_bytes());
    let hex = hasher
        .finalize()
        .iter()
        .take(16)
        .fold(String::new(), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        });
    format!("W/\"{hex}\"")
}

/// Whether the request's `If-None-Match` matches `tag`.
#[must_use]
pub fn matches_etag(headers: &axum::http::HeaderMap, tag: &str) -> bool {
    headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|candidate| candidate.trim() == tag))
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
