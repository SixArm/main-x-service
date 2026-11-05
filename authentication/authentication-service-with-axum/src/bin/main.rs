//! Command-line entry point for the authentication-service single sign-on provider.
//!
//! Parses the shared command line and hands it to [`App::run`].
//! See the crate library docs and `spec/index.md` for the service shape.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use authentication_service::app::App;
use axa::cli::Cli;
use clap::Parser;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(Cli::parse()).await
}
