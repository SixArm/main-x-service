//! API documentation endpoints: the `OpenAPI` spec + a Swagger UI page.
//!
//! Swagger UI assets are loaded from a CDN (swagger-ui-dist) to keep
//! the crate dependency-light; the spec itself is served from this
//! service.

use axa::{openapi, prelude::*};
use axum::{
    http::header,
    response::{Html, IntoResponse},
};

/// The `OpenAPI` 3 document.
#[debug_handler]
async fn openapi_json() -> Result<Response> {
    format::json(crate::openapi::spec())
}

/// A Swagger UI page wired to `/api-docs/openapi.json`.
#[debug_handler]
async fn swagger_ui() -> Result<Response> {
    let page = openapi::swagger_html("Patient Flow Service API", openapi::SPEC_PATH);
    Ok((
        [(header::CONTENT_SECURITY_POLICY, openapi::PAGE_CSP)],
        Html(page),
    )
        .into_response())
}

/// Routes for the API documentation.
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
