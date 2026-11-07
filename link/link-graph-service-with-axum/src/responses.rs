//! Response helpers: bare JSON on success, an RFC 9457 problem on failure
//! (`spec/restful` §1.3, this crate's spec §9).
//!
//! Graph responses carry an `as_of` watermark in the body itself (spec §6
//! FR-17).

use axa::prelude::*;
use axum::response::IntoResponse;
use serde::Serialize;

/// A `200` success: `data` as bare JSON.
///
/// # Errors
///
/// Propagates serialization errors from [`format::json`].
pub fn ok<T: Serialize>(data: T) -> Result<Response> {
    format::json(data)
}

/// A `400 Bad Request` problem (malformed ref, unknown kind, or depth over
/// the cap — spec §9.2), with `code` `bad_request` and `message` as its
/// `detail`.
///
/// # Errors
///
/// Infallible in practice; the `Result` matches the handler signature.
pub fn bad_request(message: &str) -> Result<Response> {
    Ok(
        axa::problem::Problem::new(axum::http::StatusCode::BAD_REQUEST)
            .with_detail(message)
            .with_extension("code", serde_json::Value::from("bad_request"))
            .into_response(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_bad_request_is_a_problem_with_its_detail() {
        let response = bad_request("depth exceeds cap of 2").unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers()[axum::http::header::CONTENT_TYPE],
            "application/problem+json"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "type": "about:blank", "title": "Bad Request", "status": 400,
                "detail": "depth exceeds cap of 2", "code": "bad_request"
            })
        );
    }
}
