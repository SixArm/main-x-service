//! Prometheus metrics for the patient-flow service.
//!
//! A process-wide [`Registry`](prometheus::Registry) with flow
//! counters plus the live capacity **gauges** (occupancy, DTOC, open
//! requests — spec `capacity.md`), rendered at `GET /metrics.prom`
//! (`metrics_path: /metrics.prom`).

use std::sync::OnceLock;

use prometheus::{Counter, Encoder, IntGauge, Opts, Registry, TextEncoder};

/// Content type for the Prometheus text-exposition format.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

/// Process-wide registry and the patient-flow metric handles.
pub struct Metrics {
    /// The underlying registry.
    pub registry: Registry,
    /// Stays admitted (`POST /api/stays`).
    pub stay_admitted_total: Counter,
    /// Stay transfers (`POST /api/stays/{pid}/transfer`).
    pub stay_transferred_total: Counter,
    /// Stays discharged.
    pub stay_discharged_total: Counter,
    /// Bed state transitions.
    pub bed_state_changed_total: Counter,
    /// Bed requests created.
    pub bed_request_created_total: Counter,
    /// Live gauge: beds currently occupied.
    pub beds_occupied: IntGauge,
    /// Live gauge: beds currently available.
    pub beds_available: IntGauge,
    /// Live gauge: current DTOC count.
    pub dtoc_current: IntGauge,
    /// Live gauge: open bed requests.
    pub bed_requests_open: IntGauge,
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
        let stay_admitted_total = counter("stay_admitted_total", "Total stays admitted.");
        let stay_transferred_total = counter("stay_transferred_total", "Total stay transfers.");
        let stay_discharged_total = counter("stay_discharged_total", "Total stays discharged.");
        let bed_state_changed_total =
            counter("bed_state_changed_total", "Total bed state transitions.");
        let bed_request_created_total =
            counter("bed_request_created_total", "Total bed requests created.");
        let beds_occupied = gauge("beds_occupied", "Beds currently occupied.");
        let beds_available = gauge("beds_available", "Beds currently available.");
        let dtoc_current = gauge("dtoc_current", "Current delayed-transfer-of-care count.");
        let bed_requests_open = gauge("bed_requests_open", "Open bed requests.");
        for collector in [
            Box::new(stay_admitted_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(stay_transferred_total.clone()),
            Box::new(stay_discharged_total.clone()),
            Box::new(bed_state_changed_total.clone()),
            Box::new(bed_request_created_total.clone()),
            Box::new(beds_occupied.clone()),
            Box::new(beds_available.clone()),
            Box::new(dtoc_current.clone()),
            Box::new(bed_requests_open.clone()),
        ] {
            registry
                .register(collector)
                .expect("registering a freshly-constructed collector cannot fail");
        }
        Self {
            registry,
            stay_admitted_total,
            stay_transferred_total,
            stay_discharged_total,
            bed_state_changed_total,
            bed_request_created_total,
            beds_occupied,
            beds_available,
            dtoc_current,
            bed_requests_open,
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
        m.stay_admitted_total.inc();
        m.beds_available.set(42);
        let body = m.render();
        assert!(body.contains("# TYPE stay_admitted_total counter"));
        assert!(body.contains("# TYPE beds_available gauge"));
        assert!(body.contains("beds_available 42"));
    }
}
