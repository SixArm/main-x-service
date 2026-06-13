//! API documentation endpoints: the `OpenAPI` spec + a Swagger UI page.
//!
//! Swagger UI assets are loaded from a CDN (swagger-ui-dist) to keep the
//! crate dependency-light; the spec itself is served from this service.

use loco_rs::prelude::*;

/// The `OpenAPI` 3 document.
#[debug_handler]
async fn openapi_json() -> Result<Response> {
    format::json(crate::openapi::spec())
}

/// A Swagger UI page wired to `/api-docs/openapi.json`.
#[debug_handler]
async fn swagger_ui() -> Result<Response> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Care Pathway Service API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin></script>
  <script>
    window.ui = SwaggerUIBundle({ url: '/api-docs/openapi.json', dom_id: '#swagger-ui' });
  </script>
</body>
</html>"#;
    format::html(html)
}

/// Routes for the API documentation: the raw `OpenAPI` JSON and the
/// Swagger UI page that renders it.
pub fn routes() -> Routes {
    Routes::new()
        .add("/api-docs/openapi.json", get(openapi_json))
        .add("/swagger-ui", get(swagger_ui))
}
