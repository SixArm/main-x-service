//! API documentation endpoints (spec §9): the `OpenAPI` 3 spec + a
//! Swagger UI page. Swagger UI assets load from a CDN (swagger-ui-dist) to
//! keep the crate dependency-light; the spec is served from this service.
//! Both paths are public (see `auth::is_public_path`), so the blanket read
//! guard never gates the docs.

use axa::{openapi, prelude::*};
use axum::{
    http::header,
    response::{Html, IntoResponse},
};

/// `GET /api-docs/openapi.json` — the hand-written `OpenAPI` document.
#[debug_handler]
async fn openapi_json() -> Result<Response> {
    format::json(crate::openapi::spec())
}

/// `GET /swagger-ui` — a Swagger UI page wired to the spec above.
#[debug_handler]
async fn swagger_ui() -> Result<Response> {
    let page = openapi::swagger_html("Link Graph Service API", openapi::SPEC_PATH);
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
