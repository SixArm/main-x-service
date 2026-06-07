//! Observability: tracing + OpenTelemetry initialization.
//!
//! [`init_telemetry`](crate::observability::init_telemetry) wires up the
//! `tracing` subscriber (JSON layer + `EnvFilter`) and the OTLP resource
//! attributes;
//! [`shutdown_telemetry`](crate::observability::shutdown_telemetry)
//! flushes the tracer provider on shutdown. OTLP export wiring is
//! stubbed pending exporter selection.
//! [`custom_metrics`](crate::observability::custom_metrics) sketches the
//! OpenTelemetry metric set (the live Prometheus metrics are in
//! [`crate::metrics`]).

use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::ObservabilityConfig;
use crate::Result;

/// OpenTelemetry metrics helpers.
pub mod metrics;
/// Distributed-tracing helpers.
pub mod traces;

/// Initialize tracing/logging from [`ObservabilityConfig`]: build the
/// OTLP resource, install a JSON `tracing` subscriber, and honor
/// `RUST_LOG` (falling back to the configured log level).
pub fn init_telemetry(config: &ObservabilityConfig) -> Result<()> {
    // Set up resource with service information
    let _resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // TODO: Initialize OTLP exporter
    // let tracer = opentelemetry_otlp::new_pipeline()
    //     .tracing()
    //     .with_exporter(...)
    //     .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Set up tracing subscriber
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        // .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    Ok(())
}

/// Flush and shut down the global OpenTelemetry tracer provider.
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

/// OpenTelemetry custom-metric definitions for the service.
pub mod custom_metrics {
    use opentelemetry::metrics::{Counter, Histogram};

    /// Bundle of OpenTelemetry instruments for service operations.
    pub struct MpiMetrics {
        /// Count of events created.
        pub event_created: Counter<u64>,
        /// Count of events updated.
        pub event_updated: Counter<u64>,
        /// Count of events deleted.
        pub event_deleted: Counter<u64>,
        /// Count of match operations.
        pub event_matched: Counter<u64>,
        /// Distribution of match scores.
        pub match_score: Histogram<f64>,
        /// Distribution of API request durations.
        pub api_request_duration: Histogram<f64>,
        /// Distribution of search-query durations.
        pub search_query_duration: Histogram<f64>,
    }

    impl MpiMetrics {
        /// Build the instrument set. Currently unimplemented (`todo!`).
        pub fn new() -> Self {
            // TODO: Initialize metrics
            todo!("Initialize OpenTelemetry metrics")
        }
    }
}
