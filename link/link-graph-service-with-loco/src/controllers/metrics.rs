//! Prometheus metrics endpoint (spec §9 / T-21).
//!
//! Serves the process-wide registry ([`crate::metrics`]) at
//! `GET /metrics.prom`, mounted at the **root** (not under `/api`) and
//! public even under the blanket guard (it is in
//! [`crate::auth::is_public_path`]) so a scraper can poll it without a
//! token. The DB-derived gauges (edge counts by status; per-entity
//! consumer lag) are refreshed here at scrape time; the processed-events
//! counter is incremented in `apply_event`.

use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;
use chrono::Utc;
use loco_rs::prelude::*;

use crate::events::entity_from_topic;
use crate::metrics::{self, Metrics};
use crate::models::{consumer_offsets, edges};

/// Refresh the DB-derived gauges and render the registry in Prometheus
/// text-exposition format.
#[debug_handler]
async fn metrics_prom(State(ctx): State<AppContext>) -> Result<Response> {
    let m = Metrics::global();

    // Edge counts by integrity status.
    for (status, n) in edges::Model::count_by_status(&ctx.db).await? {
        m.edges
            .with_label_values(&[status.as_str()])
            .set(i64::try_from(n).unwrap_or(i64::MAX));
    }

    // Per-entity consumer lag (now − last consumed occurred_at).
    let now = Utc::now().fixed_offset();
    for (topic, last) in consumer_offsets::Model::freshness(&ctx.db).await? {
        m.consumer_lag_seconds
            .with_label_values(&[entity_from_topic(&topic)])
            .set((now - last).num_seconds());
    }

    Ok((
        StatusCode::OK,
        [(
            CONTENT_TYPE,
            HeaderValue::from_static(metrics::CONTENT_TYPE),
        )],
        m.render(),
    )
        .into_response())
}

/// The `GET /metrics.prom` route, mounted at the root.
pub fn routes() -> Routes {
    Routes::new().add("/metrics.prom", get(metrics_prom))
}
