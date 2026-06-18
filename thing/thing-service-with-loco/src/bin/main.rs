//! Binary entry point for the `thing-service` loco.rs application.
//!
//! Boots the Loco CLI with the service's [`App`](thing_service::app::App)
//! and the migration runner, wiring up the REST API, background workers,
//! and database migrations. All application logic lives in the
//! `thing-service` library; this binary is a thin launcher.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Global allocator override for MUSL static builds, where the default
/// allocator is slow; mimalloc is markedly faster for this workload. Only
/// compiled in under `target_env = "musl"`.
// When we build for MUSL static, use a faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use loco_rs::cli;
use migration::Migrator;
use thing_service::app::App;

/// Process entry point: hand control to the loco CLI parameterized with
/// this service's [`App`] hooks and the migration runner. The Tokio runtime
/// is set up by `#[tokio::main]`.
///
/// # Errors
///
/// Returns any error surfaced by the loco CLI (bad args, boot failure,
/// migration error, server bind failure).
#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
