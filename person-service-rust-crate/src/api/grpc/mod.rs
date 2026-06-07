//! gRPC API (Tonic) — stub.
//!
//! Reserved surface for a high-throughput gRPC interface mirroring the
//! REST/FHIR APIs. The Protobuf service is not yet defined and
//! [`serve`](crate::api::grpc::serve) is a no-op that returns immediately; the commented body
//! sketches the intended Tonic wiring. Callers should use the REST API
//! ([`crate::api::rest`]) until this is implemented.

use crate::config::ServerConfig;
use crate::Result;

/// Generated Protobuf types (reserved).
///
/// Will hold `tonic::include_proto!("mpi")` output once the `.proto`
/// service definition exists; empty for now.
pub mod proto {
    // Protocol buffer generated code will go here
    // tonic::include_proto!("mpi");
}

/// Start the gRPC server (currently a no-op stub).
///
/// Accepts the [`ServerConfig`] for forward compatibility (host/port)
/// but performs no work and returns `Ok(())` immediately. Will bind and
/// serve the Tonic service once the gRPC API is implemented.
pub async fn serve(_config: ServerConfig) -> Result<()> {
    // TODO: Implement gRPC server
    // let addr = format!("{}:{}", config.host, config.grpc_port)
    //     .parse::<std::net::SocketAddr>()
    //     .map_err(|e| crate::Error::Api(format!("Invalid gRPC address: {}", e)))?;
    //
    // tracing::info!("gRPC server listening on {}", addr);
    //
    // Server::builder()
    //     .add_service(...)
    //     .serve(addr)
    //     .await
    //     .map_err(|e| crate::Error::Api(e.to_string()))?;

    Ok(())
}
