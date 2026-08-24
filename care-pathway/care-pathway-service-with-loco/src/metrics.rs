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
//! | `care_pathway_flow_value_adding_ratio` | gauge vec | `pathway` |
//! | `care_pathway_flow_lead_time_p90_days` | gauge vec | `pathway` |
//! | `care_pathway_flow_coverage_ratio` | gauge vec | `pathway` |
//! | `care_pathway_flow_instances` | gauge vec | `pathway` |
//! | `care_pathway_flow_pathways_exported` | gauge | — |
//! | `care_pathway_flow_pathways_suppressed` | gauge | — |
//! | `care_pathway_flow_pathways_dropped` | gauge | — |
//! | `care_pathway_flow_last_refresh_timestamp_seconds` | gauge | — |
//!
//! # The flow gauges are default-off, bounded, and suppressed
//!
//! The `care_pathway_flow_*` family is populated by a background
//! refresh loop that only runs when `CARE_PATHWAY_FLOW_METRICS_SECS` is
//! set (see [`crate::flow_metrics`]). Unset, the gauges are never
//! touched and the family does not appear in the exposition at all —
//! the same default-off posture as every other control in this crate.
//!
//! Three properties are deliberate, because `/metrics.prom` is on the
//! **public allow-list**: it stays scrapeable with
//! `CARE_PATHWAY_REQUIRE_AUTH` on, so anything here is readable by
//! whoever can reach the port.
//!
//! - **Per-pathway series are capped** (`..._MAX_PATHWAYS`, default 50).
//!   One series per record is how a Prometheus install falls over, and a
//!   metric that kills the monitoring is worse than no metric.
//! - **Small cohorts are suppressed.** A p90 lead time over three
//!   patients *is* a patient's lead time; the API withholds it
//!   (spec §12.2) and the exporter must too, or the figure simply
//!   leaves by the side door.
//! - **Neither bound is silent.** `..._suppressed` and `..._dropped`
//!   are exported, so a reader can see the gauges are a view rather
//!   than the whole estate.
//!
//! The label is the pathway **pid**, never its name: a rename would
//! fork the series and silently reset its history.

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, Gauge, GaugeVec, IntCounterVec, Opts, Registry, TextEncoder};

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

    /// Cohort value-adding ratio per pathway (`pathway` = pid).
    pub flow_value_adding_ratio: GaugeVec,
    /// Cohort p90 lead time in days per pathway.
    pub flow_lead_time_p90_days: GaugeVec,
    /// Cohort coverage ratio per pathway.
    pub flow_coverage_ratio: GaugeVec,
    /// Instances behind each pathway's figures.
    pub flow_instances: GaugeVec,
    /// Pathways exported as their own series in the last pass.
    pub flow_pathways_exported: Gauge,
    /// Pathways withheld in the last pass for having too small a cohort.
    pub flow_pathways_suppressed: Gauge,
    /// Pathways dropped in the last pass for exceeding the series cap.
    pub flow_pathways_dropped: Gauge,
    /// Unix time of the last successful refresh. A scraper can alert on
    /// this going stale, which is the only way to tell a healthy zero
    /// from a refresh loop that died.
    pub flow_last_refresh_timestamp_seconds: Gauge,
}

/// The time-based-analysis gauge handles, grouped so [`Metrics::new`]
/// stays readable and the flow family can be registered in one place.
struct FlowGauges {
    value_adding_ratio: GaugeVec,
    lead_time_p90_days: GaugeVec,
    coverage_ratio: GaugeVec,
    instances: GaugeVec,
    pathways_exported: Gauge,
    pathways_suppressed: Gauge,
    pathways_dropped: Gauge,
    last_refresh_timestamp_seconds: Gauge,
}

impl FlowGauges {
    /// Construct the flow gauges and register them into `registry`.
    ///
    /// Registered unconditionally but never written unless the refresh
    /// loop is configured, so an unconfigured deployment renders no
    /// `care_pathway_flow_*` family at all rather than a wall of zeroes
    /// that reads like a measured estate with nothing in it.
    fn register_into(registry: &Registry) -> Self {
        let gauge_vec = |name: &str, help: &str| {
            GaugeVec::new(Opts::new(name, help), &["pathway"])
                .expect("static opts and one label name cannot fail")
        };
        let flow_value_adding_ratio = gauge_vec(
            "care_pathway_flow_value_adding_ratio",
            "Cohort value-adding time as a share of elapsed calendar time, per pathway.",
        );
        let flow_lead_time_p90_days = gauge_vec(
            "care_pathway_flow_lead_time_p90_days",
            "Cohort 90th-percentile lead time in days (nearest-rank), per pathway.",
        );
        let flow_coverage_ratio = gauge_vec(
            "care_pathway_flow_coverage_ratio",
            "Share of cohort elapsed time covered by a recorded segment, per pathway.",
        );
        let flow_instances = gauge_vec(
            "care_pathway_flow_instances",
            "Instances behind the flow figures, per pathway.",
        );
        let scalar = |name: &str, help: &str| {
            Gauge::with_opts(Opts::new(name, help)).expect("static opts cannot fail")
        };
        let flow_pathways_exported = scalar(
            "care_pathway_flow_pathways_exported",
            "Pathways exported as their own series in the last refresh.",
        );
        let flow_pathways_suppressed = scalar(
            "care_pathway_flow_pathways_suppressed",
            "Pathways withheld in the last refresh: cohort too small to aggregate \
         without describing an individual patient.",
        );
        let flow_pathways_dropped = scalar(
            "care_pathway_flow_pathways_dropped",
            "Pathways dropped in the last refresh for exceeding the series cap.",
        );
        let flow_last_refresh_timestamp_seconds = scalar(
            "care_pathway_flow_last_refresh_timestamp_seconds",
            "Unix time of the last successful flow-metrics refresh; alert on it going stale.",
        );
        for collector in [
            Box::new(flow_value_adding_ratio.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(flow_lead_time_p90_days.clone()),
            Box::new(flow_coverage_ratio.clone()),
            Box::new(flow_instances.clone()),
            Box::new(flow_pathways_exported.clone()),
            Box::new(flow_pathways_suppressed.clone()),
            Box::new(flow_pathways_dropped.clone()),
            Box::new(flow_last_refresh_timestamp_seconds.clone()),
        ] {
            registry
                .register(collector)
                .expect("registering a freshly-constructed collector cannot fail");
        }

        Self {
            value_adding_ratio: flow_value_adding_ratio,
            lead_time_p90_days: flow_lead_time_p90_days,
            coverage_ratio: flow_coverage_ratio,
            instances: flow_instances,
            pathways_exported: flow_pathways_exported,
            pathways_suppressed: flow_pathways_suppressed,
            pathways_dropped: flow_pathways_dropped,
            last_refresh_timestamp_seconds: flow_last_refresh_timestamp_seconds,
        }
    }
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

        let flow = FlowGauges::register_into(&registry);
        let FlowGauges {
            value_adding_ratio: flow_value_adding_ratio,
            lead_time_p90_days: flow_lead_time_p90_days,
            coverage_ratio: flow_coverage_ratio,
            instances: flow_instances,
            pathways_exported: flow_pathways_exported,
            pathways_suppressed: flow_pathways_suppressed,
            pathways_dropped: flow_pathways_dropped,
            last_refresh_timestamp_seconds: flow_last_refresh_timestamp_seconds,
        } = flow;

        Self {
            registry,
            care_pathway_created_total,
            care_pathway_updated_total,
            care_pathway_deleted_total,
            care_pathway_merged_total,
            http_requests_total,
            flow_value_adding_ratio,
            flow_lead_time_p90_days,
            flow_coverage_ratio,
            flow_instances,
            flow_pathways_exported,
            flow_pathways_suppressed,
            flow_pathways_dropped,
            flow_last_refresh_timestamp_seconds,
        }
    }

    /// A fresh, unregistered metric set for a test.
    ///
    /// The gauge tests reset labelled series, so sharing
    /// [`Metrics::global`] would let them clobber each other under the
    /// test harness's parallelism. Each gets its own registry instead;
    /// serialising them would have hidden the coupling rather than
    /// removed it.
    #[cfg(test)]
    fn isolated() -> Self {
        Self::new()
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

    /// Publish one flow-metrics pass (spec §15 TBA-11).
    ///
    /// Every labelled series is **reset first**. Without that, a pathway
    /// that drops out of the export — merged away, cohort shrunk below
    /// the suppression floor, pushed past the cap — would keep its last
    /// value forever, and a stale figure that looks live is worse than
    /// an absent one.
    pub fn publish_flow(&self, set: &crate::tba::FlowMetricSet, now_unix: f64) {
        self.flow_value_adding_ratio.reset();
        self.flow_lead_time_p90_days.reset();
        self.flow_coverage_ratio.reset();
        self.flow_instances.reset();

        for row in &set.rows {
            let labels = &[row.pathway_pid.as_str()];
            // A `None` figure is left unset rather than written as zero:
            // an undefined ratio is not a ratio of nothing.
            if let Some(ratio) = row.value_adding_ratio {
                self.flow_value_adding_ratio
                    .with_label_values(labels)
                    .set(ratio);
            }
            if let Some(days) = row.lead_time_p90_days {
                self.flow_lead_time_p90_days
                    .with_label_values(labels)
                    .set(days);
            }
            if let Some(coverage) = row.coverage_ratio {
                self.flow_coverage_ratio
                    .with_label_values(labels)
                    .set(coverage);
            }
            #[allow(clippy::cast_precision_loss)] // a bounded instance count
            self.flow_instances
                .with_label_values(labels)
                .set(row.instances as f64);
        }
        #[allow(clippy::cast_precision_loss)] // bounded counts
        {
            self.flow_pathways_exported.set(set.rows.len() as f64);
            self.flow_pathways_suppressed
                .set(set.suppressed_pathways as f64);
            self.flow_pathways_dropped.set(set.dropped_pathways as f64);
        }
        self.flow_last_refresh_timestamp_seconds.set(now_unix);
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

    /// A published pass renders labelled series, and the two bound
    /// counters travel with them so the gauges cannot be mistaken for
    /// the whole estate.
    #[test]
    fn publishing_flow_renders_labelled_series_and_its_bounds() {
        let metrics = Metrics::isolated();
        let set = crate::tba::FlowMetricSet {
            rows: vec![crate::tba::FlowMetricRow {
                pathway_pid: "abc".to_string(),
                instances: 12,
                value_adding_ratio: Some(0.14),
                lead_time_p90_days: Some(61.0),
                coverage_ratio: Some(0.8),
            }],
            suppressed_pathways: 3,
            dropped_pathways: 7,
        };
        metrics.publish_flow(&set, 1_700_000_000.0);
        let body = metrics.render();

        assert!(
            body.contains(r#"care_pathway_flow_value_adding_ratio{pathway="abc"} 0.14"#),
            "{body}"
        );
        assert!(
            body.contains(r#"care_pathway_flow_lead_time_p90_days{pathway="abc"} 61"#),
            "{body}"
        );
        assert!(
            body.contains(r#"care_pathway_flow_instances{pathway="abc"} 12"#),
            "{body}"
        );
        assert!(
            body.contains("care_pathway_flow_pathways_suppressed 3"),
            "{body}"
        );
        assert!(
            body.contains("care_pathway_flow_pathways_dropped 7"),
            "{body}"
        );
        assert!(
            body.contains("care_pathway_flow_pathways_exported 1"),
            "{body}"
        );
        assert!(
            body.contains("care_pathway_flow_last_refresh_timestamp_seconds 1700000000"),
            "a scraper alerts on this going stale: {body}"
        );
    }

    /// A pathway that drops out of the export loses its series rather
    /// than keeping its last value forever. A stale figure that looks
    /// live is worse than an absent one.
    #[test]
    fn a_pathway_that_leaves_the_export_loses_its_series() {
        let metrics = Metrics::isolated();
        let row = |pid: &str| crate::tba::FlowMetricRow {
            pathway_pid: pid.to_string(),
            instances: 9,
            value_adding_ratio: Some(0.5),
            lead_time_p90_days: Some(3.0),
            coverage_ratio: Some(1.0),
        };
        metrics.publish_flow(
            &crate::tba::FlowMetricSet {
                rows: vec![row("gone"), row("stays")],
                suppressed_pathways: 0,
                dropped_pathways: 0,
            },
            1.0,
        );
        assert!(metrics.render().contains(r#"pathway="gone""#));

        metrics.publish_flow(
            &crate::tba::FlowMetricSet {
                rows: vec![row("stays")],
                suppressed_pathways: 1,
                dropped_pathways: 0,
            },
            2.0,
        );
        let body = metrics.render();
        assert!(
            !body.contains(r#"pathway="gone""#),
            "a withdrawn pathway must not keep its last value: {body}"
        );
        assert!(body.contains(r#"pathway="stays""#));
    }

    /// An undefined figure is left unset rather than written as zero —
    /// a ratio that does not exist is not a ratio of nothing.
    #[test]
    fn a_null_figure_is_absent_not_zero() {
        let metrics = Metrics::isolated();
        metrics.publish_flow(
            &crate::tba::FlowMetricSet {
                rows: vec![crate::tba::FlowMetricRow {
                    pathway_pid: "nulls".to_string(),
                    instances: 6,
                    value_adding_ratio: None,
                    lead_time_p90_days: None,
                    coverage_ratio: None,
                }],
                suppressed_pathways: 0,
                dropped_pathways: 0,
            },
            3.0,
        );
        let body = metrics.render();
        assert!(
            !body.contains(r#"care_pathway_flow_value_adding_ratio{pathway="nulls"}"#),
            "an undefined ratio must not render as 0: {body}"
        );
        // The instance count is a real observation and is still there.
        assert!(
            body.contains(r#"care_pathway_flow_instances{pathway="nulls"} 6"#),
            "{body}"
        );
    }

    /// The exported content type is the Prometheus 0.0.4 text format.
    #[test]
    fn content_type_is_prometheus_text() {
        assert_eq!(CONTENT_TYPE, "text/plain; version=0.0.4");
    }
}
