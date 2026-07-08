//! Header-based API versioning for the REST surface.
//!
//! The family versions its HTTP API through a custom request header, not
//! the URL (`agents/share/api-versioning.md`): URLs are version-free
//! (`/api/projects`, with no version segment in the path), and a client
//! selects the representation version with `Accepts-version: 1.0`. This mirrors the
//! reference implementation in the event service, layered here in
//! `app.rs` alongside the blanket auth guard.
//!
//! - Pure core: [`resolve_version`] over [`SUPPORTED_API_VERSIONS`] /
//!   [`CURRENT_API_VERSION`] — trims, lowercases the compare, accepts a
//!   bare major (`1`) as an alias for its current minor (`1.0`).
//! - Edge: [`require_version_mw`] runs [`resolve_version`] on
//!   `Accepts-version` for `/api/*` requests, returns `406 Not
//!   Acceptable` on an unsupported explicit version, and stamps the
//!   resolved version onto the response as `Accepts-version`. It is
//!   orthogonal to the auth guard and a near-noop when the header is
//!   absent (defaults to the current version).

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header::HeaderName};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// The custom versioning header name (case-insensitive on the wire),
/// matching `agents/share/api-versioning.md`.
pub const VERSION_HEADER: &str = "accepts-version";

/// The current (latest) API version, served when a request omits
/// [`VERSION_HEADER`].
pub const CURRENT_API_VERSION: &str = "1.0";

/// The closed set of API versions this service accepts. Today just the
/// current one; a future `2.0` is added here, not in the URL.
pub const SUPPORTED_API_VERSIONS: &[&str] = &["1.0"];

/// Resolve the requested API version to a supported one.
///
/// `None` (no header) ⇒ [`CURRENT_API_VERSION`]. A present value is
/// trimmed and matched case-insensitively against
/// [`SUPPORTED_API_VERSIONS`]; a bare major (`"1"`) matches that major's
/// supported minor (`"1.0"`).
///
/// # Errors
///
/// Returns the (trimmed) requested version string when it is present but
/// not supported — the caller maps this to `406 Not Acceptable`.
pub fn resolve_version(requested: Option<&str>) -> Result<&'static str, String> {
    let Some(raw) = requested else {
        return Ok(CURRENT_API_VERSION);
    };
    let want = raw.trim().to_ascii_lowercase();
    if want.is_empty() {
        return Ok(CURRENT_API_VERSION);
    }
    SUPPORTED_API_VERSIONS
        .iter()
        .copied()
        .find(|&v| {
            // Exact (case-insensitive) match, or a bare-major alias:
            // `"1"` matches `"1.0"`, `"1.2"`, … .
            v.eq_ignore_ascii_case(&want)
                || v.split_once('.').is_some_and(|(major, _)| major == want)
        })
        .ok_or(want)
}

/// Whether `path` is on the native API surface this middleware versions
/// (`/api` and `/api/...`). Non-API paths (docs, metrics, health) are
/// exempt.
fn is_versioned_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}

/// Axum middleware: negotiate the API version for `/api/*` requests.
///
/// Non-API paths pass through untouched. For an API path, an unsupported
/// explicit `Accepts-version` yields `406` with a JSON body naming the
/// requested and supported versions; otherwise the request proceeds and
/// the resolved version is echoed in the response `Accepts-version`
/// header.
pub async fn require_version_mw(req: Request, next: Next) -> Response {
    if !is_versioned_path(req.uri().path()) {
        return next.run(req).await;
    }
    let requested = req
        .headers()
        .get(VERSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    match resolve_version(requested.as_deref()) {
        Ok(resolved) => {
            let mut response = next.run(req).await;
            response.headers_mut().insert(
                HeaderName::from_static(VERSION_HEADER),
                HeaderValue::from_static(resolved),
            );
            response
        }
        Err(bad) => (
            StatusCode::NOT_ACCEPTABLE,
            axum::Json(serde_json::json!({
                "error": "unsupported_api_version",
                "requested": bad,
                "supported": SUPPORTED_API_VERSIONS,
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_header_resolves_to_current() {
        assert_eq!(resolve_version(None), Ok(CURRENT_API_VERSION));
        assert_eq!(resolve_version(Some("")), Ok(CURRENT_API_VERSION));
        assert_eq!(resolve_version(Some("   ")), Ok(CURRENT_API_VERSION));
    }

    #[test]
    fn exact_and_case_insensitive_supported() {
        assert_eq!(resolve_version(Some("1.0")), Ok("1.0"));
        assert_eq!(resolve_version(Some(" 1.0 ")), Ok("1.0"));
    }

    #[test]
    fn bare_major_is_alias_for_current_minor() {
        assert_eq!(resolve_version(Some("1")), Ok("1.0"));
    }

    #[test]
    fn unsupported_version_is_error() {
        assert_eq!(resolve_version(Some("2.0")), Err("2.0".to_string()));
        assert_eq!(resolve_version(Some("9")), Err("9".to_string()));
    }

    #[test]
    fn only_api_paths_are_versioned() {
        assert!(is_versioned_path("/api"));
        assert!(is_versioned_path("/api/projects"));
        assert!(!is_versioned_path("/api-docs/openapi.json"));
        assert!(!is_versioned_path("/swagger-ui"));
        assert!(!is_versioned_path("/metrics.prom"));
    }
}
