//! The **mounted** middleware, end to end (repo `tasks.md` PRO-H12).
//!
//! Ported from care-pathway-service's `tests/otlp_middleware.rs`.
//! `tests/otlp_export.rs` proves the export pipeline; this proves the
//! thing the service actually runs — the exact
//! [`observability::trace_mw`](case_service::observability::trace_mw)
//! layer `App::after_routes` installs (this crate's **only**
//! router-construction surface — see that module's doc comment). It
//! layers `trace_mw` onto a minimal, real Axum router — rather than
//! booting the full application, which would need a database — serves
//! it over a real socket, makes a real HTTP request, and then asserts
//! that
//!
//! 1. the response carries a well-formed W3C `traceparent`, and
//! 2. a span for that request arrived at a real OTLP/gRPC collector, and
//! 3. the `traceparent`'s trace id **is** that exported span's trace id —
//!    so the header genuinely points at the trace, rather than merely
//!    being a plausible-looking string.
//!
//! This binary installs a **process-global** subscriber (the middleware
//! emits through the global dispatcher, as it does in production, and an
//! async request handler cannot rely on a thread-local default), which is
//! why it is its own test binary rather than another case in
//! `otlp_export.rs`.

mod otlp_collector;

use std::net::SocketAddr;
use std::time::Duration;

use axum::{Router, routing::get};
use opentelemetry::trace::TracerProvider as _;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use case_service::observability::{TelemetryConfig, build_tracer_provider, trace_mw};

#[tokio::test(flavor = "multi_thread")]
async fn a_served_request_exports_a_span_and_returns_its_traceparent() {
    let (endpoint, captured) = otlp_collector::start().await;

    let config = TelemetryConfig {
        service_name: "case-service-mw-test".to_string(),
        endpoint: Some(endpoint),
    };
    let provider = build_tracer_provider(&config)
        .expect("build tracer provider")
        .expect("export enabled");
    // The filter stands in for loco's module whitelist. Without one the
    // bridge exports every internal `h2` / `hyper` span the server and the
    // exporter's own gRPC client produce — hundreds per request, which is
    // exactly what a production deployment's `EnvFilter` prevents.
    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(provider.tracer("case-service")))
        .with(tracing_subscriber::EnvFilter::new(
            "case_service=trace,otlp_middleware=trace",
        ))
        .init();

    let router = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(trace_mw));

    let listener = tokio::net::TcpListener::bind::<SocketAddr>("127.0.0.1:0".parse().unwrap())
        .await
        .expect("bind app");
    let addr = listener.local_addr().expect("app addr");
    tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });

    let response = reqwest::Client::new()
        .get(format!("http://{addr}/api/health"))
        .send()
        .await
        .expect("request the served app");
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let traceparent = response
        .headers()
        .get("traceparent")
        .expect("response carried no traceparent header")
        .to_str()
        .expect("traceparent is ASCII")
        .to_string();

    // W3C traceparent: version-traceid-spanid-flags.
    let parts: Vec<&str> = traceparent.split('-').collect();
    assert_eq!(parts.len(), 4, "malformed traceparent: {traceparent}");
    assert_eq!(parts[0], "00", "unexpected traceparent version");
    assert_eq!(
        parts[1].len(),
        32,
        "trace id is not 16 bytes: {traceparent}"
    );
    assert_eq!(parts[2].len(), 16, "span id is not 8 bytes: {traceparent}");
    assert_ne!(
        parts[1],
        "0".repeat(32),
        "traceparent carries the invalid trace id"
    );

    provider.force_flush().expect("force flush");
    let seen = captured.clone();
    assert!(
        otlp_collector::wait_for(Duration::from_secs(10), || {
            seen.spans().iter().any(|s| s.name == "http.server.request")
        })
        .await,
        "the served request produced no exported span; got {:?}",
        captured
            .spans()
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>()
    );

    let span = captured
        .spans()
        .into_iter()
        .find(|s| s.name == "http.server.request")
        .expect("request span");

    // The header points at the span that was exported — the whole point of
    // emitting it.
    let exported_trace_id: String = span.trace_id.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        parts[1], exported_trace_id,
        "the traceparent's trace id does not match the exported span's"
    );

    // And the request's shape is on the span, so the trace is useful.
    assert!(
        span.attributes
            .iter()
            .any(|kv| kv.key == "http.request.method"),
        "request span carried no http.request.method attribute"
    );

    provider.shutdown().ok();
}
