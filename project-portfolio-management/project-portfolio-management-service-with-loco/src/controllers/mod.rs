//! HTTP controllers for the portfolio service.

pub mod automation;
/// Sprint ceremonies and the commitment snapshot.
pub mod ceremony;
pub mod collaboration;
/// Integrity-verification endpoints.
pub mod compliance;
/// The Controlling-process register: standards, readings, actions, and
/// the coverage report of what is **not** being controlled.
pub mod controls;
/// Flow Distribution: the feature / defect / risk / debt mix.
pub mod distribution;
pub mod docs;
/// Recorded effort and utilisation, including per person.
pub mod effort;
pub mod engineering;
pub mod governance;
pub mod insights;
pub mod metrics;
/// The OKR engine: key results, check-ins, and the derived objective
/// and alignment-weighted plan scores.
pub mod okr;
pub mod oversight;
/// The sequential project phase: one-step advancement, explicitly
/// reasoned regression, and per-phase durations.
pub mod phase;
pub mod plans;
pub mod prioritisation;
pub mod strategy;
/// Time-based analysis: the read surface over the task transition log —
/// per-task and plan flow, constraints, aging WIP, and Little's Law.
pub mod tba;
/// Total Project Control: DIPP, expected monetary value, cost estimate
/// to complete, and portfolio triage by DIPP.
pub mod tpc;
/// Realized gains and strategic performance.
pub mod value;
pub mod visibility;
/// Custom workflows: the configurable task and issue state
/// vocabularies, and the resolution of which one is in force.
pub mod workflow;

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
    loco_rs::Error::CustomError(status, loco_rs::controller::ErrorDetail::new(code, reason))
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

/// Family-wide `?limit=`/`?offset=` collection-read pagination
/// (`agents/share/restful.md`): headers, not an envelope, so the body
/// stays the bare array every caller already parses.
///
/// Introduced alongside `GET /api/plans`' pagination (2026-08-01) and
/// promoted here so every other collection-read controller (automation
/// runs, the deadline queue, delegations, approvals, …) shares one
/// implementation instead of five copies drifting apart.
pub mod pagination {
    use axum::response::Response;
    use loco_rs::controller::ErrorDetail;

    /// Largest page any collection read will serve; a bigger `limit` is
    /// clamped rather than refused.
    pub const MAX_LIMIT: u64 = 500;

    /// Largest accepted `offset`; past this a request is a `400` (SEC-G7)
    /// rather than a query that makes the database materialise
    /// arbitrarily many rows just to discard them.
    pub const MAX_OFFSET: u64 = 10_000;

    /// `?limit=` / `?offset=` on a collection read.
    ///
    /// Declare the two fields directly on each query-parameter struct
    /// (as here) rather than `#[serde(flatten)]`-ing this type in:
    /// flattening deserializes via a string-keyed map, so `limit=2`
    /// arrives as the string `"2"` and fails to parse as a `u64` — a
    /// `400` on an otherwise-valid request.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct Page {
        /// Page size; `None`/zero ⇒ the endpoint default.
        pub limit: Option<u64>,
        /// Rows to skip; `None` ⇒ 0.
        pub offset: Option<u64>,
    }

    impl Page {
        /// The clamped `(limit, offset)` this request will use.
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
        /// `400` when the offset exceeds [`MAX_OFFSET`].
        pub fn check_offset(self) -> loco_rs::Result<()> {
            if self.offset.unwrap_or(0) > MAX_OFFSET {
                return Err(loco_rs::Error::CustomError(
                    axum::http::StatusCode::BAD_REQUEST,
                    ErrorDetail::new(
                        "offset_too_large",
                        format!("offset must not exceed {MAX_OFFSET}; narrow the query instead"),
                    ),
                ));
            }
            Ok(())
        }
    }

    /// Stamp `X-Total-Count` / `X-Limit` / `X-Offset` onto a response.
    #[must_use]
    pub fn with_page_headers(
        mut response: Response,
        total: u64,
        limit: u64,
        offset: u64,
    ) -> Response {
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
}
