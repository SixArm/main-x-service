//! Command-line entry point. See [`axa::cli::Cli`] for the commands:
//! `start` runs the service, `db` manages the database, and `task` runs a
//! named task.

// SEC-I3: the server binary has no reason to reach for `unsafe`; forbid it
// so the binary target matches the crate's `src/lib.rs`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

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
