//! This service's OTLP wiring: a request through [`App::with_telemetry`] and
//! axa's standard stack is exported to a real OTLP/gRPC collector, with its
//! metric, and answers with a `traceparent` naming the exported trace.
//!
//! `axa::opentelemetry` tests its own export pipeline; this proves the
//! service puts it where it belongs. The collector runs in this process, so
//! nothing is `#[ignore]`d and no database is needed.

mod otlp_collector;

use std::net::SocketAddr;
use std::time::Duration;

use axa::config::Http;
use axa::opentelemetry::{Config, start};
use axum::{Router, routing::get};
use event_service::app::App;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// axa's middleware settings with nothing a request here would notice.
fn http() -> Http {
    Http {
        body_limit: 1024 * 1024,
        compression: false,
        timeout_seconds: 0,
        timeout_overrides: vec![],
        cors_origins: vec![],
        hsts: false,
        hsts_max_age: 0,
        csp: None,
        permissions_policy: None,
        trusted_proxies: vec![],
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_served_request_is_exported_with_its_metric_and_traceparent() {
    let (endpoint, captured) = otlp_collector::start().await;
    let config = Config {
        service_name: "event-service-otlp-test".to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        endpoint: Some(endpoint),
        allow_insecure: false,
    };
    let telemetry = start(&config).expect("start telemetry");
    assert!(telemetry.exporting());
    // The filter stands in for axa's: without one the bridge would export
    // every `h2` and `hyper` span the exporter's own client makes.
    tracing_subscriber::registry()
        .with(telemetry.layers())
        .with(tracing_subscriber::EnvFilter::new(
            "axa=info,event_service=info",
        ))
        .init();

    let router = Router::new().route("/api/health", get(|| async { "ok" }));
    let router = App::with_telemetry(router, Some(&telemetry));
    let router = axa::middleware::apply(router, &http()).expect("standard stack");
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
        .expect("the response carried no traceparent")
        .to_str()
        .expect("traceparent is ASCII")
        .to_string();
    let parts: Vec<&str> = traceparent.split('-').collect();
    assert_eq!(parts.len(), 4, "malformed traceparent: {traceparent}");
    assert_eq!(
        parts[1].len(),
        32,
        "trace id is not 16 bytes: {traceparent}"
    );

    telemetry.flush();
    let seen = captured.clone();
    assert!(
        otlp_collector::wait_for(Duration::from_secs(10), || {
            seen.spans().iter().any(|span| span.name == "http-request")
        })
        .await,
        "the request produced no exported span; got {:?}",
        captured
            .spans()
            .iter()
            .map(|span| span.name.clone())
            .collect::<Vec<_>>()
    );
    let span = captured
        .spans()
        .into_iter()
        .find(|span| span.name == "http-request")
        .expect("request span");
    let trace_id: String = span.trace_id.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(parts[1], trace_id, "traceparent names another trace");
    assert!(
        captured
            .service_names()
            .iter()
            .any(|name| name == "event-service-otlp-test"),
        "the export carried another service.name"
    );

    let seen = captured.clone();
    assert!(
        otlp_collector::wait_for(Duration::from_secs(10), || {
            seen.metric_names()
                .iter()
                .any(|name| name == "http.server.request.duration")
        })
        .await,
        "no request metric was exported; got {:?}",
        captured.metric_names()
    );
    telemetry.shutdown();
}

/// With no endpoint, export is off and the router is left as it was.
#[test]
fn no_endpoint_means_no_export() {
    let config = Config {
        service_name: "event-service".to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        endpoint: None,
        allow_insecure: false,
    };
    let telemetry = start(&config).expect("start telemetry");
    assert!(!telemetry.exporting());
    assert!(telemetry.layers().is_empty());
}

/// Plain `http` to another host is refused unless `OTLP_ALLOW_INSECURE` is
/// set, because spans carry what the logs do.
#[test]
fn plain_http_to_another_host_is_refused() {
    let mut config = Config {
        service_name: "event-service".to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
        endpoint: Some("http://otel-collector:4317".to_string()),
        allow_insecure: false,
    };
    assert!(start(&config).is_err());
    config.allow_insecure = true;
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let telemetry = rt.block_on(async { start(&config).expect("allowed") });
    assert!(telemetry.exporting());
}
