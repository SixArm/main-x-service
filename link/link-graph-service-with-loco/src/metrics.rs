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
//! | `link_graph_reconciliation_divergence` | gauge | — |
//! | `link_graph_suggestion_last_run` | gauge vec | `stat` (T-33, OQ-9(d)) |

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
    /// The most recent cross-service `same_identity` suggestion pass's
    /// counts (T-33, OQ-9(d)), labeled by `stat`
    /// (`persons_fetched`/`workers_fetched`/`candidates`/`posted`/
    /// `failed`/`dropped`). A live, alertable snapshot — the durable
    /// per-pass history lives in `suggestion_runs`
    /// ([`crate::models::suggestion_runs`]), which this gauge does not
    /// replace: a gauge only ever holds the *latest* pass. Set by
    /// [`crate::suggest::job::run_periodic`].
    pub suggestion_last_run: IntGaugeVec,
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
        let suggestion_last_run = IntGaugeVec::new(
            Opts::new(
                "link_graph_suggestion_last_run",
                "Counts from the most recent cross-service same_identity suggestion pass.",
            ),
            &["stat"],
        )
        .expect("static gauge-vec opts are always valid");

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
        registry
            .register(Box::new(suggestion_last_run.clone()))
            .expect("registering a fresh collector cannot fail");

        Self {
            registry,
            events_processed_total,
            edges,
            consumer_lag_seconds,
            reconciliation_divergence,
            suggestion_last_run,
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

    /// Set the `link_graph_suggestion_last_run` gauge vec from a
    /// completed pass's stats (T-33, OQ-9(d)). Overwrites the previous
    /// pass's values — this is the *latest snapshot*, not history; see
    /// `suggestion_runs` for the durable per-pass record.
    pub fn set_suggestion_run_stats(&self, stats: &crate::suggest::job::SuggestionRunStats) {
        let g = &self.suggestion_last_run;
        g.with_label_values(&["persons_fetched"])
            .set(to_i64(stats.persons_fetched));
        g.with_label_values(&["workers_fetched"])
            .set(to_i64(stats.workers_fetched));
        g.with_label_values(&["candidates"])
            .set(to_i64(stats.candidates));
        g.with_label_values(&["posted"]).set(to_i64(stats.posted));
        g.with_label_values(&["failed"]).set(to_i64(stats.failed));
        g.with_label_values(&["dropped"]).set(to_i64(stats.dropped));
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

/// Cast a count to `i64` for a gauge, saturating rather than panicking.
/// No realistic pass count approaches `i64::MAX`; this only guards the
/// type conversion itself (mirrors `reconcile::reconcile`'s own
/// `i64::try_from(count).unwrap_or(i64::MAX)`).
fn to_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
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

    /// T-33 (OQ-9(d)): a suggestion pass's stats are exposed on the
    /// `link_graph_suggestion_last_run` gauge vec, one series per `stat`
    /// label.
    #[test]
    fn suggestion_run_stats_are_exposed_on_the_gauge_vec() {
        let stats = crate::suggest::job::SuggestionRunStats {
            persons_fetched: 10,
            workers_fetched: 8,
            candidates: 3,
            posted: 2,
            failed: 0,
            dropped: 1,
        };
        Metrics::global().set_suggestion_run_stats(&stats);
        let out = Metrics::global().render();
        assert!(out.contains("link_graph_suggestion_last_run"));
        assert!(out.contains("stat=\"candidates\""));
        assert!(out.contains("stat=\"dropped\""));
    }

    #[test]
    fn to_i64_never_panics_and_saturates_at_the_top() {
        assert_eq!(to_i64(0), 0);
        assert_eq!(to_i64(usize::MAX), i64::MAX);
    }
}
