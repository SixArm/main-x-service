//! Event Service server binary.
//!
//! Entry point for the Event Service REST API. Delegates to the
//! loco.rs CLI, which wires up configuration, the database, migrations
//! (via [`migration::Migrator`]), background workers, and the Axum
//! router defined by [`event_service::app::App`].
//!
//! Run with `cargo run` (development) or `cargo run --release`
//! (production). See the crate README for configuration and endpoints.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// When we build for MUSL static, use a faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use event_service::app::App;
use loco_rs::cli;
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
