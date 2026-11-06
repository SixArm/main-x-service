//! Command-line entry point. See [`axa::cli::Cli`] for the commands:
//! `start` runs the service, `db` manages the database, and `task` runs a
//! named task.

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

use axa::cli;
use thing_service::app::App;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(cli::parse(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    ))
    .await
}
