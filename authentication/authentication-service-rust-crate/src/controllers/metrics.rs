//! Prometheus metrics endpoint.
//!
//! Exposes the process-wide registry (see [`crate::metrics`]) at
//! `GET /metrics.prom` in Prometheus text-exposition format. Mounted at
//! the **root** (not under `/api`) so a scraper can reach it with the
//! conventional `metrics_path: /metrics.prom`, alongside the other root
//! routes (`/.well-known/jwks.json`, `/api-docs/openapi.json`).
//!
//! The endpoint is unauthenticated: this service has no blanket-auth
//! middleware, and the auth flows under `/api/auth/*` are already public,
//! so no allowlist change is needed. The body carries only aggregate
//! counters — never a token, email, or pid (see [`crate::metrics`] for
//! the no-secret-labels contract).

use crate::metrics::{CONTENT_TYPE, Metrics};
use axum::http::{HeaderValue, header::CONTENT_TYPE as CONTENT_TYPE_HEADER};
use axum::response::IntoResponse;
use loco_rs::prelude::*;

/// `GET /metrics.prom` — render the Prometheus registry as
/// `text/plain; version=0.0.4`.
#[debug_handler]
async fn metrics_prom() -> impl IntoResponse {
    (
        [(CONTENT_TYPE_HEADER, HeaderValue::from_static(CONTENT_TYPE))],
        Metrics::global().render(),
    )
}

/// Route for `GET /metrics.prom`, mounted at the root (no `/api` prefix).
pub fn routes() -> Routes {
    Routes::new().add("/metrics.prom", get(metrics_prom))
}
