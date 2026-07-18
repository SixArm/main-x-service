//! Prometheus metrics for the CRM service.
//!
//! A process-wide [`Registry`](prometheus::Registry) with lifecycle
//! counters plus live gauges (open deals, open tickets, active
//! nurture enrolments), rendered at `GET /metrics.prom`.

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, IntGauge, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Process-wide registry and the CRM metric handles.
pub struct Metrics {
    /// The underlying registry.
    pub registry: Registry,
    /// Leads captured.
    pub lead_captured_total: Counter,
    /// Leads converted.
    pub lead_converted_total: Counter,
    /// Deals won.
    pub deal_won_total: Counter,
    /// Deals lost.
    pub deal_lost_total: Counter,
    /// Nurture steps sent (simulated).
    pub nurture_step_sent_total: Counter,
    /// SLA breaches recorded by the sweep.
    pub sla_breached_total: Counter,
    /// Live gauge: open deals.
    pub deals_open: IntGauge,
    /// Live gauge: open + pending tickets.
    pub tickets_open: IntGauge,
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
        let lead_captured_total = counter("lead_captured_total", "Total leads captured.");
        let lead_converted_total = counter("lead_converted_total", "Total leads converted.");
        let deal_won_total = counter("deal_won_total", "Total deals won.");
        let deal_lost_total = counter("deal_lost_total", "Total deals lost.");
        let nurture_step_sent_total =
            counter("nurture_step_sent_total", "Total nurture steps sent.");
        let sla_breached_total = counter("sla_breached_total", "Total SLA breaches recorded.");
        let deals_open = gauge("deals_open", "Deals currently open.");
        let tickets_open = gauge("tickets_open", "Tickets currently open or pending.");
        for collector in [
            Box::new(lead_captured_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(lead_converted_total.clone()),
            Box::new(deal_won_total.clone()),
            Box::new(deal_lost_total.clone()),
            Box::new(nurture_step_sent_total.clone()),
            Box::new(sla_breached_total.clone()),
            Box::new(deals_open.clone()),
            Box::new(tickets_open.clone()),
        ] {
            registry
                .register(collector)
                .expect("registering a freshly-constructed collector cannot fail");
        }
        Self {
            registry,
            lead_captured_total,
            lead_converted_total,
            deal_won_total,
            deal_lost_total,
            nurture_step_sent_total,
            sla_breached_total,
            deals_open,
            tickets_open,
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
        m.lead_captured_total.inc();
        m.deals_open.set(7);
        let body = m.render();
        assert!(body.contains("# TYPE lead_captured_total counter"));
        assert!(body.contains("# TYPE deals_open gauge"));
        assert!(body.contains("deals_open 7"));
    }
}
