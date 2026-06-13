//! Person Service binary entry point.
//!
//! Boots the loco.rs application (Axum REST API, SeaORM persistence,
//! matching, search, streaming, observability) via the loco CLI. See
//! [`person_service::app::App`] for the wiring and `lib.rs` for the
//! supported library surface.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use loco_rs::cli;
use migration::Migrator;
use person_service::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
