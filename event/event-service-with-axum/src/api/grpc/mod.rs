//! gRPC API (Tonic) — a real server (PRO-H11), not a stub.
//!
//! Follows person-service's and worker-service's reference
//! implementations for this repo's gRPC rollout. [`serve`] binds a
//! [`tonic::transport::Server`] on
//! [`ServerConfig::grpc_port`](crate::config::ServerConfig::grpc_port)
//! and serves [`service::EventGrpcService`], generated from
//! `proto/event.proto` by `build.rs` into [`proto`]. The RPC handlers
//! delegate to the **same** [`crate::api::rest::AppState`] — the same
//! `EventRepository`, the same `crate::validation::validate_event`,
//! the same duplicate-detection core — the REST handlers already call
//! ([`service`] module docs), so there is one set of business rules
//! behind two wire protocols, not two.
//!
//! `App::after_routes` (`src/app.rs`) spawns [`serve`] as a background
//! task alongside the REST router at boot; a bind failure is logged,
//! not fatal — the REST surface still comes up even if the gRPC port is
//! unavailable (e.g. already in use), matching this crate's existing
//! "always boot" posture for other best-effort subsystems (key
//! refresh, policy watch, the outbox relay).
//!
//! Covers `CreateEvent` / `GetEvent` / `ListEvents` / `DeleteEvent` —
//! deliberately not the full REST surface (no `UpdateEvent`, no
//! match/merge/search/FHIR over gRPC; this crate has no REST list
//! endpoint at all yet, so `ListEvents` calls
//! [`crate::db::EventRepository::list_active`] directly — real domain
//! logic, just not otherwise exposed). See this crate's
//! `AGENTS.md`'s gRPC section for what is out of scope for this slice
//! and tracked as follow-up, rather than silently missing.

use crate::Result;
use crate::api::rest::AppState;
use crate::config::ServerConfig;

pub mod service;

/// Generated Protobuf types and the `EventService` client/server
/// traits, compiled from `proto/event.proto` by `build.rs`.
#[allow(missing_docs, clippy::pedantic, clippy::all)]
pub mod proto {
    tonic::include_proto!("event");
}

/// Bind and serve the `EventService` gRPC server on
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
        .add_service(proto::event_service_server::EventServiceServer::new(
            service::EventGrpcService::new(state),
        ))
        .serve(addr)
        .await
        .map_err(|e| crate::Error::Api(format!("gRPC server failed: {e}")))?;

    Ok(())
}
