//! `portfolio-service` binary entrypoint — boots the loco.rs CLI
//! (`portfolio-service`), wiring the `App` hooks and the database
//! migrator. See the crate library docs for the service overview.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use loco_rs::cli;
use migration::Migrator;
use portfolio_service::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
