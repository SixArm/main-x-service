//! API documentation endpoints: the `OpenAPI` spec + a Swagger UI page.
//!
//! The page comes from `axa-openapi`: one exact Swagger UI version from a
//! CDN, with integrity hashes and its own content security policy. The
//! spec itself is served from this service.

use axa::{openapi, prelude::*};
use axum::{
    http::header,
    response::{Html, IntoResponse},
};

/// The `OpenAPI` 3 document.
///
/// `GET /api-docs/openapi.json`. Returns `200` with the hand-written
/// spec from [`crate::openapi::spec`] as JSON. Stays public even under
/// blanket auth enforcement (see `auth::is_public_path`).
#[debug_handler]
async fn openapi_json() -> Result<Response> {
    format::json(crate::openapi::spec())
}

/// A Swagger UI page wired to `/api-docs/openapi.json`.
///
/// `GET /swagger-ui`. Returns `200` with a static HTML page that loads
/// Swagger UI from a CDN and points it at the spec endpoint. Also public
/// under blanket auth enforcement.
#[debug_handler]
async fn swagger_ui() -> Result<Response> {
    let page = openapi::swagger_html("Organization Service API", openapi::SPEC_PATH);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, openapi::PAGE_CSP)],
        Html(page),
    )
        .into_response())
}

/// Routes for the API documentation: the raw `OpenAPI` JSON and the
/// Swagger UI page that renders it.
pub fn routes() -> Routes {
    Routes::new()
        .add("/api-docs/openapi.json", get(openapi_json))
        .add("/swagger-ui", get(swagger_ui))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// axa's production default is `default-src 'none'`, which blanks this
    /// page unless the page sends its own policy.
    #[tokio::test]
    async fn the_page_sets_its_own_policy_and_pins_its_assets() {
        let response = swagger_ui().await.unwrap();
        assert_eq!(
            response.headers()[header::CONTENT_SECURITY_POLICY],
            openapi::PAGE_CSP
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let page = String::from_utf8(body.to_vec()).unwrap();
        assert!(page.contains(openapi::SWAGGER_UI_VERSION));
        assert!(page.contains("integrity=\"sha384-"));
        assert!(!page.contains("swagger-ui-dist@5/"));
    }
}
