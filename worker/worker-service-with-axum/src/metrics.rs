//! Prometheus metrics for the worker service.
//!
//! This module owns a process-wide [`Registry`](prometheus::Registry)
//! populated with a fixed set of counters and histograms. Application code
//! increments the global [`METRICS`](crate::metrics::METRICS) (e.g.
//! `METRICS.worker_created_total.inc()`). The REST API exposes the
//! registry at `GET /metrics.prom` in Prometheus text-exposition format
//! (see [`crate::api::rest::handlers::metrics_prom`]). Configure your
//! scraper with `metrics_path: /metrics.prom`.
//!
//! # Metric inventory
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `worker_created_total` | counter | — |
//! | `worker_updated_total` | counter | — |
//! | `worker_deleted_total` | counter | — |
//! | `worker_matched_total` | counter | — |
//! | `http_requests_total` | counter vec | `method`, `path`, `status` |
//! | `http_request_duration_seconds` | histogram | — |
//! | `worker_match_score` | histogram | — |
//! | `worker_search_duration_seconds` | histogram | — |

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

    /// Count of worker records created.
    pub worker_created_total: Counter,
    /// Count of worker records updated.
    pub worker_updated_total: Counter,
    /// Count of worker records soft-deleted.
    pub worker_deleted_total: Counter,
    /// Count of worker match operations performed.
    pub worker_matched_total: Counter,

    /// HTTP requests, labeled by method, path, and status code.
    pub http_requests_total: IntCounterVec,
    /// End-to-end HTTP request latency in seconds.
    pub http_request_duration_seconds: Histogram,
    /// Match-confidence scores produced by the matching engine.
    pub worker_match_score: Histogram,
    /// Search query latency in seconds.
    pub worker_search_duration_seconds: Histogram,
}

impl Metrics {
    /// Constructs the metric set and registers every collector with a fresh
    /// registry. Called once via [`METRICS`]; the `.expect(...)` calls are
    /// safe because all opts are static and registration of freshly-built
    /// collectors cannot collide.
    fn new() -> Self {
        // Fresh, isolated registry owned by this `Metrics` instance.
        let registry = Registry::new();

        // Entity-CRUD counters: one monotonic counter per worker mutation kind.
        let worker_created_total = Counter::with_opts(Opts::new(
            "worker_created_total",
            "Total worker records created.",
        ))
        .expect("static counter opts are always valid");
        let worker_updated_total = Counter::with_opts(Opts::new(
            "worker_updated_total",
            "Total worker records updated.",
        ))
        .expect("static counter opts are always valid");
        let worker_deleted_total = Counter::with_opts(Opts::new(
            "worker_deleted_total",
            "Total worker records soft-deleted.",
        ))
        .expect("static counter opts are always valid");
        let worker_matched_total = Counter::with_opts(Opts::new(
            "worker_matched_total",
            "Total worker match operations performed.",
        ))
        .expect("static counter opts are always valid");

        // HTTP request counter vector, dimensioned by the label set
        // {method, path, status} (e.g. method="GET", path="/api/workers",
        // status="200").
        let http_requests_total = IntCounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total HTTP requests handled, labeled by method, path, and status.",
            ),
            &["method", "path", "status"],
        )
        .expect("static counter-vec opts are always valid");

        // Latency histogram: explicit second-scale buckets from 1ms to 10s.
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

        // Score histogram: buckets clustered around the match thresholds
        // (note the extra 0.85/0.95 edges for the probable/certain cutoffs).
        let worker_match_score = Histogram::with_opts(
            HistogramOpts::new(
                "worker_match_score",
                "Match-confidence scores produced by the matching engine (0.0–1.0).",
            )
            .buckets(vec![
                0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.85, 0.9, 0.95, 1.0,
            ]),
        )
        .expect("static histogram opts are always valid");

        // Search-latency histogram: second-scale buckets from 1ms to 2.5s.
        let worker_search_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "worker_search_duration_seconds",
                "Search query latency in seconds.",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5,
            ]),
        )
        .expect("static histogram opts are always valid");

        // Register every collector with the registry so it appears in
        // `render()` output. The four CRUD counters share one type-erased
        // boxed loop; the remaining collectors are registered individually.
        for c in [
            Box::new(worker_created_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(worker_updated_total.clone()),
            Box::new(worker_deleted_total.clone()),
            Box::new(worker_matched_total.clone()),
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
            .register(Box::new(worker_match_score.clone()))
            .expect("registering a freshly-constructed collector cannot fail");
        registry
            .register(Box::new(worker_search_duration_seconds.clone()))
            .expect("registering a freshly-constructed collector cannot fail");

        Self {
            registry,
            worker_created_total,
            worker_updated_total,
            worker_deleted_total,
            worker_matched_total,
            http_requests_total,
            http_request_duration_seconds,
            worker_match_score,
            worker_search_duration_seconds,
        }
    }

    /// Render the registry to Prometheus text exposition format
    /// (`text/plain; version=0.0.4`).
    ///
    /// Gathers every registered metric family and encodes it with the
    /// [`TextEncoder`]. Serve the result with the [`CONTENT_TYPE`] header.
    ///
    /// # Panics
    ///
    /// Panics only on impossible conditions: encoding into an in-memory
    /// `Vec` cannot fail, and the Prometheus text encoder always emits valid
    /// UTF-8, so the inner `.expect(...)` calls never fire in practice.
    #[must_use]
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

/// The process-wide `worker` service metrics. Initialised on first access.
pub static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::new);

/// Content type for the Prometheus text-exposition format. Use this on
/// HTTP responses serving [`Metrics::render`].
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

#[cfg(test)]
mod tests {
    use super::*;

    /// Rendering the registry includes the registered counters and histograms.
    ///
    /// Pins that a counter (`worker_created_total`) and a histogram
    /// (`http_request_duration_seconds`) both appear in the text-exposition
    /// output, proving registration and rendering wire up end to end.
    #[test]
    fn render_includes_default_counters() {
        // Touch a counter so a sample exists, then render the whole registry.
        METRICS.worker_created_total.inc();
        let body = METRICS.render();
        // The CRUD counter must be present in the exposition text.
        assert!(body.contains("worker_created_total"), "got: {body}");
        // A histogram metric must also be present.
        assert!(
            body.contains("http_request_duration_seconds"),
            "got: {body}"
        );
    }
}
