//! Observability bootstrap: tracing subscriber + OpenTelemetry wiring.
//!
//! [`init_telemetry`](crate::observability::init_telemetry) installs a JSON `tracing` subscriber filtered by
//! `RUST_LOG` (falling back to the configured `log_level`) and is the
//! single place process-wide logging is set up; call it once at startup.
//! [`shutdown_telemetry`](crate::observability::shutdown_telemetry) flushes the tracer provider on exit. The OTLP
//! exporter and the [`custom_metrics::MpiMetrics`](crate::observability::custom_metrics::MpiMetrics) instrument set are
//! stubbed (`todo!`) pending the OTLP pipeline. Submodules: [`traces`](crate::observability::traces)
//! and [`metrics`](crate::observability::metrics).

use opentelemetry::{KeyValue, global};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::Result;
use crate::config::ObservabilityConfig;

/// Metrics instrument definitions (reserved).
pub mod metrics;
/// Distributed-tracing helpers (reserved).
pub mod traces;

/// Initialize process-wide tracing/logging and (eventually) OTLP export.
///
/// Builds an OTel [`Resource`] tagging `service.name` / `service.version`,
/// then installs a JSON `tracing` subscriber whose level comes from the
/// `RUST_LOG` env filter or, failing that, `config.log_level`. The OTLP
/// exporter and the OpenTelemetry layer are currently commented out
/// (TODO). Must be called exactly once; a second call will panic because
/// the global subscriber can only be set once.
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

/// Flush and tear down the global tracer provider before exit.
///
/// Call during graceful shutdown so batched spans are not lost.
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

/// OpenTelemetry instrument definitions for the MPI system.
pub mod custom_metrics {
    use opentelemetry::metrics::{Counter, Histogram};

    /// The full set of OpenTelemetry instruments emitted by the service.
    ///
    /// Counters track lifecycle volume (creates/updates/deletes/matches);
    /// histograms track match scores and operation latencies. Construct
    /// via [`MpiMetrics::new`] (currently stubbed).
    pub struct MpiMetrics {
        /// Count of person records created.
        pub person_created: Counter<u64>,
        /// Count of person records updated.
        pub person_updated: Counter<u64>,
        /// Count of person records deleted.
        pub person_deleted: Counter<u64>,
        /// Count of match operations performed.
        pub person_matched: Counter<u64>,
        /// Distribution of computed match scores in `[0.0, 1.0]`.
        pub match_score: Histogram<f64>,
        /// Distribution of API request durations (seconds).
        pub api_request_duration: Histogram<f64>,
        /// Distribution of search query durations (seconds).
        pub search_query_duration: Histogram<f64>,
    }

    impl MpiMetrics {
        /// Build and register the instrument set (not yet implemented).
        pub fn new() -> Self {
            // TODO: Initialize metrics
            todo!("Initialize OpenTelemetry metrics")
        }
    }
}
