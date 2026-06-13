//! Binary entry point for the `thing-service` loco.rs application.
//!
//! Boots the Loco CLI with the service's [`App`](thing_service::app::App)
//! and the migration runner, wiring up the REST API, background workers,
//! and database migrations. All application logic lives in the
//! `thing-service` library; this binary is a thin launcher.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// When we build for MUSL static, use a faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use loco_rs::cli;
use migration::Migrator;
use thing_service::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
