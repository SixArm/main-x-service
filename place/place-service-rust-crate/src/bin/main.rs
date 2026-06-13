//! Binary entry point for the **Place Service** microservice.
//!
//! Wires the loco.rs CLI to the service [`App`](place_service::app::App) and
//! the SeaORM [`Migrator`](migration::Migrator). Run `cargo run -- --help`
//! for the available subcommands (start the server, run migrations, …).

// Always start with high quality coding conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use loco_rs::cli;
use migration::Migrator;
use place_service::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
