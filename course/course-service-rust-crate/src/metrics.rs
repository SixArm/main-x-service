//! Prometheus metrics for the course service.
//!
//! This module owns a process-wide [`Registry`](prometheus::Registry)
//! populated with a fixed set of counters. Application code increments the
//! global handles via [`Metrics::global`] (e.g.
//! `crate::metrics::Metrics::global().course_created_total.inc()`).
//! The REST surface exposes the registry at the **root** path
//! `GET /metrics.prom` in Prometheus text-exposition format (see the
//! `metrics_prom` handler in [`crate::api::rest::handlers`]). Configure
//! your scraper with `metrics_path: /metrics.prom`.
//!
//! The registry is a process-wide [`OnceLock`] built on first access —
//! there is no loco state to thread through, so handlers reach the
//! metrics directly.
//!
//! # Metric inventory
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `course_created_total` | counter | — |
//! | `course_updated_total` | counter | — |
//! | `course_deleted_total` | counter | — |
//! | `course_merged_total` | counter | — |
//! | `http_requests_total` | counter vec | `path`, `status` |

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, IntCounterVec, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format (version
/// `0.0.4`). Use this on HTTP responses serving [`Metrics::render`].
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Process-wide Prometheus registry and the standard metric handles.
/// Cloning a counter handle is cheap (`Arc` under the hood); always go
/// through [`Metrics::global`] rather than re-constructing the set.
pub struct Metrics {
    /// The underlying registry. Exposed for callers that want to register
    /// service-specific metrics beyond this default set.
    pub registry: Registry,

    /// Count of course records created (`POST /api/courses`).
    pub course_created_total: Counter,
    /// Count of course records updated (`PUT /api/courses/{id}`).
    pub course_updated_total: Counter,
    /// Count of course records soft-deleted (`DELETE /api/courses/{id}`).
    pub course_deleted_total: Counter,
    /// Count of course merges (`POST /api/courses/merge`).
    pub course_merged_total: Counter,

    /// HTTP requests, labelled by `path` and `status`. Reserved for an
    /// observability middleware; declared here so the metric appears in
    /// the exposition from the first scrape even before any request is
    /// recorded.
    pub http_requests_total: IntCounterVec,
}

impl Metrics {
    /// Construct the full metric set and register every collector into a
    /// fresh [`Registry`]. Called once by [`Metrics::global`]; the
    /// `.expect`s are infallible because all opts/collectors are
    /// statically valid (distinct names, well-formed labels).
    fn new() -> Self {
        let registry = Registry::new();

        let course_created_total = Counter::with_opts(Opts::new(
            "course_created_total",
            "Total course records created.",
        ))
        .expect("static counter opts are always valid");
        let course_updated_total = Counter::with_opts(Opts::new(
            "course_updated_total",
            "Total course records updated.",
        ))
        .expect("static counter opts are always valid");
        let course_deleted_total = Counter::with_opts(Opts::new(
            "course_deleted_total",
            "Total course records soft-deleted.",
        ))
        .expect("static counter opts are always valid");
        let course_merged_total = Counter::with_opts(Opts::new(
            "course_merged_total",
            "Total course merges performed.",
        ))
        .expect("static counter opts are always valid");

        let http_requests_total = IntCounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total HTTP requests handled, labelled by path and status.",
            ),
            &["path", "status"],
        )
        .expect("static counter-vec opts are always valid");

        for c in [
            Box::new(course_created_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(course_updated_total.clone()),
            Box::new(course_deleted_total.clone()),
            Box::new(course_merged_total.clone()),
            Box::new(http_requests_total.clone()),
        ] {
            registry
                .register(c)
                .expect("registering a freshly-constructed collector cannot fail");
        }

        Self {
            registry,
            course_created_total,
            course_updated_total,
            course_deleted_total,
            course_merged_total,
            http_requests_total,
        }
    }

    /// The process-wide course-service metrics, initialised on first
    /// access: a [`OnceLock`] built once and shared read-only thereafter.
    #[must_use]
    pub fn global() -> &'static Metrics {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        METRICS.get_or_init(Metrics::new)
    }

    /// Render the registry to Prometheus text-exposition format
    /// (`text/plain; version=0.0.4`). The output is what
    /// `GET /metrics.prom` returns.
    ///
    /// # Panics
    ///
    /// Never in practice: the [`TextEncoder`] writes into a `Vec` (which
    /// cannot fail to grow) and always produces valid UTF-8.
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

/// DB-free pins for the Prometheus registry and its text rendering: the
/// declared counters appear in the exposition (with `# HELP`/`# TYPE`
/// banners) even before being incremented, and an increment is reflected
/// in the rendered output.
#[cfg(test)]
mod tests {
    use super::*;

    /// The plain counters, their `# HELP`/`# TYPE` banners, and the
    /// counter type are present in the rendered exposition — the format a
    /// `0.0.4`-compatible scraper consumes. Plain counters render even
    /// before being observed because they hold a single (label-free)
    /// series registered up front. The labelled `http_requests_total`
    /// vector emits no line until a label combination is observed, so it
    /// is asserted separately below once a sample exists.
    #[test]
    fn render_includes_declared_metrics() {
        let metrics = Metrics::global();
        let body = metrics.render();
        for name in [
            "course_created_total",
            "course_updated_total",
            "course_deleted_total",
            "course_merged_total",
        ] {
            assert!(body.contains(name), "missing metric {name}; got: {body}");
        }
        // Prometheus text-exposition banners (one HELP + one TYPE per family).
        assert!(
            body.contains("# HELP course_created_total"),
            "missing HELP banner; got: {body}"
        );
        assert!(
            body.contains("# TYPE course_created_total counter"),
            "missing TYPE banner; got: {body}"
        );

        // The labelled vector appears once a label combination is touched.
        metrics
            .http_requests_total
            .with_label_values(&["/metrics.prom", "200"])
            .inc();
        let body = metrics.render();
        assert!(
            body.contains("# TYPE http_requests_total counter"),
            "missing http_requests_total after observing a label; got: {body}"
        );

        // The content type advertises the 0.0.4 exposition version.
        assert!(CONTENT_TYPE.contains("version=0.0.4"));
    }

    /// Incrementing a counter is reflected in the rendered exposition: the
    /// sample line for the metric reports a value of at least 1.
    #[test]
    fn increment_is_reflected_in_exposition() {
        Metrics::global().course_merged_total.inc();
        let body = Metrics::global().render();
        // The counter sample line is `course_merged_total <value>`.
        let line = body
            .lines()
            .find(|l| l.starts_with("course_merged_total "))
            .expect("counter sample line present");
        let value: f64 = line
            .rsplit(' ')
            .next()
            .and_then(|v| v.parse().ok())
            .expect("counter sample has a numeric value");
        assert!(value >= 1.0, "expected >= 1 after inc(), got {value}");
    }
}
