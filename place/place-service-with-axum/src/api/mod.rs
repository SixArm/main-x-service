//! API plumbing: the response helpers shared by REST handlers.
//!
//! A REST success is bare JSON, built with [`ok`]. An error is an RFC 9457
//! problem, built with [`ApiProblem`], the same shape axa answers its own
//! errors with (`spec/restful` §1.3).

pub mod rest;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// A successful response: `body` as bare JSON with `status`.
pub fn ok<T: Serialize>(status: StatusCode, body: T) -> Response {
    (status, Json(body)).into_response()
}

/// An error, answered as an RFC 9457 problem by [`ApiProblem::respond`].
///
/// `code` is a stable lowercase `snake_case` token, sent as the `code`
/// extension member. `detail` is for the caller, and `errors` carries
/// structured context such as duplicate candidates.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiProblem {
    /// Stable machine-readable code, such as `not_found`.
    pub code: String,
    /// What went wrong, for the caller.
    pub detail: String,
    /// Structured context, sent as the `errors` extension member.
    pub errors: Option<serde_json::Value>,
}

impl ApiProblem {
    /// A problem with `code` and `detail`, and no `errors`.
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
            errors: None,
        }
    }

    /// Attach structured context, sent as `errors`.
    #[must_use]
    pub fn with_errors(mut self, errors: Option<serde_json::Value>) -> Self {
        self.errors = errors;
        self
    }

    /// The problem as it goes on the wire for `status`.
    ///
    /// A `500` says nothing about why: its code is the status's own and it
    /// has no `detail`, and the real code and detail are logged instead.
    /// Any other status sends `detail`, so it must be written for the caller.
    #[must_use]
    pub fn to_problem(&self, status: StatusCode) -> axa::problem::Problem {
        let problem = axa::problem::Problem::new(status);
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(code = %self.code, detail = %self.detail, "request failed");
            let code = status_code_token(status);
            return problem.with_extension("code", serde_json::Value::String(code));
        }
        let mut problem = problem
            .with_detail(self.detail.clone())
            .with_extension("code", serde_json::Value::String(self.code.clone()));
        if let Some(errors) = &self.errors {
            problem = problem.with_extension("errors", errors.clone());
        }
        problem
    }

    /// Answer with this problem and `status`.
    #[must_use]
    pub fn respond(&self, status: StatusCode) -> Response {
        self.to_problem(status).into_response()
    }
}

impl From<crate::Error> for ApiProblem {
    /// Any [`crate::Error`] as an internal error. Answered with a `500`,
    /// its text is logged and never sent.
    fn from(err: crate::Error) -> Self {
        Self::new("internal_error", err.to_string())
    }
}

/// A status's reason as a `snake_case` token, as axa writes it:
/// `500` is `internal_server_error`.
fn status_code_token(status: StatusCode) -> String {
    status
        .canonical_reason()
        .unwrap_or("error")
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn body(response: Response) -> (Option<String>, serde_json::Value) {
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_string());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (content_type, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn a_success_is_bare_json() {
        let (content_type, value) = body(ok(StatusCode::OK, serde_json::json!({"id": 1}))).await;
        assert_eq!(content_type.as_deref(), Some("application/json"));
        assert_eq!(value, serde_json::json!({"id": 1}));
    }

    #[tokio::test]
    async fn a_client_error_is_a_problem_with_its_detail() {
        let response = ApiProblem::new("duplicate_detected", "review matches")
            .with_errors(Some(serde_json::json!({"potential_matches": []})))
            .respond(StatusCode::CONFLICT);
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let (content_type, value) = body(response).await;
        assert_eq!(content_type.as_deref(), Some("application/problem+json"));
        assert_eq!(
            value,
            serde_json::json!({
                "type": "about:blank", "title": "Conflict", "status": 409,
                "detail": "review matches", "code": "duplicate_detected",
                "errors": {"potential_matches": []}
            })
        );
    }

    #[tokio::test]
    async fn a_server_error_says_nothing_about_why() {
        let response = ApiProblem::new("database_error", "connection refused to 10.0.0.5")
            .respond(StatusCode::INTERNAL_SERVER_ERROR);
        let (_, value) = body(response).await;
        assert_eq!(
            value,
            serde_json::json!({
                "type": "about:blank", "title": "Internal Server Error", "status": 500,
                "code": "internal_server_error"
            })
        );
    }
}
