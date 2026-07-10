//! API documentation endpoints (spec §9): the `OpenAPI` 3 spec + a
//! Swagger UI page. Swagger UI assets load from a CDN (swagger-ui-dist) to
//! keep the crate dependency-light; the spec is served from this service.
//! Both paths are public (see `auth::is_public_path`), so the blanket read
//! guard never gates the docs.

use loco_rs::prelude::*;

/// `GET /api-docs/openapi.json` — the hand-written `OpenAPI` document.
#[debug_handler]
async fn openapi_json() -> Result<Response> {
    format::json(crate::openapi::spec())
}

/// `GET /swagger-ui` — a Swagger UI page wired to the spec above.
#[debug_handler]
async fn swagger_ui() -> Result<Response> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Link Graph Service API</title>
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
