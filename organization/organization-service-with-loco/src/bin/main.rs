//! Organization Service binary entry-point.
//!
//! Boots the service through loco's CLI: parses subcommands (`start`,
//! `db migrate`, …) and dispatches to the [`organization_service::app::App`]
//! hooks. The REST router is mounted by the app's `after_routes` hook;
//! see [`organization_service`] for the library surface and
//! `../spec/index.md` for the canonical behaviour reference.

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
use organization_service::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
