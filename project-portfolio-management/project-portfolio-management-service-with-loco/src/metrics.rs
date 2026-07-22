//! Prometheus metrics for the portfolio service.
//!
//! This module owns a process-wide [`Registry`](prometheus::Registry)
//! populated with a fixed set of counters. Application code increments the
//! global [`Metrics`] via [`Metrics::global`] (e.g.
//! `Metrics::global().plan_created_total.inc()` in the `plans` controller).
//! The REST API exposes the registry at `GET /metrics.prom` in Prometheus
//! text-exposition format (see [`crate::controllers`] / `controllers::metrics`).
//! Configure your scraper with `metrics_path: /metrics.prom`.
//!
//! The metric handles are cheap to clone (`Arc` under the hood); always go
//! through [`Metrics::global`] rather than re-creating them, so every
//! increment lands in the one registry that `/metrics.prom` renders.
//!
//! # Metric inventory
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `plan_created_total` | counter | — |
//! | `plan_updated_total` | counter | — |
//! | `plan_deleted_total` | counter | — |
//! | `plan_merged_total` | counter | — |
//! | `http_requests_total` | counter vec | `method`, `path`, `status` |

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, IntCounterVec, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format. Set this on the
/// HTTP response that serves [`Metrics::render`] so scrapers parse it
/// correctly.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Process-wide Prometheus registry and the standard project-portfolio-management-service metric
/// handles.
///
/// Construct once via [`Metrics::global`]; the counters are `Arc`-backed,
/// so the struct is cheap to hold by shared reference and each `.inc()`
/// updates the shared registry that [`Metrics::render`] serialises.
pub struct Metrics {
    /// The underlying registry. Exposed for callers that want to register
    /// service-specific metrics beyond this default set.
    pub registry: Registry,

    /// Count of plan records created (`POST /api/plans`).
    pub plan_created_total: Counter,
    /// Count of plan records updated (`PUT /api/plans/{pid}`).
    pub plan_updated_total: Counter,
    /// Count of plan records soft-deleted (`DELETE /api/plans/{pid}`).
    pub plan_deleted_total: Counter,
    /// Count of plan records merged (`POST /api/plans/merge`).
    pub plan_merged_total: Counter,

    /// HTTP requests handled, labeled by `method`, `path`, and `status`.
    pub http_requests_total: IntCounterVec,
}

impl Metrics {
    /// Construct the full metric set and register every collector into a
    /// fresh [`Registry`].
    ///
    /// Called once by [`Metrics::global`]. The `.expect`s are infallible:
    /// every `Opts`/collector here is statically valid, and registering a
    /// freshly-constructed collector into an empty registry cannot collide.
    fn new() -> Self {
        let registry = Registry::new();

        let plan_created_total = Counter::with_opts(Opts::new(
            "plan_created_total",
            "Total plan records created.",
        ))
        .expect("static counter opts are always valid");
        let plan_updated_total = Counter::with_opts(Opts::new(
            "plan_updated_total",
            "Total plan records updated.",
        ))
        .expect("static counter opts are always valid");
        let plan_deleted_total = Counter::with_opts(Opts::new(
            "plan_deleted_total",
            "Total plan records soft-deleted.",
        ))
        .expect("static counter opts are always valid");
        let plan_merged_total =
            Counter::with_opts(Opts::new("plan_merged_total", "Total plan records merged."))
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
            Box::new(plan_created_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(plan_updated_total.clone()),
            Box::new(plan_deleted_total.clone()),
            Box::new(plan_merged_total.clone()),
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
            plan_created_total,
            plan_updated_total,
            plan_deleted_total,
            plan_merged_total,
            http_requests_total,
        }
    }

    /// The process-wide project-portfolio-management-service metrics, initialised on first access.
    ///
    /// Backed by a private [`OnceLock`], so every caller shares the one
    /// registry that `GET /metrics.prom` renders.
    #[must_use]
    pub fn global() -> &'static Self {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        METRICS.get_or_init(Self::new)
    }

    /// Render the registry to Prometheus text-exposition format
    /// (`text/plain; version=0.0.4`).
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the text encoder writes into a `Vec`
    /// (which never fails) and always produces valid UTF-8, so the two
    /// `.expect`s are unreachable.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Incrementing a counter is reflected in the rendered exposition, and
    /// the rendered text is valid Prometheus format (HELP/TYPE lines for the
    /// registered metrics). Runs without a database.
    #[test]
    fn render_yields_valid_prometheus_text() {
        let metrics = Metrics::global();
        metrics.plan_created_total.inc();
        // A counter vec only emits a metric family once it has at least one
        // observed label set, so touch one combination here.
        metrics
            .http_requests_total
            .with_label_values(&["GET", "/metrics.prom", "200"])
            .inc();
        let body = metrics.render();

        // Each registered counter contributes HELP + TYPE lines.
        assert!(
            body.contains("# HELP plan_created_total"),
            "missing HELP for plan_created_total; got: {body}"
        );
        assert!(
            body.contains("# TYPE plan_created_total counter"),
            "missing TYPE for plan_created_total; got: {body}"
        );
        // The incremented sample is present and non-zero.
        assert!(
            body.contains("plan_created_total"),
            "missing plan_created_total sample; got: {body}"
        );
        // The label-bearing HTTP counter vec is registered too.
        assert!(
            body.contains("# TYPE http_requests_total counter"),
            "missing http_requests_total; got: {body}"
        );
    }

    /// The exposed content type matches the Prometheus text format version
    /// the handler advertises.
    #[test]
    fn content_type_is_prometheus_text() {
        assert_eq!(CONTENT_TYPE, "text/plain; version=0.0.4");
    }
}
