//! gRPC API implementation with Tonic (stub).
//!
//! Reserved for a high-throughput Tonic/Prost gRPC surface mirroring
//! the REST API. [`serve`](crate::api::grpc::serve) is currently a
//! no-op; [`proto`](crate::api::grpc::proto) will hold
//! the Prost-generated message and service code once the `.proto`
//! schema is added.

use crate::Result;
use crate::config::ServerConfig;

/// Prost-generated protocol-buffer types and service stubs (empty
/// until the `.proto` schema is wired in via `tonic::include_proto!`).
pub mod proto {
    // Protocol buffer generated code will go here
    // tonic::include_proto!("event");
}

/// Start the gRPC server. Currently a no-op stub that returns `Ok(())`
/// immediately; the commented body sketches the intended Tonic setup.
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
