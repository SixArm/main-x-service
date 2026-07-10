//! Prometheus metrics for the link-graph aggregator (spec §9 / T-21).
//!
//! A process-wide [`Registry`](prometheus::Registry) holds one **counter**
//! (events processed, labeled by kind — incremented in
//! [`crate::events::apply_event`]) and two **gauges** (edge counts by
//! status; per-topic consumer lag) that are refreshed from the database at
//! scrape time by the `GET /metrics.prom` handler. Go through
//! [`Metrics::global`] so every update lands in the one registry the
//! endpoint renders.
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `link_graph_events_processed_total` | counter vec | `kind` |
//! | `link_graph_edges` | gauge vec | `status` |
//! | `link_graph_consumer_lag_seconds` | gauge vec | `entity` |

use std::sync::OnceLock;

use prometheus::{Encoder, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Process-wide registry + the aggregator's metric handles.
pub struct Metrics {
    /// The underlying registry.
    pub registry: Registry,
    /// Events folded into the read-model, by `kind`
    /// (`created`/`deleted`/`linked`/`unlinked`/`merged`).
    pub events_processed_total: IntCounterVec,
    /// Current edge count by `status` (`unverified`/`verified`/`dangling`);
    /// refreshed at scrape time.
    pub edges: IntGaugeVec,
    /// Per-entity consumer lag in seconds (now − last consumed
    /// `occurred_at`); refreshed at scrape time.
    pub consumer_lag_seconds: IntGaugeVec,
    /// Edges that diverged between the read-model and a service's
    /// authoritative `entity_links` at the last reconciliation pass
    /// (design §8 SLO — steady-state ~0). Set by [`crate::reconcile`].
    pub reconciliation_divergence: IntGauge,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();
        let events_processed_total = IntCounterVec::new(
            Opts::new(
                "link_graph_events_processed_total",
                "Total events folded into the read-model, by kind.",
            ),
            &["kind"],
        )
        .expect("static counter-vec opts are always valid");
        let edges = IntGaugeVec::new(
            Opts::new(
                "link_graph_edges",
                "Current edge count by integrity status.",
            ),
            &["status"],
        )
        .expect("static gauge-vec opts are always valid");
        let consumer_lag_seconds = IntGaugeVec::new(
            Opts::new(
                "link_graph_consumer_lag_seconds",
                "Per-entity consumer lag in seconds (freshness watermark vs now).",
            ),
            &["entity"],
        )
        .expect("static gauge-vec opts are always valid");
        let reconciliation_divergence = IntGauge::new(
            "link_graph_reconciliation_divergence",
            "Edges diverging between the read-model and authoritative entity_links at the last reconciliation.",
        )
        .expect("static gauge opts are always valid");

        registry
            .register(Box::new(events_processed_total.clone()))
            .expect("registering a fresh collector cannot fail");
        registry
            .register(Box::new(edges.clone()))
            .expect("registering a fresh collector cannot fail");
        registry
            .register(Box::new(consumer_lag_seconds.clone()))
            .expect("registering a fresh collector cannot fail");
        registry
            .register(Box::new(reconciliation_divergence.clone()))
            .expect("registering a fresh collector cannot fail");

        Self {
            registry,
            events_processed_total,
            edges,
            consumer_lag_seconds,
            reconciliation_divergence,
        }
    }

    /// The process-wide metrics, initialised on first access.
    #[must_use]
    pub fn global() -> &'static Self {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        METRICS.get_or_init(Self::new)
    }

    /// Increment the processed-events counter for an event `kind`.
    pub fn event_processed(kind: &str) {
        Self::global()
            .events_processed_total
            .with_label_values(&[kind])
            .inc();
    }

    /// Render the registry to Prometheus text-exposition format.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the encoder writes into a `Vec` (which
    /// never fails) and the Prometheus exposition format is always valid
    /// UTF-8, so both `expect`s are unreachable.
    #[must_use]
    pub fn render(&self) -> String {
        let mut buf = Vec::new();
        TextEncoder::new()
            .encode(&self.registry.gather(), &mut buf)
            .expect("encoding into a Vec never fails");
        String::from_utf8(buf).expect("prometheus output is valid UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_the_declared_metric_families() {
        Metrics::event_processed("linked");
        let out = Metrics::global().render();
        assert!(out.contains("link_graph_events_processed_total"));
        // The counter carries the kind label we incremented.
        assert!(out.contains("kind=\"linked\""));
    }
}
