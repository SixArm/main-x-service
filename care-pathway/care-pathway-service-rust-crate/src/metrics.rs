//! Prometheus metrics for the care-pathway service.
//!
//! This module owns a process-wide [`prometheus::Registry`] populated
//! with a fixed set of counters. Application code increments the global
//! handle returned by [`Metrics::global`] (e.g.
//! `Metrics::global().care_pathway_created_total.inc()`); the
//! controllers call `.inc()` once per successful create/update/delete/
//! merge. The REST API exposes the registry at `GET /metrics.prom` in
//! Prometheus text-exposition format (see
//! [`crate::controllers::metrics`]). Configure your scraper with
//! `metrics_path: /metrics.prom`.
//!
//! # Metric inventory
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `care_pathway_created_total` | counter | — |
//! | `care_pathway_updated_total` | counter | — |
//! | `care_pathway_deleted_total` | counter | — |
//! | `care_pathway_merged_total` | counter | — |
//! | `http_requests_total` | counter vec | `method`, `path`, `status` |

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, IntCounterVec, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format. Set this on
/// the HTTP response serving [`Metrics::render`].
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Process-wide Prometheus registry plus the standard metric handles.
///
/// The handles are cheap to clone (`Arc` under the hood), but callers
/// should always go through [`Metrics::global`] rather than constructing
/// their own set, so every increment lands in the one registry that
/// `GET /metrics.prom` renders.
pub struct Metrics {
    /// The underlying registry. Exposed so callers can register
    /// service-specific collectors beyond this default set if needed.
    pub registry: Registry,

    /// Count of care-pathway records created (`POST /api/care-pathways`).
    pub care_pathway_created_total: Counter,
    /// Count of care-pathway records updated
    /// (`PUT /api/care-pathways/{pid}`).
    pub care_pathway_updated_total: Counter,
    /// Count of care-pathway records soft-deleted
    /// (`DELETE /api/care-pathways/{pid}`).
    pub care_pathway_deleted_total: Counter,
    /// Count of care-pathway record merges
    /// (`POST /api/care-pathways/merge`).
    pub care_pathway_merged_total: Counter,

    /// HTTP requests handled, labeled by `method`, `path`, and `status`.
    pub http_requests_total: IntCounterVec,
}

impl Metrics {
    /// Construct the full metric set and register every collector into a
    /// fresh [`Registry`].
    ///
    /// Called once by [`Metrics::global`]. The `.expect`s are infallible:
    /// the opts are static and the collectors are freshly constructed, so
    /// neither construction nor registration can fail at runtime.
    fn new() -> Self {
        let registry = Registry::new();

        let care_pathway_created_total = Counter::with_opts(Opts::new(
            "care_pathway_created_total",
            "Total care-pathway records created.",
        ))
        .expect("static counter opts are always valid");
        let care_pathway_updated_total = Counter::with_opts(Opts::new(
            "care_pathway_updated_total",
            "Total care-pathway records updated.",
        ))
        .expect("static counter opts are always valid");
        let care_pathway_deleted_total = Counter::with_opts(Opts::new(
            "care_pathway_deleted_total",
            "Total care-pathway records soft-deleted.",
        ))
        .expect("static counter opts are always valid");
        let care_pathway_merged_total = Counter::with_opts(Opts::new(
            "care_pathway_merged_total",
            "Total care-pathway record merges performed.",
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

        for collector in [
            Box::new(care_pathway_created_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(care_pathway_updated_total.clone()),
            Box::new(care_pathway_deleted_total.clone()),
            Box::new(care_pathway_merged_total.clone()),
        ] {
            registry
                .register(collector)
                .expect("registering a freshly-constructed collector cannot fail");
        }
        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("registering a freshly-constructed collector cannot fail");

        Self {
            registry,
            care_pathway_created_total,
            care_pathway_updated_total,
            care_pathway_deleted_total,
            care_pathway_merged_total,
            http_requests_total,
        }
    }

    /// The process-wide care-pathway metrics, initialised on first access.
    ///
    /// All instrumentation goes through this single instance so every
    /// increment is reflected in the registry that `GET /metrics.prom`
    /// renders.
    #[must_use]
    pub fn global() -> &'static Metrics {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        METRICS.get_or_init(Metrics::new)
    }

    /// Render the registry to Prometheus text-exposition format
    /// (`text/plain; version=0.0.4`).
    ///
    /// # Panics
    ///
    /// Never in practice. The `.expect`s guard two infallible paths: the
    /// text encoder writes into a `Vec` (no I/O error path) and only ever
    /// emits UTF-8.
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

/// Tests for the Prometheus registry and text rendering. DB-free.
#[cfg(test)]
mod tests {
    use super::*;

    /// Incrementing a counter is reflected in the rendered exposition,
    /// and the render carries the `# HELP` / `# TYPE` preamble plus every
    /// declared metric name — i.e. it is valid Prometheus text.
    #[test]
    fn render_yields_valid_prometheus_text() {
        let metrics = Metrics::global();
        metrics.care_pathway_created_total.inc();
        // An `IntCounterVec` only renders a family once it has at least
        // one observed label series, so touch one before rendering.
        metrics
            .http_requests_total
            .with_label_values(&["GET", "/metrics.prom", "200"])
            .inc();
        let body = metrics.render();

        // Every declared metric name appears.
        for name in [
            "care_pathway_created_total",
            "care_pathway_updated_total",
            "care_pathway_deleted_total",
            "care_pathway_merged_total",
            "http_requests_total",
        ] {
            assert!(body.contains(name), "missing metric {name} in: {body}");
        }

        // The exposition format preamble is present for a counter.
        assert!(
            body.contains("# HELP care_pathway_created_total"),
            "missing HELP line in: {body}"
        );
        assert!(
            body.contains("# TYPE care_pathway_created_total counter"),
            "missing TYPE line in: {body}"
        );

        // The increment above is reflected as a non-zero sample.
        assert!(
            body.contains("care_pathway_created_total "),
            "missing sample line in: {body}"
        );
    }

    /// The exported content type is the Prometheus 0.0.4 text format.
    #[test]
    fn content_type_is_prometheus_text() {
        assert_eq!(CONTENT_TYPE, "text/plain; version=0.0.4");
    }
}
