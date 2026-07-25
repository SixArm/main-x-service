//! Server binary entry point for the Case-Folder Service.
//!
//! This is a thin shim over Loco's CLI. It binds the application's
//! `App` hooks and the database `Migrator`, then hands control to
//! `loco_rs::cli::main`, which parses subcommands (`start`, `db`,
//! `task`, `routes`, …) and runs the requested one. All real wiring —
//! initializer order, route mounting, tasks — lives in
//! `case_folder_service_with_rust::app`.

// SEC-I3: the server binary has no reason to reach for `unsafe`; forbid it
// so the binary target matches the crate's `src/lib.rs`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

use case_folder_service_with_rust::app::App;
use loco_rs::cli;
use migration::Migrator;

/// Async entry point: delegate to Loco's CLI dispatcher.
///
/// Runs under the Tokio multi-threaded runtime. `cli::main` reads
/// command-line arguments and boots whichever subcommand was requested,
/// using `App` for application hooks and `Migrator` for schema
/// migrations.
///
/// # Errors
///
/// Returns the `loco_rs::Error` propagated by `cli::main` when boot,
/// migration, or the selected subcommand fails (bad config, database
/// connection failure, migration error, etc.).
#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
