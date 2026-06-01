//! Prometheus metrics for the place service.
//!
//! This module owns a process-wide [`Registry`] populated with a fixed set
//! of counters and histograms. Application code increments the global
//! [`METRICS`] via `crate::metrics::METRICS` (e.g.
//! `METRICS.place_created_total.inc()`). The web tier exposes the registry
//! at `GET /metrics.prom` in Prometheus text-exposition format
//! ([`crate::web::controllers::prometheus`]).
//!
//! `/metrics` is intentionally left to the existing HTML dashboard; the
//! `.prom` suffix marks the machine-readable scrape endpoint. Configure
//! the scraper with `metrics_path: /metrics.prom`.
//!
//! # Metric inventory
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `place_created_total` | counter | — |
//! | `place_updated_total` | counter | — |
//! | `place_deleted_total` | counter | — |
//! | `place_matched_total` | counter | — |
//! | `http_requests_total` | counter vec | `method`, `path`, `status` |
//! | `http_request_duration_seconds` | histogram | — |
//! | `place_match_score` | histogram | — |
//! | `place_search_duration_seconds` | histogram | — |

use prometheus::{
    Counter, Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, Registry, TextEncoder,
};
use std::sync::LazyLock;

/// Process-wide Prometheus registry and the standard metric handles. Cloning
/// these counter handles is cheap (`Arc` under the hood); always go through
/// [`METRICS`] rather than re-creating them.
pub struct Metrics {
    /// The underlying registry. Exposed for callers that want to register
    /// service-specific metrics beyond this default set.
    pub registry: Registry,

    pub place_created_total: Counter,
    pub place_updated_total: Counter,
    pub place_deleted_total: Counter,
    pub place_matched_total: Counter,

    /// HTTP requests, labeled by method, path, and status code.
    pub http_requests_total: IntCounterVec,
    /// End-to-end HTTP request latency in seconds.
    pub http_request_duration_seconds: Histogram,
    /// Match-confidence scores produced by the matching engine.
    pub place_match_score: Histogram,
    /// Search query latency in seconds.
    pub place_search_duration_seconds: Histogram,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();

        let place_created_total = Counter::with_opts(Opts::new(
            "place_created_total",
            "Total place records created.",
        ))
        .expect("static counter opts are always valid");
        let place_updated_total = Counter::with_opts(Opts::new(
            "place_updated_total",
            "Total place records updated.",
        ))
        .expect("static counter opts are always valid");
        let place_deleted_total = Counter::with_opts(Opts::new(
            "place_deleted_total",
            "Total place records soft-deleted.",
        ))
        .expect("static counter opts are always valid");
        let place_matched_total = Counter::with_opts(Opts::new(
            "place_matched_total",
            "Total place match operations performed.",
        ))
        .expect("static counter opts are always valid");

        let http_requests_total = IntCounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total HTTP requests handled, labeled by method, path, and status.",
            ),
            &["method", "path", "status"],
        )
        .expect("static counter-vec opts are always valid");

        let http_request_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request latency in seconds.",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
        )
        .expect("static histogram opts are always valid");

        let place_match_score = Histogram::with_opts(
            HistogramOpts::new(
                "place_match_score",
                "Match-confidence scores produced by the matching engine (0.0–1.0).",
            )
            .buckets(vec![
                0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.85, 0.9, 0.95, 1.0,
            ]),
        )
        .expect("static histogram opts are always valid");

        let place_search_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "place_search_duration_seconds",
                "Search query latency in seconds.",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
            ]),
        )
        .expect("static histogram opts are always valid");

        for c in [
            Box::new(place_created_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(place_updated_total.clone()),
            Box::new(place_deleted_total.clone()),
            Box::new(place_matched_total.clone()),
        ] {
            registry
                .register(c)
                .expect("registering a freshly-constructed collector cannot fail");
        }
        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("registering a freshly-constructed collector cannot fail");
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .expect("registering a freshly-constructed collector cannot fail");
        registry
            .register(Box::new(place_match_score.clone()))
            .expect("registering a freshly-constructed collector cannot fail");
        registry
            .register(Box::new(place_search_duration_seconds.clone()))
            .expect("registering a freshly-constructed collector cannot fail");

        Self {
            registry,
            place_created_total,
            place_updated_total,
            place_deleted_total,
            place_matched_total,
            http_requests_total,
            http_request_duration_seconds,
            place_match_score,
            place_search_duration_seconds,
        }
    }

    /// Render the registry to Prometheus text exposition format
    /// (`text/plain; version=0.0.4`).
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buf = Vec::new();
        encoder
            .encode(&metric_families, &mut buf)
            .expect("text encoder writes to a Vec which never fails");
        String::from_utf8(buf).expect("Prometheus text encoder produces UTF-8")
    }
}

/// The process-wide `place` service metrics. Initialised on first access.
pub static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::new);

/// Content type for the Prometheus text-exposition format. Use this on
/// HTTP responses serving [`Metrics::render`].
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_default_counters() {
        METRICS.place_created_total.inc();
        let body = METRICS.render();
        assert!(body.contains("place_created_total"), "got: {body}");
        assert!(body.contains("http_request_duration_seconds"), "got: {body}");
    }
}
