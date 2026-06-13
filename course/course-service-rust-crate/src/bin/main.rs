//! Course Service binary entry-point.
//!
//! Boots the service through loco's CLI: parses subcommands (`start`,
//! `db migrate`, …) and dispatches to the [`course_service::app::App`]
//! hooks. The REST router is mounted by the app's `after_routes` hook;
//! see [`course_service`] for the library surface and `../spec.md` for
//! the canonical behaviour reference.

// Always start with high quality coding conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use course_service::app::App;
use loco_rs::cli;
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
