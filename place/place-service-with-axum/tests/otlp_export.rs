//! End-to-end proof that spans and metrics actually leave this process
//! over OTLP/gRPC (repo `tasks.md` PRO-H12).
//!
//! Ported from person-service's `tests/otlp_export.rs` (itself from
//! link-graph-service's, the family's first working exporter). Each case
//! stands up a **real OTLP/gRPC collector in-process** ([`otlp_collector`]),
//! points the exporter at it, and asserts on the decoded protobuf that
//! arrives over the socket.
//!
//! No external collector, nothing `#[ignore]`d, no database: these run in
//! a normal `cargo test`.

mod otlp_collector;

use std::time::{Duration, Instant};

use opentelemetry::metrics::MeterProvider as _;
use opentelemetry::trace::TracerProvider as _;
use tracing_subscriber::layer::SubscriberExt;

use place_service::observability::{TelemetryConfig, build_meter_provider, build_tracer_provider};

/// **The headline assertion.** A `tracing` span emitted by ordinary
/// application code reaches a real OTLP/gRPC endpoint, carrying the
/// configured `service.name`, the span's own name, and its fields as `OTel`
/// attributes.
///
/// The subscriber is installed with `with_default` rather than `init()`, so
/// this leaves the process-global subscriber alone and can share a binary
/// with the cases below.
#[tokio::test(flavor = "multi_thread")]
async fn a_tracing_span_reaches_a_real_otlp_collector() {
    let (endpoint, captured) = otlp_collector::start().await;

    let config = TelemetryConfig {
        service_name: "place-service-otlp-test".to_string(),
        endpoint: Some(endpoint),
    };
    let provider = build_tracer_provider(&config)
        .expect("build tracer provider")
        .expect("export enabled");

    {
        let layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("place-service"));
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("places.get", place.id = "0c4f1e2a-test");
            let _entered = span.enter();
            tracing::info!("serving a place read");
        });
    }

    provider.force_flush().expect("force flush");

    let seen = captured.clone();
    assert!(
        otlp_collector::wait_for(Duration::from_secs(10), || !seen.spans().is_empty()).await,
        "no OTLP export request reached the collector"
    );

    // service.name survived the resource -> protobuf trip.
    let service_names = captured.service_names();
    assert!(
        service_names.contains(&"place-service-otlp-test".to_string()),
        "expected the configured service.name in the exported resource, got {service_names:?}"
    );

    // The span itself, by name, with its `tracing` field carried across as
    // an OTel attribute — i.e. the bridge really bridged.
    let spans = captured.spans();
    let span = spans
        .iter()
        .find(|span| span.name == "places.get")
        .unwrap_or_else(|| {
            panic!(
                "exported spans did not include `places.get`: {:?}",
                spans.iter().map(|span| &span.name).collect::<Vec<_>>()
            )
        });
    assert!(
        span.attributes.iter().any(|kv| kv.key == "place.id"),
        "the span's `tracing` field did not survive as an OTel attribute"
    );
    assert!(
        !span.trace_id.is_empty() && !span.span_id.is_empty(),
        "exported span carried no trace/span id"
    );

    provider.shutdown().ok();
}

/// The metrics half of the same pipeline: an instrument recorded through the
/// SDK meter provider is exported over OTLP/gRPC.
#[tokio::test(flavor = "multi_thread")]
async fn a_metric_reaches_a_real_otlp_collector() {
    let (endpoint, captured) = otlp_collector::start().await;
    let config = TelemetryConfig {
        service_name: "place-service-otlp-test".to_string(),
        endpoint: Some(endpoint),
    };
    let provider = build_meter_provider(&config)
        .expect("build meter provider")
        .expect("export enabled");

    let meter = provider.meter("place-service");
    let histogram = meter
        .f64_histogram("http.server.request.duration")
        .with_unit("s")
        .build();
    histogram.record(
        0.0125,
        &[opentelemetry::KeyValue::new("http.request.method", "GET")],
    );

    provider.force_flush().expect("force flush");

    let seen = captured.clone();
    assert!(
        otlp_collector::wait_for(Duration::from_secs(10), || !seen.metric_names().is_empty()).await,
        "no OTLP metric export reached the collector"
    );

    let names = captured.metric_names();
    assert!(
        names.contains(&"http.server.request.duration".to_string()),
        "expected the recorded instrument in the export, got {names:?}"
    );

    provider.shutdown().ok();
}

/// Export must never be load-bearing for availability: with **no** collector
/// listening, building the provider, emitting spans, flushing and shutting
/// down all complete promptly. This is the property that lets export default
/// to on, per the shared doc, without an activation flag.
#[tokio::test(flavor = "multi_thread")]
async fn no_collector_is_not_an_outage() {
    let config = TelemetryConfig {
        service_name: "place-service".to_string(),
        endpoint: Some(otlp_collector::dead_endpoint().await),
    };

    let started = Instant::now();
    let provider = build_tracer_provider(&config)
        .expect("building an exporter must not fail without a collector")
        .expect("export enabled");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "building the exporter blocked on connecting to the collector"
    );

    {
        let layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("test"));
        let subscriber = tracing_subscriber::registry().with(layer);
        let emitted = Instant::now();
        tracing::subscriber::with_default(subscriber, || {
            for _ in 0..100 {
                let span = tracing::info_span!("no-collector");
                let _entered = span.enter();
            }
        });
        assert!(
            emitted.elapsed() < Duration::from_secs(1),
            "emitting spans blocked while the collector was unreachable"
        );
    }

    // A failing flush/shutdown is fine and expected; hanging is not.
    let torn_down = Instant::now();
    let _ = provider.force_flush();
    let _ = provider.shutdown();
    assert!(
        torn_down.elapsed() < Duration::from_secs(60),
        "shutdown hung with an unreachable collector"
    );
}
