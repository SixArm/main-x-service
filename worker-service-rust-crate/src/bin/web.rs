//! Web server entry point.
//!
//! Boots an Axum server that serves the Tera + HTMX + Alpine + Lily UI
//! from [`worker_service::web::router`]. Loco config in `config/*.yaml`
//! documents the conventional shape; this entry uses Axum directly so
//! it works with the existing crate structure.
//!
//! Run:
//!
//! ```bash
//! cargo run --bin web
//! ```

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5150);
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    let router = worker_service::web::router()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, router).await?;
    Ok(())
}
