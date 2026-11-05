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
//! `http_requests_total` is observed by [`track_http_requests_mw`], an
//! Axum middleware layered (via `route_layer`, T-18) on both router
//! surfaces in [`crate::api::rest`] and [`crate::app`].
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

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;

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

    /// HTTP requests, labelled by `path` and `status`. Observed on every
    /// request by [`track_http_requests_mw`] (T-18); declared here so the
    /// metric appears in the exposition from the first scrape even before
    /// any request is recorded.
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

/// Axum middleware: observe every routed request on
/// [`Metrics::http_requests_total`] (T-18).
///
/// Labelled by the **matched route template** (e.g. `/api/courses/{id}`),
/// not the raw request path — using the raw path would let each course
/// `pid` mint its own label series and grow the metric unboundedly.
/// Reading [`MatchedPath`] requires this to be layered with
/// [`axum::Router::route_layer`] rather than [`axum::Router::layer`]: a
/// plain `layer` wraps the whole router *before* route matching runs, so
/// `MatchedPath` would not yet be resolved; `route_layer` applies to the
/// already-registered routes, after matching. A request that matches no
/// route (a stray path returning a raw `404`) never reaches this
/// middleware at all under `route_layer`, and so is not counted — it
/// isn't on the declared API surface the metric describes.
pub async fn track_http_requests_mw(
    matched_path: MatchedPath,
    req: Request,
    next: Next,
) -> Response {
    let path = matched_path.as_str().to_owned();
    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();
    Metrics::global()
        .http_requests_total
        .with_label_values(&[&path, &status])
        .inc();
    response
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

    /// [`track_http_requests_mw`] observes a real request on the live
    /// request path: a router carrying one dynamic-segment route, layered
    /// with `route_layer` (the only way `MatchedPath` resolves), records
    /// the **matched template** — not the concrete id in the URL — with
    /// the response status.
    #[tokio::test]
    async fn track_http_requests_mw_labels_by_matched_route_template() {
        use axum::{Router, body::Body, http::Request, routing::get};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/probe/{id}", get(|| async { "ok" }))
            .route_layer(axum::middleware::from_fn(track_http_requests_mw));

        let before = Metrics::global()
            .http_requests_total
            .with_label_values(&["/probe/{id}", "200"])
            .get();

        let req = Request::builder()
            .uri("/probe/some-concrete-uuid")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let after = Metrics::global()
            .http_requests_total
            .with_label_values(&["/probe/{id}", "200"])
            .get();
        assert!(
            after > before,
            "expected the matched-template label to increment: before={before} after={after}"
        );

        // The concrete id never became its own label series.
        let body = Metrics::global().render();
        assert!(
            !body.contains("some-concrete-uuid"),
            "the raw path segment must not appear as a label value; got: {body}"
        );
    }
}
