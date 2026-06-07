//! Observability bootstrap: tracing, logging, and OpenTelemetry.
//!
//! [`init_telemetry`](crate::observability::init_telemetry) installs a JSON
//! `tracing` subscriber driven by `RUST_LOG` (falling back to the configured
//! log level) and is called once at startup;
//! [`shutdown_telemetry`](crate::observability::shutdown_telemetry) flushes
//! the tracer provider at exit. OTLP export and the
//! [`custom_metrics`](crate::observability::custom_metrics) OpenTelemetry instruments are
//! scaffolded but not yet wired up — the runtime Prometheus metrics actually
//! served live in [`crate::metrics`].

use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::ObservabilityConfig;
use crate::Result;

pub mod metrics;
pub mod traces;

/// Initializes process-wide tracing/logging from `config`.
///
/// Builds an OpenTelemetry [`Resource`] describing the service, then installs
/// a `tracing` registry with an `EnvFilter` (from `RUST_LOG`, else
/// `config.log_level`) and a JSON formatting layer. The OTLP exporter and the
/// OpenTelemetry bridge layer are left commented out pending wiring. Call
/// once at startup.
pub fn init_telemetry(config: &ObservabilityConfig) -> Result<()> {
    // Resource attributes (service.name/version) for the future OTLP exporter.
    let _resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // TODO: Initialize OTLP exporter
    // let tracer = opentelemetry_otlp::new_pipeline()
    //     .tracing()
    //     .with_exporter(...)
    //     .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Prefer RUST_LOG; fall back to the configured default level.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        // .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    Ok(())
}

/// Flushes and shuts down the global OpenTelemetry tracer provider. Call once
/// during graceful shutdown so buffered spans are exported.
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

/// OpenTelemetry instrument definitions for the MPI system (scaffold).
///
/// These mirror the runtime Prometheus counters/histograms in
/// [`crate::metrics`] but target the OpenTelemetry metrics API; construction
/// is not yet implemented.
pub mod custom_metrics {
    use opentelemetry::metrics::{Counter, Histogram};

    /// Bundle of OpenTelemetry instruments for worker operations.
    pub struct MpiMetrics {
        /// Count of worker records created.
        pub worker_created: Counter<u64>,
        /// Count of worker records updated.
        pub worker_updated: Counter<u64>,
        /// Count of worker records deleted.
        pub worker_deleted: Counter<u64>,
        /// Count of worker match operations.
        pub worker_matched: Counter<u64>,
        /// Distribution of match-confidence scores.
        pub match_score: Histogram<f64>,
        /// Distribution of API request latencies.
        pub api_request_duration: Histogram<f64>,
        /// Distribution of search query latencies.
        pub search_query_duration: Histogram<f64>,
    }

    impl MpiMetrics {
        /// Not yet implemented — panics via `todo!`. Construct the
        /// OpenTelemetry instruments here once the meter is wired up.
        pub fn new() -> Self {
            // TODO: Initialize metrics
            todo!("Initialize OpenTelemetry metrics")
        }
    }
}
