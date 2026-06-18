//! Prometheus metrics endpoint.
//!
//! Serves the process-wide registry (see [`crate::metrics`]) at the
//! **root** path `GET /metrics.prom` in Prometheus text-exposition format.
//! Mounted at root (not under `/api`) alongside the OpenAPI/Swagger docs,
//! and kept public under blanket auth enforcement (see
//! `auth::is_public_path`) so a scraper needs no bearer token.

use axum::http::header::CONTENT_TYPE;
use loco_rs::prelude::*;

use crate::metrics::{self, Metrics};

/// Prometheus metrics in text-exposition format.
///
/// `GET /metrics.prom`. Returns `200` with the rendered registry and
/// `Content-Type: text/plain; version=0.0.4`. Public even under blanket
/// auth enforcement so scraping needs no bearer token.
///
/// # Errors
///
/// Never in practice; the signature is loco's. Response construction is
/// infallible (a static header value plus an owned body).
#[debug_handler]
async fn metrics_prom() -> Result<Response> {
    let body = Metrics::global().render();
    Response::builder()
        .header(CONTENT_TYPE, metrics::CONTENT_TYPE)
        .body(body.into())
        .map_err(Into::into)
}

/// Route for the Prometheus metrics endpoint, mounted at the application
/// root (`/metrics.prom`).
pub fn routes() -> Routes {
    Routes::new().add("/metrics.prom", get(metrics_prom))
}
