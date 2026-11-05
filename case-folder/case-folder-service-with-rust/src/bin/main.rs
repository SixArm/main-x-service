//! Command-line entry point. `cargo run -- start` runs the service and
//! `cargo run -- task seed` loads demo data; see [`axa::cli::Cli`].

use axa::cli::Cli;
use case_folder_service_with_rust::app;
use clap::Parser;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`app::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    app::run(Cli::parse()).await
}
