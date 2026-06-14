//! Prometheus metrics endpoint.
//!
//! Serves the process-wide registry ([`crate::metrics::Metrics`]) at the
//! root path `GET /metrics.prom` in Prometheus text-exposition format.
//! The route is mounted at the application root (not under `/api`) and is
//! a public path under blanket JWT enforcement, so a scraper needs no
//! bearer token. Configure your scraper with `metrics_path: /metrics.prom`.

use axum::body::Body;
use axum::http::header::CONTENT_TYPE as CONTENT_TYPE_HEADER;
use loco_rs::prelude::*;

use crate::metrics::{Metrics, CONTENT_TYPE};

/// Render the process-wide Prometheus registry as text-exposition format.
///
/// `GET /metrics.prom` — body is [`Metrics::render`] with the
/// `text/plain; version=0.0.4` content type that Prometheus expects. The
/// response is built directly so the content type is exactly that string
/// (loco's `format::text` would force `text/plain; charset=utf-8`).
///
/// # Errors
///
/// Propagates a response-builder error (infallible in practice — the body
/// is owned text and the header is a static, valid value).
#[debug_handler]
async fn metrics_prom() -> Result<Response> {
    let body = Metrics::global().render();
    let response = Response::builder()
        .header(CONTENT_TYPE_HEADER, CONTENT_TYPE)
        .body(Body::from(body))?;
    Ok(response)
}

/// Route for the Prometheus metrics endpoint, mounted at the application
/// root (`/metrics.prom`) — mirroring how `controllers::docs` mounts its
/// root-level `/api-docs/openapi.json` and `/swagger-ui` routes.
pub fn routes() -> Routes {
    Routes::new().add("/metrics.prom", get(metrics_prom))
}
