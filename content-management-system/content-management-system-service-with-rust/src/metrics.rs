//! Prometheus metrics for the CMS service.
//!
//! A process-wide [`Registry`](prometheus::Registry) with lifecycle
//! counters, rendered at `GET /metrics.prom`: the declaration surface
//! (sites, content types, and — separately — **breaking** schema
//! changes, which is the number an operator wants alerting on) and the
//! authoring surface (revisions written, and blocks the sanitizer had
//! to alter, which should stay near zero). Publishing, asset, and
//! delivery metrics arrive with their phases.

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, IntGauge, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Process-wide registry and the CMS metric handles.
pub struct Metrics {
    /// The underlying registry.
    pub registry: Registry,
    /// Sites declared.
    pub site_created_total: Counter,
    /// Content types declared.
    pub content_type_created_total: Counter,
    /// Content-type edits classified `breaking` and confirmed.
    pub content_type_breaking_change_total: Counter,
    /// Revisions written (every save, including restores).
    pub revision_created_total: Counter,
    /// Blocks the sanitizer altered on write — a number that should be
    /// small and boring; a spike means something is posting markup.
    pub blocks_sanitized_total: Counter,
    /// Assets stored (excludes deduplicated re-uploads).
    pub asset_uploaded_total: Counter,
    /// Variants published (by hand or by schedule).
    pub variant_published_total: Counter,
    /// Variants unpublished.
    pub variant_unpublished_total: Counter,
    /// Schedules the sweep actually applied.
    pub scheduled_execution_total: Counter,
    /// Live gauge: sites whose delivery is anonymously readable.
    pub sites_public: IntGauge,
}

impl Metrics {
    /// Construct the metric set into a fresh registry. The `.expect`s
    /// are infallible: static opts, empty registry.
    fn new() -> Self {
        let registry = Registry::new();
        let counter = |name: &str, help: &str| {
            Counter::with_opts(Opts::new(name, help)).expect("static counter opts are always valid")
        };
        let gauge = |name: &str, help: &str| {
            IntGauge::with_opts(Opts::new(name, help)).expect("static gauge opts are always valid")
        };
        let site_created_total = counter("site_created_total", "Total sites declared.");
        let content_type_created_total = counter(
            "content_type_created_total",
            "Total content types declared.",
        );
        let content_type_breaking_change_total = counter(
            "content_type_breaking_change_total",
            "Total confirmed breaking content-type schema changes.",
        );
        let revision_created_total = counter("revision_created_total", "Total revisions written.");
        let blocks_sanitized_total = counter(
            "blocks_sanitized_total",
            "Total blocks altered by the HTML sanitizer on write.",
        );
        let asset_uploaded_total = counter("asset_uploaded_total", "Total assets stored.");
        let variant_published_total =
            counter("variant_published_total", "Total variants published.");
        let variant_unpublished_total =
            counter("variant_unpublished_total", "Total variants unpublished.");
        let scheduled_execution_total = counter(
            "scheduled_execution_total",
            "Total scheduled publish/unpublish transitions applied by the sweep.",
        );
        let sites_public = gauge(
            "sites_public",
            "Sites whose published delivery is anonymously readable.",
        );
        for collector in [
            Box::new(site_created_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(content_type_created_total.clone()),
            Box::new(content_type_breaking_change_total.clone()),
            Box::new(revision_created_total.clone()),
            Box::new(blocks_sanitized_total.clone()),
            Box::new(asset_uploaded_total.clone()),
            Box::new(variant_published_total.clone()),
            Box::new(variant_unpublished_total.clone()),
            Box::new(scheduled_execution_total.clone()),
            Box::new(sites_public.clone()),
        ] {
            registry
                .register(collector)
                .expect("registering a freshly-constructed collector cannot fail");
        }
        Self {
            registry,
            site_created_total,
            content_type_created_total,
            content_type_breaking_change_total,
            revision_created_total,
            blocks_sanitized_total,
            asset_uploaded_total,
            variant_published_total,
            variant_unpublished_total,
            scheduled_execution_total,
            sites_public,
        }
    }

    /// The process-wide metrics, initialised on first access.
    #[must_use]
    pub fn global() -> &'static Self {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        METRICS.get_or_init(Self::new)
    }

    /// Render the registry in Prometheus text-exposition format.
    ///
    /// # Panics
    ///
    /// Not in practice: the text encoder writes into a `Vec` and
    /// produces valid UTF-8.
    #[must_use]
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let mut buf = Vec::new();
        encoder
            .encode(&self.registry.gather(), &mut buf)
            .expect("text encoder writes to a Vec which never fails");
        String::from_utf8(buf).expect("Prometheus text encoder produces UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counters and gauges register and render in valid exposition
    /// format.
    #[test]
    fn render_yields_valid_prometheus_text() {
        let m = Metrics::global();
        m.content_type_created_total.inc();
        m.sites_public.set(2);
        let body = m.render();
        assert!(body.contains("# TYPE content_type_created_total counter"));
        assert!(body.contains("# TYPE sites_public gauge"));
        assert!(body.contains("sites_public 2"));
    }
}
