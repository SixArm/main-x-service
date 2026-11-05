//! A real, in-process OTLP/gRPC collector for the export tests.
//!
//! Ported from care-pathway-service's `tests/otlp_collector/mod.rs`
//! (PRO-H12, itself ported from organization-service's, itself
//! course-service's, itself link-graph-service's) — nothing here is
//! service-specific. Shared by
//! `tests/otlp_export.rs` (the pipeline) and `tests/otlp_middleware.rs`
//! (the mounted middleware). It is the generated `TraceServiceServer` /
//! `MetricsServiceServer`, served by tonic on an ephemeral port, capturing
//! every decoded request — so the assertions are made against the
//! protobuf that actually crossed a socket rather than against in-process
//! SDK state.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use opentelemetry_proto::tonic::collector::metrics::v1::{
    ExportMetricsServiceRequest, ExportMetricsServiceResponse,
    metrics_service_server::{MetricsService, MetricsServiceServer},
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
    trace_service_server::{TraceService, TraceServiceServer},
};
use opentelemetry_proto::tonic::trace::v1::Span;
// Plain `tonic` — this crate carries no gRPC stub of its own, unlike
// person/worker/event, so there is no extern-prelude collision to dodge
// with a renamed `package = "tonic"` dev-dependency (see Cargo.toml).
use tokio::net::TcpListener;
use tonic::{Request, Response, Status};

/// Everything the collector has received.
#[derive(Clone, Default)]
pub struct Captured {
    /// Decoded trace export requests, in arrival order.
    pub traces: Arc<Mutex<Vec<ExportTraceServiceRequest>>>,
    /// Decoded metric export requests, in arrival order.
    pub metrics: Arc<Mutex<Vec<ExportMetricsServiceRequest>>>,
}

#[tonic::async_trait]
impl TraceService for Captured {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        self.traces.lock().unwrap().push(request.into_inner());
        Ok(Response::new(ExportTraceServiceResponse::default()))
    }
}

#[tonic::async_trait]
impl MetricsService for Captured {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        self.metrics.lock().unwrap().push(request.into_inner());
        Ok(Response::new(ExportMetricsServiceResponse::default()))
    }
}

impl Captured {
    /// Every span across every received export request.
    pub fn spans(&self) -> Vec<Span> {
        self.traces
            .lock()
            .unwrap()
            .iter()
            .flat_map(|request| request.resource_spans.clone())
            .flat_map(|rs| rs.scope_spans)
            .flat_map(|ss| ss.spans)
            .collect()
    }

    /// Every `service.name` seen on an exported resource.
    pub fn service_names(&self) -> Vec<String> {
        use opentelemetry_proto::tonic::common::v1::any_value::Value;
        self.traces
            .lock()
            .unwrap()
            .iter()
            .flat_map(|request| request.resource_spans.clone())
            .filter_map(|rs| rs.resource)
            .flat_map(|resource| resource.attributes)
            .filter(|kv| kv.key == "service.name")
            .filter_map(|kv| kv.value)
            .filter_map(|value| match value.value {
                Some(Value::StringValue(s)) => Some(s),
                _ => None,
            })
            .collect()
    }

    /// Every exported metric name.
    pub fn metric_names(&self) -> Vec<String> {
        self.metrics
            .lock()
            .unwrap()
            .iter()
            .flat_map(|request| request.resource_metrics.clone())
            .flat_map(|rm| rm.scope_metrics)
            .flat_map(|sm| sm.metrics)
            .map(|metric| metric.name)
            .collect()
    }
}

/// Start the collector on an ephemeral port; returns its `http://` endpoint
/// and the capture handle.
pub async fn start() -> (String, Captured) {
    let listener = TcpListener::bind::<SocketAddr>("127.0.0.1:0".parse().unwrap())
        .await
        .expect("bind collector");
    let addr = listener.local_addr().expect("collector addr");
    let captured = Captured::default();
    let service = captured.clone();
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(TraceServiceServer::new(service.clone()))
            .add_service(MetricsServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .ok();
    });
    (format!("http://{addr}"), captured)
}

/// An `http://` endpoint with nothing listening on it.
pub async fn dead_endpoint() -> String {
    let listener = TcpListener::bind::<SocketAddr>("127.0.0.1:0".parse().unwrap())
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    format!("http://{addr}")
}

/// Poll `predicate` until it holds or `timeout` elapses.
pub async fn wait_for(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    predicate()
}
