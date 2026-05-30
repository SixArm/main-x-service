//! Observability setup with OpenTelemetry

use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::ObservabilityConfig;
use crate::Result;

pub mod metrics;
pub mod traces;

/// Initialize OpenTelemetry tracing and logging
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

/// Shutdown OpenTelemetry
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

/// Custom metrics for MPI system
pub mod custom_metrics {
    use opentelemetry::metrics::{Counter, Histogram};

    pub struct MpiMetrics {
        pub worker_created: Counter<u64>,
        pub worker_updated: Counter<u64>,
        pub worker_deleted: Counter<u64>,
        pub worker_matched: Counter<u64>,
        pub match_score: Histogram<f64>,
        pub api_request_duration: Histogram<f64>,
        pub search_query_duration: Histogram<f64>,
    }

    impl MpiMetrics {
        pub fn new() -> Self {
            // TODO: Initialize metrics
            todo!("Initialize OpenTelemetry metrics")
        }
    }
}
