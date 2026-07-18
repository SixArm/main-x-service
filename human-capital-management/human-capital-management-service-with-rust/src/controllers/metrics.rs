//! Prometheus metrics endpoint.
//!
//! Exposes the process-wide registry from [`crate::metrics`] at
//! `GET /metrics.prom` (mounted at the **root**, not under `/api`) in
//! Prometheus text-exposition format. The route is public even under
//! blanket auth enforcement (see [`crate::auth::is_public_path`]) so that a
//! scraper can poll it without a bearer token. Configure your scraper with
//! `metrics_path: /metrics.prom`.

use axum::http::{HeaderValue, StatusCode, header::CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use loco_rs::prelude::*;

use crate::metrics::{self, Metrics};

/// Render the process-wide registry in Prometheus text-exposition format.
///
/// Responds `200` with `Content-Type: text/plain; version=0.0.4` (see
/// [`metrics::CONTENT_TYPE`]) and the gathered metric families as the body.
#[debug_handler]
async fn metrics_prom() -> Response {
    let body = Metrics::global().render();
    (
        StatusCode::OK,
        [(
            CONTENT_TYPE,
            HeaderValue::from_static(metrics::CONTENT_TYPE),
        )],
        body,
    )
        .into_response()
}

/// The `GET /metrics.prom` route, mounted at the root (no `/api` prefix) to
/// mirror the conventional Prometheus scrape path.
pub fn routes() -> Routes {
    Routes::new().add("/metrics.prom", get(metrics_prom))
}
