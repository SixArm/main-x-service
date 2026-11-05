//! `patient-flow-service` binary entrypoint — boots the loco.rs CLI,
//! wiring the `App` hooks and the database migrator. See the crate
//! library docs for the service overview.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axa::cli::Cli;
use clap::Parser;
use patient_flow_service::app::App;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(Cli::parse()).await
}
