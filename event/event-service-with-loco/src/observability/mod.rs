//! Observability: tracing + OpenTelemetry initialization.
//!
//! [`init_telemetry`](crate::observability::init_telemetry) wires up the
//! `tracing` subscriber (JSON layer + `EnvFilter`) and the OTLP resource
//! attributes;
//! [`shutdown_telemetry`](crate::observability::shutdown_telemetry)
//! flushes the tracer provider on shutdown. OTLP export wiring is
//! stubbed pending exporter selection. The live Prometheus metrics are
//! in [`crate::metrics`].

use opentelemetry::{KeyValue, global};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::Result;
use crate::config::ObservabilityConfig;

/// OpenTelemetry metrics helpers.
pub mod metrics;
/// Distributed-tracing helpers.
pub mod traces;

/// Initialize tracing/logging from [`ObservabilityConfig`]: build the
/// OTLP resource, install a JSON `tracing` subscriber, and honor
/// `RUST_LOG` (falling back to the configured log level).
///
/// # Errors
///
/// Returns an error if the tracing subscriber cannot be installed.
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
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

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
