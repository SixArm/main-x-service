//! Command-line entry point. See [`axa::cli::Cli`] for the commands.

use axa::cli::Cli;
use clap::Parser;
use place_service::app::App;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(Cli::parse()).await
}
