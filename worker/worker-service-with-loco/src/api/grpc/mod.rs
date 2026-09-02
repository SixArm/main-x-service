//! gRPC API (Tonic) — a real server (PRO-H11), not a stub.
//!
//! Follows person-service's reference implementation for this repo's
//! gRPC rollout. [`serve`] binds a [`tonic::transport::Server`] on
//! [`ServerConfig::grpc_port`](crate::config::ServerConfig::grpc_port)
//! and serves [`service::WorkerGrpcService`], generated from
//! `proto/worker.proto` by `build.rs` into [`proto`]. The RPC handlers
//! delegate to the **same** [`crate::api::rest::AppState`] — the same
//! `WorkerRepository`, the same `crate::validation::validate_worker`,
//! the same duplicate-detection core, the same offline-PASETO/ABAC
//! machinery — the REST handlers already call
//! ([`service`] module docs), so there is one set of business rules
//! behind two wire protocols, not two.
//!
//! `App::after_routes` (`src/app.rs`) spawns [`serve`] as a background
//! task alongside the REST router at boot; a bind failure is logged,
//! not fatal — the REST surface still comes up even if the gRPC port is
//! unavailable (e.g. already in use), matching this crate's existing
//! "always boot" posture for other best-effort subsystems (key
//! refresh, policy watch).
//!
//! Covers `CreateWorker` / `GetWorker` / `ListWorkers` / `DeleteWorker`
//! — deliberately not the full REST surface (no `UpdateWorker`, no
//! match/merge/search/assessments/FHIR over gRPC). See this crate's
//! spec T-6-equivalent task and `AGENTS.md`'s gRPC section for what is
//! out of scope for this slice and tracked as follow-up, rather than
//! silently missing.

use crate::Result;
use crate::api::rest::AppState;
use crate::config::ServerConfig;

pub mod service;

/// Generated Protobuf types and the `WorkerService` client/server
/// traits, compiled from `proto/worker.proto` by `build.rs`.
#[allow(missing_docs, clippy::pedantic, clippy::all)]
pub mod proto {
    tonic::include_proto!("worker");
}

/// Bind and serve the `WorkerService` gRPC server on
/// `config.grpc_port`, forever (or until the process shuts down).
///
/// # Errors
///
/// [`crate::Error::Api`] if `host:grpc_port` fails to parse as a socket
/// address, or if binding/serving fails (e.g. the port is already in
/// use).
pub async fn serve(config: ServerConfig, state: AppState) -> Result<()> {
    let addr = format!("{}:{}", config.host, config.grpc_port)
        .parse::<std::net::SocketAddr>()
        .map_err(|e| crate::Error::Api(format!("invalid gRPC bind address: {e}")))?;

    tracing::info!(%addr, "gRPC server listening");

    tonic::transport::Server::builder()
        .add_service(proto::worker_service_server::WorkerServiceServer::new(
            service::WorkerGrpcService::new(state),
        ))
        .serve(addr)
        .await
        .map_err(|e| crate::Error::Api(format!("gRPC server failed: {e}")))?;

    Ok(())
}
