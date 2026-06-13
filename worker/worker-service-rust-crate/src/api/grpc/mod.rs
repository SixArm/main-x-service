//! gRPC API (Tonic) — stub for high-throughput callers.
//!
//! The intended transport for bulk/low-latency integrations. Currently a
//! placeholder: [`serve`] is a no-op and [`proto`] will hold the
//! Tonic/prost-generated types once the `.proto` is compiled in.

use crate::Result;
use crate::config::ServerConfig;

/// Generated Protocol Buffers code (Tonic/prost) will live here.
pub mod proto {
    // Protocol buffer generated code will go here
    // tonic::include_proto!("mpi");
}

/// Starts the gRPC server (not yet implemented — returns `Ok(())` without
/// binding). The intended implementation is sketched in the commented body.
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
