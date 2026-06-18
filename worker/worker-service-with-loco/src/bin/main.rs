//! Worker Service binary entry point.
//!
//! Boots the loco application ([`worker_service::app::App`]) through loco's
//! CLI, wiring in the [`migration::Migrator`]. All behaviour lives in
//! the `worker-service` library crate; this binary is a thin launcher.

// Always start with high quality coding conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// When we build for MUSL static, use a faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use loco_rs::cli;
use migration::Migrator;
use worker_service::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
