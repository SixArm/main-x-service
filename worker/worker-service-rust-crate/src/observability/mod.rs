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

use opentelemetry::{KeyValue, global};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::Result;
use crate::config::ObservabilityConfig;

/// Placeholder for OpenTelemetry-exported metrics (runtime metrics live in [`crate::metrics`]).
pub mod metrics;
/// Placeholder for OpenTelemetry distributed-tracing span-exporter wiring.
pub mod traces;

/// Initializes process-wide tracing/logging from `config`.
///
/// Builds an OpenTelemetry [`Resource`] describing the service, then installs
/// a `tracing` registry with an `EnvFilter` (from `RUST_LOG`, else
/// `config.log_level`) and a JSON formatting layer. The OTLP exporter and the
/// OpenTelemetry bridge layer are left commented out pending wiring. Call
/// once at startup.
///
/// # Errors
///
/// Returns an [`Err`] of [`crate::Error`] if telemetry initialization
/// fails. The current implementation never errors, but the contract is
/// reserved for the OTLP-exporter wiring (which can fail to build the
/// pipeline).
pub fn init_telemetry(config: &ObservabilityConfig) -> Result<()> {
    // Build the OTLP `Resource`: service.name comes from config, service.version
    // from the crate version. Held in `_resource` until the exporter is wired.
    let _resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
    ]);

    // TODO: Initialize OTLP exporter — build a batch tracing pipeline on the
    // Tokio runtime exporting to `config.otlp_endpoint`, then add the
    // `tracing_opentelemetry` bridge layer below.
    // let tracer = opentelemetry_otlp::new_pipeline()
    //     .tracing()
    //     .with_exporter(...)
    //     .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Log-level filter: prefer the `RUST_LOG` env var; otherwise use the
    // configured default level (e.g. "info").
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Install the global subscriber: env filter + JSON-formatted fmt layer.
    // The commented OpenTelemetry bridge layer is added once `tracer` exists.
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

    /// Bundle of OpenTelemetry instruments for worker operations. Each field
    /// is the OpenTelemetry-API analogue of a Prometheus collector in
    /// [`crate::metrics::Metrics`]; counters are monotonic `u64`, histograms
    /// record `f64` distributions.
    pub struct MpiMetrics {
        /// Monotonic counter of worker records created.
        pub worker_created: Counter<u64>,
        /// Monotonic counter of worker records updated.
        pub worker_updated: Counter<u64>,
        /// Monotonic counter of worker records deleted.
        pub worker_deleted: Counter<u64>,
        /// Monotonic counter of worker match operations.
        pub worker_matched: Counter<u64>,
        /// Histogram of match-confidence scores (`0.0`–`1.0`).
        pub match_score: Histogram<f64>,
        /// Histogram of API request latencies (seconds).
        pub api_request_duration: Histogram<f64>,
        /// Histogram of search query latencies (seconds).
        pub search_query_duration: Histogram<f64>,
    }

    impl MpiMetrics {
        /// Not yet implemented — construct the OpenTelemetry instruments here
        /// once the meter is wired up.
        ///
        /// # Panics
        ///
        /// Always panics via `todo!` because the OpenTelemetry meter pipeline
        /// is not yet built; use [`crate::metrics::METRICS`] for runtime
        /// metrics in the meantime.
        pub fn new() -> Self {
            // TODO: Initialize metrics — obtain a meter from the global
            // provider and create each counter/histogram instrument.
            todo!("Initialize OpenTelemetry metrics")
        }
    }
}
